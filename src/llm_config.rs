use candle_core::Device;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Device selection for model inference.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    #[default]
    Cpu,
    Cuda(usize),
}

impl DeviceType {
    pub fn to_device(&self) -> Device {
        match self {
            Self::Cpu => Device::Cpu,
            #[cfg(feature = "cuda")]
            Self::Cuda(ord) => {
                let cuda_device = candle_core::CudaDevice::new(*ord);
                match cuda_device {
                    Ok(cuda) => Device::Cuda(cuda),
                    Err(e) => {
                        eprintln!("[Vortex] CUDA device {ord} failed: {e}, falling back to CPU");
                        Device::Cpu
                    }
                }
            }
            #[cfg(not(feature = "cuda"))]
            Self::Cuda(ord) => {
                eprintln!("[Vortex] CUDA feature not enabled (requested device {ord}), using CPU");
                Device::Cpu
            }
        }
    }
}

/// Supported quantized model architectures in candle-transformers.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelArchitecture {
    Llama,
    Phi3,
    Qwen2,
    Qwen3,
    Gemma3,
}

impl ModelArchitecture {
    pub fn from_gguf_str(s: &str) -> Option<Self> {
        match s {
            "llama" => Some(Self::Llama),
            "phi3" => Some(Self::Phi3),
            "qwen2" => Some(Self::Qwen2),
            "qwen3" => Some(Self::Qwen3),
            "gemma3" => Some(Self::Gemma3),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModelArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Llama => write!(f, "llama"),
            Self::Phi3 => write!(f, "phi3"),
            Self::Qwen2 => write!(f, "qwen2"),
            Self::Qwen3 => write!(f, "qwen3"),
            Self::Gemma3 => write!(f, "gemma3"),
        }
    }
}

/// Sampling parameters for LLM generation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SamplingConfig {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<usize>,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            temperature: Some(0.8),
            top_p: Some(0.95),
            top_k: None,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
        }
    }
}

/// Standard model size constants (approximate parameter counts).
pub mod model_sizes {
    pub const P0_135B: u64 = 135_000_000;
    pub const P0_5B: u64 = 500_000_000;
    pub const P1_5B: u64 = 1_500_000_000;
    pub const P3B: u64 = 3_000_000_000;
    pub const P7B: u64 = 7_000_000_000;
    pub const P8B: u64 = 8_000_000_000;
}

/// Configuration for the Vortex LLM inference engine.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LlmConfig {
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub architecture: Option<ModelArchitecture>,
    pub device_type: DeviceType,
    pub sampling: SamplingConfig,
    pub max_seq_len: usize,
    pub max_generation_tokens: usize,
    pub system_prompt: String,
    pub use_flash_attn: bool,
    pub seed: u64,
    pub model_size_params: u64,
    pub model_quantization: String,
    /// When true, prefault mapped model pages at startup (background thread).
    #[serde(default)]
    pub prefault: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/model.gguf"),
            tokenizer_path: PathBuf::from("models/tokenizer.json"),
            architecture: None,
            device_type: DeviceType::Cpu,
            sampling: SamplingConfig::default(),
            max_seq_len: 4096,
            max_generation_tokens: 2048,
            system_prompt: "You are Vortex Atoms AI, a helpful assistant integrated into a low-latency 5-Kernel Matrix runtime. Be concise, accurate, and helpful.".to_string(),
            use_flash_attn: false,
            seed: 42,
            model_size_params: model_sizes::P0_5B,
            model_quantization: "Q4_K_M".to_string(),
            prefault: false,
        }
    }
}
