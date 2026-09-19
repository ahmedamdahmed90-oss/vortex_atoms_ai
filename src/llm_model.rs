use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use std::io::{Read, Seek};

use crate::llm_config::ModelArchitecture;
use crate::Result;

/// Unified trait for all quantized models supported by the Vortex LLM engine.
pub trait LlmModel: Send {
    /// Forward pass: given token IDs tensor and position index, return logits.
    fn forward(&mut self, input: &Tensor, pos: usize) -> Result<Tensor>;

    /// Clear the KV cache for a new generation.
    fn clear_kv_cache(&mut self);
}

/// Concrete enum dispatching to specific quantized model implementations.
pub enum LlmModelEnum {
    Llama(candle_transformers::models::quantized_llama::ModelWeights),
    Phi3(candle_transformers::models::quantized_phi3::ModelWeights),
    Qwen2(candle_transformers::models::quantized_qwen2::ModelWeights),
    Qwen3(candle_transformers::models::quantized_qwen3::ModelWeights),
    Gemma3(candle_transformers::models::quantized_gemma3::ModelWeights),
}

impl LlmModel for LlmModelEnum {
    fn forward(&mut self, input: &Tensor, pos: usize) -> Result<Tensor> {
        match self {
            Self::Llama(m) => Ok(m.forward(input, pos)?),
            Self::Phi3(m) => Ok(m.forward(input, pos)?),
            Self::Qwen2(m) => Ok(m.forward(input, pos)?),
            Self::Qwen3(m) => Ok(m.forward(input, pos)?),
            Self::Gemma3(m) => Ok(m.forward(input, pos)?),
        }
    }

    fn clear_kv_cache(&mut self) {
        match self {
            Self::Llama(m) => m.clear_kv_cache(),
            Self::Phi3(_m) => {
                // phi3 doesn't expose clear_kv_cache, forward handles positions
            }
            Self::Qwen2(m) => m.clear_kv_cache(),
            Self::Qwen3(_m) => {
                // qwen3 doesn't expose clear_kv_cache in all versions
            }
            Self::Gemma3(_m) => {
                // gemma3 forward handles positions
            }
        }
    }
}

/// Load a quantized model from a GGUF file, auto-detecting architecture.
pub fn load_gguf_model(
    content: gguf_file::Content,
    reader: &mut (impl Read + Seek),
    device: &Device,
    arch: &ModelArchitecture,
    use_flash_attn: bool,
) -> Result<LlmModelEnum> {
    let model = match arch {
        ModelArchitecture::Llama => {
            let m = candle_transformers::models::quantized_llama::ModelWeights::from_gguf(
                content, reader, device,
            )?;
            LlmModelEnum::Llama(m)
        }
        ModelArchitecture::Phi3 => {
            let m = candle_transformers::models::quantized_phi3::ModelWeights::from_gguf(
                use_flash_attn,
                content,
                reader,
                device,
            )?;
            LlmModelEnum::Phi3(m)
        }
        ModelArchitecture::Qwen2 => {
            let m = candle_transformers::models::quantized_qwen2::ModelWeights::from_gguf(
                content, reader, device,
            )?;
            LlmModelEnum::Qwen2(m)
        }
        ModelArchitecture::Qwen3 => {
            let m = candle_transformers::models::quantized_qwen3::ModelWeights::from_gguf(
                content, reader, device,
            )?;
            LlmModelEnum::Qwen3(m)
        }
        ModelArchitecture::Gemma3 => {
            let m = candle_transformers::models::quantized_gemma3::ModelWeights::from_gguf(
                content, reader, device,
            )?;
            LlmModelEnum::Gemma3(m)
        }
    };
    Ok(model)
}

/// Detect model architecture from GGUF metadata.
pub fn detect_architecture(content: &gguf_file::Content) -> Result<ModelArchitecture> {
    match content.metadata.get("general.architecture") {
        Some(gguf_file::Value::String(arch)) => {
            ModelArchitecture::from_gguf_str(arch).ok_or_else(|| {
                crate::error::VortexAtomsError::ModelArchitectureUnsupported(arch.clone())
            })
        }
        Some(_) => Err(crate::error::VortexAtomsError::Gguf(
            "general.architecture metadata is not a string".to_string(),
        )),
        None => Err(crate::error::VortexAtomsError::Gguf(
            "GGUF file missing general.architecture metadata".to_string(),
        )),
    }
}

/// Extract the EOS token ID from GGUF metadata.
pub fn detect_eos_token_id(content: &gguf_file::Content) -> u32 {
    content
        .metadata
        .get("tokenizer.ggml.eos_token_id")
        .and_then(|v| match v {
            gguf_file::Value::U32(id) => Some(*id),
            gguf_file::Value::I32(id) => Some(*id as u32),
            _ => None,
        })
        .unwrap_or(2) // default EOS for most models
}
