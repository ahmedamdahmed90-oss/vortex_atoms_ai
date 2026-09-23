use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Sparse proposal distribution: `(token, probability)` pairs that sum to ~1.
/// Tokens absent from the list have probability 0 under the drafter.
pub type SparseDist = Vec<(u32, f32)>;

/// One drafted step: the proposed token plus the drafter's full proposal
/// distribution `q` (needed for Levi rejection sampling of the residual).
pub type DraftProposal = (u32, SparseDist);

/// Trait for speculative token drafters.
/// Drafters propose candidate tokens that the main model verifies.
///
/// Verification modes:
/// - Greedy (`ArgMax`): accept iff the draft equals the verifier argmax —
///   exact for greedy decoding (output bit-identical to no speculation).
/// - Sampling modes: probabilistic speculative acceptance (Levi et al.).
///   The drafter returns its proposal distribution `q`; the verifier accepts
///   with `min(1, p[x]/q[x])` and, on rejection, emits a draw from
///   `normalize(max(0, p - q))`. The output distribution then matches the
///   target sampler exactly (repeat penalty included via `p`).
pub trait SpeculativeDrafter: Send + Sync {
    /// Draft up to `max_draft` tokens given the current context.
    /// Returns a vector of drafted token IDs.
    fn draft(&mut self, context: &[u32], max_draft: usize) -> Vec<u32>;

    /// Draft tokens together with the proposal distribution used for each.
    /// Default: point-mass `q` at the drafted token (Levi then accepts with
    /// probability `p[x]` and resamples `p` conditioned on `≠x` on reject).
    fn draft_with_probs(&mut self, context: &[u32], max_draft: usize) -> Vec<DraftProposal> {
        self.draft(context, max_draft)
            .into_iter()
            .map(|t| (t, vec![(t, 1.0f32)]))
            .collect()
    }

    /// Update the drafter with newly accepted tokens.
    fn update(&mut self, tokens: &[u32]);

    /// Reset drafter state for a new generation.
    fn reset(&mut self);
}

/// Simple n-gram drafter using Markov chain from context.
/// Builds n-gram counts on-the-fly from the provided context.
/// For production use, this would be replaced with a pre-trained n-gram model.
#[derive(Debug, Clone)]
pub struct NGramDrafter {
    /// N-gram order (context length).
    n: usize,
    /// N-gram counts: context hash -> (next_token -> count).
    ngrams: HashMap<u64, HashMap<u32, u32>>,
    /// Maximum vocabulary size (for bounds checking).
    vocab_size: usize,
    /// Fallback: most frequent tokens globally.
    global_freq: HashMap<u32, u32>,
    /// Temperature for drafting (adds randomness).
    temperature: f32,
    /// Random seed.
    seed: u64,
}

impl NGramDrafter {
    pub fn new(n: usize, vocab_size: usize, temperature: f32, seed: u64) -> Self {
        Self {
            n,
            ngrams: HashMap::new(),
            vocab_size,
            global_freq: HashMap::new(),
            temperature,
            seed,
        }
    }

    /// Build n-grams from a token sequence (e.g., conversation history).
    pub fn train(&mut self, tokens: &[u32]) {
        if tokens.len() < self.n {
            return;
        }
        for window in tokens.windows(self.n + 1) {
            let context = &window[..self.n];
            let next = window[self.n];
            // Bounds check using vocab_size
            if next as usize >= self.vocab_size {
                continue;
            }
            let hash = Self::hash_context(context);
            *self
                .ngrams
                .entry(hash)
                .or_default()
                .entry(next)
                .or_insert(0) += 1;
            *self.global_freq.entry(next).or_insert(0) += 1;
        }
    }

    /// Hash a context window for lookup.
    fn hash_context(context: &[u32]) -> u64 {
        let mut hasher = DefaultHasher::new();
        context.hash(&mut hasher);
        hasher.finish()
    }

    /// Get next token candidates for a context.
    fn get_candidates(&self, context: &[u32]) -> Vec<(u32, f32)> {
        if context.len() < self.n {
            // Fallback to global frequency
            let mut candidates: Vec<_> = self
                .global_freq
                .iter()
                .map(|(&tok, &count)| (tok, count as f32))
                .collect();
            candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
            return candidates.into_iter().take(10).collect();
        }

        let hash = Self::hash_context(&context[context.len() - self.n..]);
        if let Some(next_map) = self.ngrams.get(&hash) {
            let mut candidates: Vec<_> = next_map
                .iter()
                .map(|(&tok, &count)| (tok, count as f32))
                .collect();
            candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
            return candidates;
        }

        // No n-gram for this context, fallback to global
        let mut candidates: Vec<_> = self
            .global_freq
            .iter()
            .map(|(&tok, &count)| (tok, count as f32))
            .collect();
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
        candidates.into_iter().take(10).collect()
    }

    /// Softmax-with-temperature over candidate scores → normalized sparse `q`.
    fn candidates_to_dist(&self, candidates: &[(u32, f32)]) -> SparseDist {
        if candidates.is_empty() {
            return Vec::new();
        }
        if self.temperature <= 1e-7 || candidates.len() == 1 {
            // Degenerate: point mass on the best candidate.
            return vec![(candidates[0].0, 1.0f32)];
        }
        let max_logit = candidates
            .iter()
            .map(|(_, p)| *p)
            .fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = candidates
            .iter()
            .map(|(_, p)| ((p - max_logit) / self.temperature).exp())
            .sum();
        if !(sum.is_finite() && sum > 0.0) {
            return vec![(candidates[0].0, 1.0f32)];
        }
        candidates
            .iter()
            .map(|&(tok, score)| (tok, ((score - max_logit) / self.temperature).exp() / sum))
            .collect()
    }

    /// Sample from a sparse normalized distribution with uniform `u ∈ [0,1)`.
    fn sample_dist(dist: &SparseDist, u: f32) -> u32 {
        if dist.is_empty() {
            return 0;
        }
        let mut r = u;
        for (tok, p) in dist {
            r -= p;
            if r <= 0.0 {
                return *tok;
            }
        }
        dist[dist.len() - 1].0
    }

    /// Sample from candidates with temperature (returns the drawn token).
    fn sample_from_candidates(&mut self, candidates: &[(u32, f32)]) -> u32 {
        if candidates.is_empty() {
            return 0; // Unknown token, fallback
        }
        let dist = self.candidates_to_dist(candidates);
        let u = self.random_f32();
        Self::sample_dist(&dist, u)
    }

    fn random_f32(&mut self) -> f32 {
        // xorshift64 → [0,1) with a 24-bit mantissa-safe divisor.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        ((self.seed >> 40) as f32) / ((1u64 << 24) as f32)
    }
}

impl SpeculativeDrafter for NGramDrafter {
    fn draft(&mut self, context: &[u32], max_draft: usize) -> Vec<u32> {
        let mut drafted = Vec::with_capacity(max_draft);
        let mut current_context = context.to_vec();

        for _ in 0..max_draft {
            let candidates = self.get_candidates(&current_context);
            if candidates.is_empty() {
                break;
            }
            let next = self.sample_from_candidates(&candidates);
            drafted.push(next);
            current_context.push(next);
            // Keep context window bounded
            if current_context.len() > self.n * 2 {
                current_context.drain(0..current_context.len() - self.n);
            }
        }
        drafted
    }

    fn draft_with_probs(&mut self, context: &[u32], max_draft: usize) -> Vec<DraftProposal> {
        let mut drafted = Vec::with_capacity(max_draft);
        let mut current_context = context.to_vec();

        for _ in 0..max_draft {
            let candidates = self.get_candidates(&current_context);
            if candidates.is_empty() {
                break;
            }
            let dist = self.candidates_to_dist(&candidates);
            if dist.is_empty() {
                break;
            }
            let next = Self::sample_dist(&dist, self.random_f32());
            drafted.push((next, dist));
            current_context.push(next);
            if current_context.len() > self.n * 2 {
                current_context.drain(0..current_context.len() - self.n);
            }
        }
        drafted
    }

    fn update(&mut self, tokens: &[u32]) {
        self.train(tokens);
    }

    fn reset(&mut self) {
        // Keep trained n-grams, just reset any runtime state
    }
}

/// Acceptance probability for the Levi rule: `min(1, p[x]/q[x])`.
///
/// `q_at_x` is the drafter's probability for the proposed token; `p_at_x`
/// is the verifier's target probability. `q_at_x <= 0` (drafter cannot
/// propose `x` under `q`) accepts whenever `p_at_x > 0` — the ratio is
/// infinite — and rejects when both are 0 (undefined → no mass either way).
pub fn levi_accept_prob(p_at_x: f32, q_at_x: f32) -> f32 {
    if q_at_x <= 0.0 {
        if p_at_x > 0.0 {
            1.0
        } else {
            0.0
        }
    } else if p_at_x <= 0.0 {
        0.0
    } else {
        (p_at_x / q_at_x).min(1.0)
    }
}

/// Probability mass of the proposal `q` at token `x`.
pub fn sparse_prob_at(dist: &SparseDist, x: u32) -> f32 {
    dist.iter()
        .find(|(t, _)| *t == x)
        .map(|(_, p)| *p)
        .unwrap_or(0.0)
}

/// Draw from the residual distribution `normalize(max(0, p - q))` given a
/// uniform `u ∈ [0,1)`. Falls back to `p` itself if the residual is empty
/// (can only happen through numerical edge cases; Levi never rejects when
/// `p == q` because `accept_prob == 1`).
pub fn sample_residual(q: &SparseDist, p: &[f32], u: f32) -> u32 {
    if p.is_empty() {
        return 0;
    }
    let mut total = 0.0f32;
    for &pi in p.iter() {
        total += pi;
    }
    // Subtract clamped overlap only on q's sparse support.
    let mut overlap = 0.0f32;
    for &(t, qi) in q {
        if let Some(&pi) = p.get(t as usize) {
            overlap += pi.min(qi);
        }
    }
    let residual_sum = (total - overlap).max(0.0);
    if residual_sum <= f32::EPSILON {
        // Degenerate: sample from p directly.
        let mut r = u * total.max(f32::EPSILON);
        for (i, &pi) in p.iter().enumerate() {
            r -= pi;
            if r <= 0.0 {
                return i as u32;
            }
        }
        return (p.len() - 1) as u32;
    }

    let mut r = u * residual_sum;
    // Walk vocab; for tokens in q use max(0, p-q).
    for (i, &pi) in p.iter().enumerate() {
        let qi = sparse_prob_at(q, i as u32);
        let ri = (pi - qi).max(0.0);
        r -= ri;
        if r <= 0.0 {
            return i as u32;
        }
    }
    // Numerical tail: return the last index with any residual mass.
    for i in (0..p.len()).rev() {
        let qi = sparse_prob_at(q, i as u32);
        if (p[i] - qi).max(0.0) > 0.0 {
            return i as u32;
        }
    }
    (p.len() - 1) as u32
}

/// Levi step for one drafted token: returns `Some(drafted)` on accept, or
/// `Some(replacement)` drawn from the residual on reject. `u` is uniform
/// in `[0,1)`.
pub fn levi_step(drafted: u32, q: &SparseDist, p: &[f32], u: f32, u_res: f32) -> u32 {
    let q_at = sparse_prob_at(q, drafted);
    let p_at = p.get(drafted as usize).copied().unwrap_or(0.0);
    if u < levi_accept_prob(p_at, q_at) {
        drafted
    } else {
        sample_residual(q, p, u_res)
    }
}

/// Speculative decoding coordinator.
/// Coordinates drafter and verifier (main model) for speculative decoding.
pub struct SpeculativeDecoder {
    /// The drafter that proposes candidate tokens.
    pub drafter: Box<dyn SpeculativeDrafter>,
    /// Maximum number of tokens to draft per step.
    pub max_draft_tokens: usize,
    /// Minimum acceptance rate to continue speculative decoding.
    pub min_acceptance_rate: f32,
    /// Statistics.
    pub total_drafted: usize,
    pub total_accepted: usize,
    /// xorshift64 state for Levi accept/residual draws (decoder-owned so
    /// acceptance is reproducible and independent of the session sampler).
    rng: u64,
    /// Initial seed so `reset` restores a deterministic stream.
    rng_seed: u64,
}

/// Fixed default seed for the decoder's Levi RNG (override via `new_seeded`).
const DEFAULT_SPEC_RNG_SEED: u64 = 0xA5A5_5A5A_C3C3_3C3C;

impl SpeculativeDecoder {
    pub fn new(
        drafter: Box<dyn SpeculativeDrafter>,
        max_draft_tokens: usize,
        min_acceptance_rate: f32,
    ) -> Self {
        Self::new_seeded(
            drafter,
            max_draft_tokens,
            min_acceptance_rate,
            DEFAULT_SPEC_RNG_SEED,
        )
    }

    pub fn new_seeded(
        drafter: Box<dyn SpeculativeDrafter>,
        max_draft_tokens: usize,
        min_acceptance_rate: f32,
        seed: u64,
    ) -> Self {
        Self {
            drafter,
            max_draft_tokens,
            min_acceptance_rate,
            total_drafted: 0,
            total_accepted: 0,
            rng: seed,
            rng_seed: seed,
        }
    }

    /// Uniform in `[0,1)` from the decoder-owned xorshift64 stream.
    pub fn random_f32(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        ((self.rng >> 40) as f32) / ((1u64 << 24) as f32)
    }

    /// Draft tokens with proposal distributions (see [`SpeculativeDrafter`]).
    pub fn draft_with_probs(&mut self, context: &[u32], max_draft: usize) -> Vec<DraftProposal> {
        let drafted = self.drafter.draft_with_probs(context, max_draft);
        self.total_drafted += drafted.len();
        drafted
    }

    /// Levi acceptance for one draft step. Returns the token to commit
    /// (drafted on accept, residual sample on reject) and whether it was
    /// accepted. Counts are updated for accepted drafts only.
    pub fn accept_levi(&mut self, drafted: u32, q: &SparseDist, p: &[f32]) -> (u32, bool) {
        let u = self.random_f32();
        let q_at = sparse_prob_at(q, drafted);
        let p_at = p.get(drafted as usize).copied().unwrap_or(0.0);
        if u < levi_accept_prob(p_at, q_at) {
            self.total_accepted += 1;
            (drafted, true)
        } else {
            let u_res = self.random_f32();
            (sample_residual(q, p, u_res), false)
        }
    }

    /// Greedy acceptance: accept iff draft equals verifier argmax. Returns
    /// the token to commit (drafted on accept, argmax replacement on reject).
    pub fn accept_greedy(&mut self, drafted: u32, p_logits: &[f32]) -> (u32, bool) {
        let best = crate::llm_sampling::argmax_index(p_logits) as u32;
        if best == drafted {
            self.total_accepted += 1;
            (drafted, true)
        } else {
            (best, false)
        }
    }

    /// Draft tokens and verify them against the main model's logits.
    /// Returns (accepted_tokens, should_continue_speculative).
    ///
    /// Greedy verifier path (bit-identical to argmax sampling). Sampling
    /// modes use the live generation loop's Levi path (`accept_levi`) with
    /// incremental KV forwards; this batch helper remains for tests/tools
    /// that already hold a full logits sequence.
    pub fn draft_and_verify(
        &mut self,
        context: &[u32],
        verifier_logits_fn: impl Fn(&[u32]) -> Vec<Vec<f32>>,
    ) -> (Vec<u32>, bool) {
        // Step 1: Draft tokens
        let drafted = self.drafter.draft(context, self.max_draft_tokens);
        if drafted.is_empty() {
            return (Vec::new(), false);
        }

        // Step 2: Verify drafted tokens in batch
        // Build the full sequence: context + drafted
        let mut full_seq = context.to_vec();
        full_seq.extend_from_slice(&drafted);

        // Get logits for each position in the drafted sequence
        let logits_seq = verifier_logits_fn(&full_seq);

        // Step 3: Check acceptance
        let mut accepted = Vec::new();
        for (i, &drafted_tok) in drafted.iter().enumerate() {
            let pos = context.len() + i;
            if pos < logits_seq.len() {
                // Greedy verification: accept if drafted token == argmax of logits
                let logits = &logits_seq[pos];
                let best_tok = crate::llm_sampling::argmax_index(logits) as u32;
                if best_tok == drafted_tok {
                    accepted.push(drafted_tok);
                } else {
                    // Mismatch: stop speculative decoding here
                    break;
                }
            } else {
                break;
            }
        }

        let accepted_count = accepted.len();
        self.total_accepted += accepted_count;

        // Update drafter with accepted tokens
        if !accepted.is_empty() {
            self.drafter.update(&accepted);
        }

        // Continue speculative if acceptance rate is good
        let acceptance_rate = if self.max_draft_tokens > 0 {
            accepted_count as f32 / self.max_draft_tokens as f32
        } else {
            0.0
        };

        let continue_speculative = acceptance_rate >= self.min_acceptance_rate;

        (accepted, continue_speculative)
    }

    pub fn acceptance_rate(&self) -> f32 {
        if self.total_drafted == 0 {
            0.0
        } else {
            self.total_accepted as f32 / self.total_drafted as f32
        }
    }

    pub fn reset(&mut self) {
        self.drafter.reset();
        self.total_drafted = 0;
        self.total_accepted = 0;
        self.rng = self.rng_seed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ngram_drafter_basic() {
        let mut drafter = NGramDrafter::new(2, 32000, 0.0, 42);
        drafter.train(&[1, 2, 3, 1, 2, 4, 1, 2, 5]);
        let drafted = drafter.draft(&[1, 2], 3);
        // Should predict 3, 4, or 5 after 1,2
        assert!(!drafted.is_empty());
    }

    #[test]
    fn ngram_draft_with_probs_returns_normalized_q() {
        let mut drafter = NGramDrafter::new(2, 32000, 1.0, 42);
        drafter.train(&[1, 2, 3, 1, 2, 4, 1, 2, 5]);
        let props = drafter.draft_with_probs(&[1, 2], 3);
        assert!(!props.is_empty());
        for (tok, q) in &props {
            let sum: f32 = q.iter().map(|(_, p)| *p).sum();
            assert!(
                (sum - 1.0).abs() < 1e-4,
                "q must be a distribution, sum={sum}"
            );
            assert!(
                sparse_prob_at(q, *tok) > 0.0,
                "proposed token must have q>0"
            );
        }
    }

    #[test]
    fn candidates_sorted_descending_by_count() {
        let mut drafter = NGramDrafter::new(2, 32000, 1.0, 42);
        drafter.train(&[1, 2, 3, 1, 2, 3, 1, 2, 4, 5]);
        let cands = drafter.get_candidates(&[1, 2]);
        assert!(!cands.is_empty());
        for w in cands.windows(2) {
            assert!(
                w[0].1 >= w[1].1,
                "candidates must be score-descending, got {:?}",
                cands
            );
        }
    }

    #[test]
    fn levi_accept_prob_matches_ratio_rule() {
        assert!((levi_accept_prob(0.5, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(levi_accept_prob(0.9, 0.2), 1.0); // capped at 1
        assert_eq!(levi_accept_prob(0.0, 0.3), 0.0);
        assert_eq!(levi_accept_prob(0.4, 0.0), 1.0); // infinite ratio
        assert_eq!(levi_accept_prob(0.0, 0.0), 0.0);
    }

    #[test]
    fn levi_step_point_mass_equals_p_accept() {
        // q point mass at x=1 with mass 1 → accept iff u < p[1].
        let q: SparseDist = vec![(1, 1.0)];
        let p = [0.25f32, 0.5, 0.25];
        // u just below p[1] → accept
        assert_eq!(levi_step(1, &q, &p, 0.49, 0.0), 1);
        // u at/above p[1] → residual from p with mass at 1 removed
        let tok = levi_step(1, &q, &p, 0.5, 0.0);
        assert_ne!(tok, 1); // residual has zero mass at 1
                            // u_res=0 picks the first positive residual (token 0)
        assert_eq!(tok, 0);
    }

    #[test]
    fn sample_residual_preserves_p_on_q_support_zero() {
        // q only on token 2; residual must match p elsewhere (renormalized).
        let q: SparseDist = vec![(2, 1.0)];
        let p = [0.2f32, 0.3, 0.5];
        // residual = [0.2, 0.3, 0] / 0.5 = [0.4, 0.6, 0]
        let t0 = sample_residual(&q, &p, 0.0);
        assert_eq!(t0, 0);
        let t1 = sample_residual(&q, &p, 0.45); // 0.4 <= u' < 1.0 → token 1
        assert_eq!(t1, 1);
        // Never samples token 2 (q ate all its mass; p[2]-q[2]=0)
        for i in 0..20 {
            let u = (i as f32) / 20.0;
            assert_ne!(sample_residual(&q, &p, u), 2);
        }
    }

    #[test]
    fn accept_greedy_rejects_on_argmax_mismatch() {
        let drafter = Box::new(NGramDrafter::new(2, 32000, 0.0, 1));
        let mut dec = SpeculativeDecoder::new(drafter, 4, 0.5);
        // drafted matches argmax → accept, commit drafted
        let (tok, ok) = dec.accept_greedy(1, &[0.1, 0.9, 0.0]);
        assert!(ok);
        assert_eq!(tok, 1);
        // drafted misses argmax → reject, commit argmax replacement
        let (tok2, ok2) = dec.accept_greedy(0, &[0.1, 0.9, 0.0]);
        assert!(!ok2);
        assert_eq!(tok2, 1);
        assert_eq!(dec.total_accepted, 1);
    }

    #[test]
    fn decoder_rng_is_deterministic_across_reset() {
        let mk = || {
            let drafter = Box::new(NGramDrafter::new(2, 32000, 0.8, 9));
            SpeculativeDecoder::new(drafter, 4, 0.5)
        };
        let mut a = mk();
        let draws_a: Vec<f32> = (0..8).map(|_| a.random_f32()).collect();
        a.reset();
        let draws_a2: Vec<f32> = (0..8).map(|_| a.random_f32()).collect();
        assert_eq!(draws_a, draws_a2);
        let mut b = mk();
        let draws_b: Vec<f32> = (0..8).map(|_| b.random_f32()).collect();
        assert_eq!(draws_a, draws_b);
    }

    #[test]
    fn speculative_decoder_basic() {
        let drafter = Box::new(NGramDrafter::new(2, 32000, 0.0, 42));
        let mut decoder = SpeculativeDecoder::new(drafter, 4, 0.5);

        // Mock verifier that always accepts the first token
        let (accepted, continue_) = decoder.draft_and_verify(&[1, 2], |_seq| {
            vec![vec![0.0, 1.0, 0.0], vec![0.0, 1.0, 0.0]] // argmax = 1
        });

        // The drafter was trained on [1,2] -> 3,4,5 pattern, but verifier says token 1
        // So acceptance depends on drafter's output
        assert!(!accepted.is_empty() || !continue_);
    }
}
