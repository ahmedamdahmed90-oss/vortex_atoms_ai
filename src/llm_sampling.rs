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
        // Apply windowed penalty in place on scratch, then create penalized tensor
        // from scratch (single Tensor alloc, no HashMap on greedy).
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
        let penalized = Tensor::from_vec(scratch.clone(), logits.shape(), logits.device())?;
        Ok(self.inner.sample(&penalized)?)
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
}
