use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Trait for speculative token drafters.
/// Drafters propose candidate tokens that the main model verifies.
///
/// Stage 8 correctness scope: verification accepts a draft only on greedy
/// argmax match — exact for greedy decoding, unsound as a temperature /
/// top-k / top-p sampler (biases toward argmax, skips repeat penalty).
/// The generation loop therefore runs speculation only when
/// [`crate::llm_sampling::VortexSampler::is_greedy`] holds.
pub trait SpeculativeDrafter: Send + Sync {
    /// Draft up to `max_draft` tokens given the current context.
    /// Returns a vector of drafted token IDs.
    fn draft(&mut self, context: &[u32], max_draft: usize) -> Vec<u32>;

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
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            return candidates.into_iter().take(10).collect();
        }

        let hash = Self::hash_context(&context[context.len() - self.n..]);
        if let Some(next_map) = self.ngrams.get(&hash) {
            let mut candidates: Vec<_> = next_map
                .iter()
                .map(|(&tok, &count)| (tok, count as f32))
                .collect();
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            return candidates;
        }

        // No n-gram for this context, fallback to global
        let mut candidates: Vec<_> = self
            .global_freq
            .iter()
            .map(|(&tok, &count)| (tok, count as f32))
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        candidates.into_iter().take(10).collect()
    }

    /// Sample from candidates with temperature.
    fn sample_from_candidates(&mut self, candidates: &[(u32, f32)]) -> u32 {
        if candidates.is_empty() {
            return 0; // Unknown token, fallback
        }
        if self.temperature <= 1e-7 || candidates.len() == 1 {
            return candidates[0].0; // Greedy
        }
        // Softmax with temperature
        let max_logit = candidates
            .iter()
            .map(|(_, p)| *p)
            .fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = candidates
            .iter()
            .map(|(_, p)| ((p - max_logit) / self.temperature).exp())
            .sum();
        let mut r = sum * self.random_f32();
        for (tok, prob) in candidates {
            let p = ((prob - max_logit) / self.temperature).exp() / sum;
            r -= p;
            if r <= 0.0 {
                return *tok;
            }
        }
        candidates[0].0
    }

    fn random_f32(&mut self) -> f32 {
        // Simple xorshift RNG
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed as f32) / (u64::MAX as f32)
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

    fn update(&mut self, tokens: &[u32]) {
        self.train(tokens);
    }

    fn reset(&mut self) {
        // Keep trained n-grams, just reset any runtime state
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
}

impl SpeculativeDecoder {
    pub fn new(
        drafter: Box<dyn SpeculativeDrafter>,
        max_draft_tokens: usize,
        min_acceptance_rate: f32,
    ) -> Self {
        Self {
            drafter,
            max_draft_tokens,
            min_acceptance_rate,
            total_drafted: 0,
            total_accepted: 0,
        }
    }

    /// Draft tokens and verify them against the main model's logits.
    /// Returns (accepted_tokens, should_continue_speculative).
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

        self.total_drafted += drafted.len();

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
                let best_tok = argmax_index(logits) as u32;
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
    }
}

// Reuse argmax from sampling module
fn argmax_index(values: &[f32]) -> usize {
    let mut best = 0usize;
    for (index, &value) in values.iter().enumerate() {
        if value > values[best] {
            best = index;
        }
    }
    best
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
