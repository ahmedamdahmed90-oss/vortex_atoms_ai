use candle_core::Tensor;
use candle_transformers::generation::{LogitsProcessor, Sampling};

use crate::llm_config::SamplingConfig;
use crate::Result;

/// Index of the largest logit. Ties resolve deterministically toward the
/// lowest token id, so greedy decoding is reproducible run-to-run.
pub fn argmax_index(values: &[f32]) -> usize {
    let mut best = 0usize;
    for (index, &value) in values.iter().enumerate() {
        if value > values[best] {
            best = index;
        }
    }
    best
}

/// Apply a repeat-penalty window to logits in place.
///
/// Each occurrence of a token inside the sliding window divides (positive) or
/// multiplies (negative) its logit by `penalty`. Iterating the window once per
/// occurrence is arithmetic-identical to the previous `powf(count)` form and
/// needs no per-token hash map.
pub fn apply_repeat_penalty_window(
    values: &mut [f32],
    generated: &[u32],
    penalty: f32,
    repeat_last_n: usize,
) {
    if penalty <= 1.0 || generated.is_empty() {
        return;
    }
    let start = generated.len().saturating_sub(repeat_last_n);
    for &token in &generated[start..] {
        if let Some(logit) = values.get_mut(token as usize) {
            if *logit > 0.0 {
                *logit /= penalty;
            } else {
                *logit *= penalty;
            }
        }
    }
}

/// Zero every mass outside the top-`k` entries (candle `Sampling::TopK`
/// semantics: multinomial over the k largest only).
fn clamp_top_k(probs: &mut [f32], k: usize) {
    if k == 0 || k >= probs.len() {
        return;
    }
    let mut idx: Vec<usize> = (0..probs.len()).collect();
    idx.select_nth_unstable_by(k, |&a, &b| probs[b].total_cmp(&probs[a]));
    for (i, &j) in idx.iter().enumerate() {
        if i >= k {
            probs[j] = 0.0;
        }
    }
}

/// Zero every mass outside the nucleus (candle `Sampling::TopP`: walk
/// descending probabilities, keep the token that crosses `p`, drop the rest).
fn clamp_top_p(probs: &mut [f32], p: f32) {
    if p <= 0.0 || p >= 1.0 {
        return;
    }
    let mut idx: Vec<usize> = (0..probs.len()).collect();
    idx.sort_by(|&a, &b| probs[b].total_cmp(&probs[a]));
    let mut cumsum = 0.0f32;
    for &i in &idx {
        if cumsum >= p {
            probs[i] = 0.0;
        } else {
            cumsum += probs[i];
        }
    }
}

/// Renormalize in place so the distribution sums to 1 (no-op if already
/// ~1 or if everything collapsed — caller handles the degenerate case).
fn renorm_in_place(probs: &mut [f32]) {
    let sum: f32 = probs.iter().sum();
    if sum.is_finite() && sum > 0.0 {
        for v in probs.iter_mut() {
            *v /= sum;
        }
    }
}

/// Wrapper around candle-transformers LogitsProcessor with VortexAtoms integration.
pub struct VortexSampler {
    inner: LogitsProcessor,
    repeat_penalty: f32,
    repeat_last_n: usize,
    config: SamplingConfig,
    seed: u64,
    /// True when the active sampling mode is `ArgMax`, enabling a fast
    /// allocation-light greedy path that bypasses candle's softmax/top-k/top-p
    /// machinery entirely.
    greedy: bool,
}

impl VortexSampler {
    pub fn new(seed: u64, config: &SamplingConfig) -> Self {
        let sampling = Self::make_sampling(config);
        let greedy = matches!(sampling, Sampling::ArgMax);
        Self {
            inner: LogitsProcessor::from_sampling(seed, sampling),
            repeat_penalty: config.repeat_penalty,
            repeat_last_n: config.repeat_last_n,
            config: config.clone(),
            seed,
            greedy,
        }
    }

    /// True when decoding will use deterministic `ArgMax` (the greedy fast path).
    pub fn is_greedy(&self) -> bool {
        self.greedy
    }

    /// Copy `logits` into `out` with the repeat-penalty window applied
    /// (same transform `sample` uses before argmax / softmax).
    pub fn penalized_logits_into(&self, logits: &[f32], generated: &[u32], out: &mut Vec<f32>) {
        out.clear();
        out.extend_from_slice(logits);
        apply_repeat_penalty_window(out, generated, self.repeat_penalty, self.repeat_last_n);
    }

    fn make_sampling(config: &SamplingConfig) -> Sampling {
        let temp = config.temperature.unwrap_or(0.8);
        // A temperature at (or below) 1e-7 means greedy: map it to candle's
        // ArgMax instead of a near-zero-temperature softmax (which divides by
        // ~0 and produces NaN logits — a latent bug in the old mapping).
        if temp <= 1e-7 {
            return Sampling::ArgMax;
        }
        match (config.top_k, config.top_p) {
            (Some(k), Some(p)) => Sampling::TopKThenTopP {
                k,
                p,
                temperature: temp,
            },
            (Some(k), None) => Sampling::TopK {
                k,
                temperature: temp,
            },
            (None, Some(p)) => Sampling::TopP {
                p,
                temperature: temp,
            },
            (None, None) => {
                if config.temperature.is_some() {
                    Sampling::All { temperature: temp }
                } else {
                    Sampling::ArgMax
                }
            }
        }
    }

    /// Fill `out` with the full target distribution `p` that `sample` /
    /// `sample_with_scratch` would draw from (repeat penalty, temperature,
    /// top-k, top-p — then renormalized to sum to 1).
    ///
    /// Used by probabilistic speculative acceptance (Levi et al.): the
    /// verifier needs `p` at every draft position, not just one draw.
    /// Greedy mode returns a one-hot at the penalized argmax so the Levi
    /// rule reduces to bit-identical argmax accept/reject.
    pub fn target_probs_into(
        &self,
        logits: &Tensor,
        generated: &[u32],
        out: &mut Vec<f32>,
    ) -> Result<()> {
        let tmp = logits.to_vec1::<f32>()?;
        self.target_probs_logits(&tmp, generated, out)
    }

    /// Slice form of [`Self::target_probs_into`] (no Tensor round-trip).
    pub fn target_probs_logits(
        &self,
        logits: &[f32],
        generated: &[u32],
        out: &mut Vec<f32>,
    ) -> Result<()> {
        out.clear();
        out.extend_from_slice(logits);
        apply_repeat_penalty_window(out, generated, self.repeat_penalty, self.repeat_last_n);

        if self.greedy {
            let best = argmax_index(out);
            out.iter_mut().for_each(|v| *v = 0.0);
            out[best] = 1.0;
            return Ok(());
        }

        let temp = self.config.temperature.unwrap_or(0.8).max(1e-7) as f32;
        let pre_max = out.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        if !pre_max.is_finite() {
            out.iter_mut().for_each(|v| *v = 0.0);
            if let Some(first) = out.first_mut() {
                *first = 1.0;
            }
            return Ok(());
        }
        let mut sum = 0.0f32;
        for v in out.iter_mut() {
            *v = ((*v - pre_max) / temp).exp();
            sum += *v;
        }
        if !(sum.is_finite() && sum > 0.0) {
            out.iter_mut().for_each(|v| *v = 0.0);
            out[0] = 1.0;
            return Ok(());
        }
        for v in out.iter_mut() {
            *v /= sum;
        }

        match (self.config.top_k, self.config.top_p) {
            (Some(k), Some(p)) => {
                clamp_top_k(out, k);
                clamp_top_p(out, p as f32);
                renorm_in_place(out);
            }
            (Some(k), None) => {
                clamp_top_k(out, k);
                renorm_in_place(out);
            }
            (None, Some(p)) => {
                clamp_top_p(out, p as f32);
                renorm_in_place(out);
            }
            (None, None) => {}
        }
        Ok(())
    }

    /// Sample with caller-provided scratch (1.2: reuse logits Vec, no per-token alloc).
    pub fn sample_with_scratch(
        &mut self,
        logits: &Tensor,
        generated_tokens: &[u32],
        scratch: &mut Vec<f32>,
    ) -> Result<u32> {
        if self.greedy {
            // Reuse scratch: clear without deallocating, fill via to_vec1 into temp
            // then move into scratch with capacity reuse (vocab ~32k, one alloc total).
            let tmp = logits.to_vec1::<f32>()?;
            scratch.clear();
            scratch.extend_from_slice(&tmp);
            apply_repeat_penalty_window(
                scratch,
                generated_tokens,
                self.repeat_penalty,
                self.repeat_last_n,
            );
            return Ok(argmax_index(scratch) as u32);
        }
        // Non-greedy reuses scratch too, then falls back to penalized tensor path.
        let tmp = logits.to_vec1::<f32>()?;
        scratch.clear();
        scratch.extend_from_slice(&tmp);
        // Apply windowed penalty in place on scratch, then move the buffer
        // into the Tensor via mem::take (no vocab-size memcpy per token —
        // the old `scratch.clone()` copied ~32k f32 on every sample).
        if self.repeat_penalty > 1.0 && !generated_tokens.is_empty() {
            let start = generated_tokens.len().saturating_sub(self.repeat_last_n);
            for &tok in &generated_tokens[start..] {
                if let Some(v) = scratch.get_mut(tok as usize) {
                    if *v > 0.0 {
                        *v /= self.repeat_penalty;
                    } else {
                        *v *= self.repeat_penalty;
                    }
                }
            }
        }
        let capacity = scratch.capacity();
        let taken = std::mem::take(scratch);
        let penalized = match Tensor::from_vec(taken, logits.shape(), logits.device()) {
            Ok(t) => t,
            Err(e) => {
                // from_vec consumed `taken`; restore capacity before propagating.
                scratch.reserve(capacity);
                return Err(e.into());
            }
        };
        let sample_result = self.inner.sample(&penalized);
        drop(penalized);
        // Buffer freed with the Tensor. Restore capacity so the next token's
        // clear+extend reuses the allocation (peak stays ~2 buffers like the
        // old clone path, but without the per-token vocab memcpy).
        scratch.reserve(capacity);
        Ok(sample_result?)
    }

    /// Sample the next token from logits, applying repeat penalty.
    pub fn sample(&mut self, logits: &Tensor, generated_tokens: &[u32]) -> Result<u32> {
        if self.greedy {
            // Greedy fast path: one F32 vector read, penalty applied in place,
            // manual argmax. Skips candle's softmax/top-k/top-p/WeightedIndex
            // machinery and the extra tensor round-trip the old path performed.
            let mut logits_vec = logits.to_vec1::<f32>()?;
            apply_repeat_penalty_window(
                &mut logits_vec,
                generated_tokens,
                self.repeat_penalty,
                self.repeat_last_n,
            );
            return Ok(argmax_index(&logits_vec) as u32);
        }

        if self.repeat_penalty > 1.0 && !generated_tokens.is_empty() {
            let penalty_tokens = if generated_tokens.len() > self.repeat_last_n {
                &generated_tokens[generated_tokens.len() - self.repeat_last_n..]
            } else {
                generated_tokens
            };

            let mut logits_vec = logits.to_vec1::<f32>()?;
            let mut token_counts = std::collections::HashMap::new();
            for &token in penalty_tokens {
                *token_counts.entry(token).or_insert(0u32) += 1;
            }

            for (&token, &count) in &token_counts {
                if let Some(logit) = logits_vec.get_mut(token as usize) {
                    if *logit > 0.0 {
                        *logit /= self.repeat_penalty.powf(count as f32);
                    } else {
                        *logit *= self.repeat_penalty.powf(count as f32);
                    }
                }
            }

            let penalized = Tensor::from_vec(logits_vec, logits.shape(), logits.device())?;
            Ok(self.inner.sample(&penalized)?)
        } else {
            Ok(self.inner.sample(logits)?)
        }
    }

    pub fn set_temperature(&mut self, temp: f64) {
        self.config.temperature = Some(temp);
        let sampling = Self::make_sampling(&self.config);
        self.greedy = matches!(sampling, Sampling::ArgMax);
        self.inner = LogitsProcessor::from_sampling(self.seed, sampling);
    }

    /// Fork an independent sampler for an inference session (`inference_session`).
    /// Same config, but the RNG stream is salted so concurrent sessions do
    /// not draw identical random sequences. `LogitsProcessor` is not
    /// `Clone`; rebuilding from `(seed, config)` is exact and cheap.
    pub fn fork(&self, salt: u64) -> Self {
        Self::new(self.seed.wrapping_add(salt), &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(
        temperature: Option<f64>,
        top_k: Option<usize>,
        top_p: Option<f64>,
    ) -> SamplingConfig {
        SamplingConfig {
            temperature,
            top_k,
            top_p,
            repeat_penalty: 1.0,
            repeat_last_n: 64,
        }
    }

    #[test]
    fn greedy_mapping_covers_stage8_scope() {
        // No temperature configured -> greedy (speculation allowed).
        assert!(VortexSampler::new(42, &config_with(None, None, None)).is_greedy());
        // Zero / near-zero temperature -> greedy.
        assert!(VortexSampler::new(42, &config_with(Some(0.0), None, None)).is_greedy());
        // Normal sampling modes -> NOT greedy (speculation must be skipped).
        assert!(!VortexSampler::new(42, &config_with(Some(0.7), None, None)).is_greedy());
        assert!(!VortexSampler::new(42, &config_with(Some(0.7), Some(40), None)).is_greedy());
        assert!(!VortexSampler::new(42, &config_with(Some(0.7), None, Some(0.9))).is_greedy());
    }

    #[test]
    fn argmax_is_deterministic_on_ties() {
        assert_eq!(argmax_index(&[1.0, 3.0, 3.0, 2.0]), 1);
        assert_eq!(argmax_index(&[5.0]), 0);
    }

    #[test]
    fn repeat_penalty_window_is_bounded() {
        let mut logits = vec![2.0f32; 10];
        let generated: Vec<u32> = (0..200).map(|i| (i % 10) as u32).collect();
        apply_repeat_penalty_window(&mut logits, &generated, 1.5, 64);
        // Only the last 64 tokens penalized; out-of-range ids ignored safely.
        assert!(logits.iter().all(|&v| v != 2.0));
        let mut short = vec![2.0f32; 10];
        apply_repeat_penalty_window(&mut short, &[3], 1.5, 64);
        assert_ne!(short[3], 2.0);
    }

    fn sample_token(sampler: &mut VortexSampler, bias: f32) -> u32 {
        // Skewed 8-token distribution at high temperature: draws vary.
        let logits = Tensor::new(
            &[bias, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            &candle_core::Device::Cpu,
        )
        .unwrap();
        sampler.sample(&logits, &[]).unwrap()
    }

    #[test]
    fn fork_preserves_sampling_config() {
        let parent = VortexSampler::new(42, &config_with(Some(0.7), None, None));
        assert!(!parent.is_greedy());
        assert!(!parent.fork(1).is_greedy());
        let greedy_parent = VortexSampler::new(42, &config_with(None, None, None));
        assert!(greedy_parent.fork(99).is_greedy());
    }

    #[test]
    fn fork_same_salt_is_deterministic() {
        let parent = VortexSampler::new(7, &config_with(Some(2.0), None, None));
        let mut a = parent.fork(5);
        let mut b = parent.fork(5);
        let draws_a: Vec<u32> = (0..16).map(|_| sample_token(&mut a, 1.0)).collect();
        let draws_b: Vec<u32> = (0..16).map(|_| sample_token(&mut b, 1.0)).collect();
        assert_eq!(draws_a, draws_b);
    }

    #[test]
    fn fork_salt_diverges_rng_streams() {
        // 16 draws on a flat-ish distribution: identical vectors across
        // different salts would require 16 consecutive RNG collisions
        // (P ~ vocab^-16) — treats RNG divergence as observable fact.
        let parent = VortexSampler::new(7, &config_with(Some(2.0), None, None));
        let mut a = parent.fork(1);
        let mut b = parent.fork(2);
        let draws_a: Vec<u32> = (0..16).map(|_| sample_token(&mut a, 0.5)).collect();
        let draws_b: Vec<u32> = (0..16).map(|_| sample_token(&mut b, 0.5)).collect();
        assert_ne!(draws_a, draws_b);
    }

    fn vocab_logits(vocab: usize) -> Tensor {
        // Deterministic non-flat distribution so sampling is meaningful.
        let data: Vec<f32> = (0..vocab)
            .map(|i| ((i * 17) % 100) as f32 / 100.0)
            .collect();
        Tensor::new(data.as_slice(), &candle_core::Device::Cpu).unwrap()
    }

    #[test]
    fn sample_with_scratch_nongreedy_restores_capacity() {
        let vocab = 4096;
        let mut sampler = VortexSampler::new(42, &config_with(Some(0.8), None, None));
        assert!(!sampler.is_greedy());
        let logits = vocab_logits(vocab);
        let mut scratch = Vec::with_capacity(vocab);
        // Prime once so capacity is non-zero after the first take/reserve.
        let _ = sampler
            .sample_with_scratch(&logits, &[], &mut scratch)
            .unwrap();
        assert!(
            scratch.capacity() >= vocab,
            "capacity must be restored after mem::take path (got {})",
            scratch.capacity()
        );
        // Repeated calls must not shrink below vocab (no per-token realloc to 0).
        for _ in 0..8 {
            let _ = sampler
                .sample_with_scratch(&logits, &[1, 2, 3], &mut scratch)
                .unwrap();
            assert!(
                scratch.capacity() >= vocab,
                "capacity held across samples (got {})",
                scratch.capacity()
            );
        }
    }

    #[test]
    fn sample_with_scratch_nongreedy_returns_valid_token() {
        let vocab = 512;
        let mut sampler = VortexSampler::new(7, &config_with(Some(0.9), Some(40), None));
        let logits = vocab_logits(vocab);
        let mut scratch = Vec::with_capacity(vocab);
        for _ in 0..32 {
            let tok = sampler
                .sample_with_scratch(&logits, &[0, 1], &mut scratch)
                .unwrap();
            assert!((tok as usize) < vocab, "token {tok} out of vocab {vocab}");
        }
    }

    #[test]
    fn sample_with_scratch_greedy_matches_argmax() {
        let vocab = 256;
        let mut sampler = VortexSampler::new(1, &config_with(Some(0.0), None, None));
        assert!(sampler.is_greedy());
        let logits = vocab_logits(vocab);
        let mut scratch = Vec::with_capacity(vocab);
        let tok = sampler
            .sample_with_scratch(&logits, &[], &mut scratch)
            .unwrap();
        let data: Vec<f32> = (0..vocab)
            .map(|i| ((i * 17) % 100) as f32 / 100.0)
            .collect();
        assert_eq!(tok as usize, argmax_index(&data));
    }

    /// tokens/sec proof for the mem::take non-greedy path (vocab 32k clone
    /// removed). Ignored in normal runs; execute explicitly and record.
    #[test]
    #[ignore]
    fn bench_sample_with_scratch_nongreedy_tps() {
        let vocab = 32_000;
        let mut sampler = VortexSampler::new(42, &config_with(Some(0.8), None, None));
        assert!(!sampler.is_greedy());
        let logits = vocab_logits(vocab);
        let mut scratch = Vec::with_capacity(vocab);
        // Warm-up (first take/reserve).
        let _ = sampler
            .sample_with_scratch(&logits, &[], &mut scratch)
            .unwrap();
        let iters = 2_000u32;
        let start = std::time::Instant::now();
        for i in 0..iters {
            let _ = sampler
                .sample_with_scratch(&logits, &[i % 100], &mut scratch)
                .unwrap();
        }
        let secs = start.elapsed().as_secs_f64();
        let tps = if secs > 0.0 { iters as f64 / secs } else { 0.0 };
        println!(
            "sample_with_scratch non-greedy mem::take: {iters} samples in {secs:.4}s → {tps:.1} samples/s (vocab={vocab}, capacity={})",
            scratch.capacity()
        );
        assert!(scratch.capacity() >= vocab, "capacity restored after bench");
    }
}
