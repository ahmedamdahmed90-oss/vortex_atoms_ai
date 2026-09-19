use serde::Serialize;
use tokenizers::Tokenizer;

use crate::Result;

/// Events emitted during streaming token generation.
#[derive(Serialize)]
pub enum StreamEvent {
    Token {
        token_id: u32,
        text: String,
        pos: usize,
    },
    Done {
        total_tokens: usize,
        tokens_per_second: f64,
        full_text: String,
    },
    Error(String),
}

/// Incremental token-to-text decoder.
/// Accumulates tokens and emits decoded text fragments.
pub struct TokenOutputStream {
    tokenizer: Tokenizer,
    current_tokens: Vec<u32>,
}

impl TokenOutputStream {
    pub fn new(tokenizer: Tokenizer) -> Self {
        Self {
            tokenizer,
            current_tokens: Vec::new(),
        }
    }

    pub fn next_token(&mut self, token_id: u32) -> Result<Option<String>> {
        self.current_tokens.push(token_id);
        let text = self
            .tokenizer
            .decode(&self.current_tokens, true)
            .map_err(crate::error::VortexAtomsError::Tokenizers)?;
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    pub fn decode_all(&self) -> Result<String> {
        let text = self
            .tokenizer
            .decode(&self.current_tokens, true)
            .map_err(crate::error::VortexAtomsError::Tokenizers)?;
        Ok(text)
    }

    pub fn token_count(&self) -> usize {
        self.current_tokens.len()
    }

    pub fn reset(&mut self) {
        self.current_tokens.clear();
    }
}
