use std::fmt::{Display, Formatter};
use std::path::PathBuf;

/// Unified error type for the Vortex Atoms AI kernel foundation.
#[derive(Debug)]
pub enum VortexAtomsError {
    Io(std::io::Error),
    Candle(candle_core::Error),
    EmptyWeightFile(PathBuf),
    InvalidRange {
        offset: u64,
        len: usize,
        file_len: usize,
    },
    Overflow(&'static str),
    InvalidTensorSpec(String),
    InvalidKnowledgeExtension(String),
    Tokenizers(tokenizers::Error),
    Gguf(String),
    ModelArchitectureUnsupported(String),
    Generation(String),
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, VortexAtomsError>;

impl Display for VortexAtomsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Candle(err) => write!(f, "Candle error: {err}"),
            Self::EmptyWeightFile(path) => {
                write!(f, "weight file is empty and cannot be memory mapped: {}", path.display())
            }
            Self::InvalidRange {
                offset,
                len,
                file_len,
            } => write!(
                f,
                "requested weight range offset={offset} len={len} exceeds mapped file length={file_len}"
            ),
            Self::Overflow(context) => write!(f, "integer overflow while computing {context}"),
            Self::InvalidTensorSpec(message) => write!(f, "invalid tensor spec: {message}"),
            Self::InvalidKnowledgeExtension(message) => {
                write!(f, "invalid knowledge extension: {message}")
            }
            Self::Tokenizers(err) => write!(f, "tokenizer error: {err}"),
            Self::Gguf(msg) => write!(f, "GGUF error: {msg}"),
            Self::ModelArchitectureUnsupported(arch) => {
                write!(f, "unsupported model architecture: {arch}")
            }
            Self::Generation(msg) => write!(f, "generation error: {msg}"),
            Self::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
        }
    }
}

impl VortexAtomsError {
    /// Caller-safe message for API responses: filesystem variants are mapped
    /// to a generic string so internal paths never leak to remote callers.
    /// Validation messages (`Gguf`/`Generation`/...) pass through — they only
    /// ever echo caller-controlled input or model behavior.
    pub fn public_message(&self) -> String {
        match self {
            Self::Io(_) => "filesystem error".to_string(),
            Self::EmptyWeightFile(_) => "weight file is empty".to_string(),
            other => other.to_string(),
        }
    }
}

impl std::error::Error for VortexAtomsError {}

impl From<std::io::Error> for VortexAtomsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<candle_core::Error> for VortexAtomsError {
    fn from(value: candle_core::Error) -> Self {
        Self::Candle(value)
    }
}

impl From<tokenizers::Error> for VortexAtomsError {
    fn from(value: tokenizers::Error) -> Self {
        Self::Tokenizers(value)
    }
}
