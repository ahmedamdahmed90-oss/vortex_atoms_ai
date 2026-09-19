use crate::llm_embed::EmbeddingModel;
use crate::Result;
use std::path::Path;

pub type EmbeddingVec = Vec<f32>;
pub const NEURAL_DIMENSIONS: usize = 384;

pub trait NeuralEmbedder: Send {
    fn embed(&mut self, text: &str) -> Result<EmbeddingVec>;
    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<EmbeddingVec>>;
    fn dimensions(&self) -> usize;
}

pub enum UnifiedEmbedder {
    Hash(EmbeddingModel),
    #[cfg(feature = "neural-embed")]
    Onnx(OrtEmbedder),
}

impl NeuralEmbedder for UnifiedEmbedder {
    fn embed(&mut self, text: &str) -> Result<EmbeddingVec> {
        match self {
            Self::Hash(m) => m.embed(text),
            #[cfg(feature = "neural-embed")]
            Self::Onnx(m) => m.embed(text),
        }
    }

    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<EmbeddingVec>> {
        match self {
            Self::Hash(m) => m.batch_embed(texts),
            #[cfg(feature = "neural-embed")]
            Self::Onnx(m) => m.embed_batch(texts),
        }
    }

    fn dimensions(&self) -> usize {
        match self {
            Self::Hash(m) => m.dimensions(),
            #[cfg(feature = "neural-embed")]
            Self::Onnx(m) => m.dimensions(),
        }
    }
}

#[allow(unused_variables)]
pub fn create_embedder(model_dir: Option<&Path>) -> Box<dyn NeuralEmbedder> {
    #[cfg(feature = "neural-embed")]
    {
        if let Some(dir) = model_dir {
            let onnx_path = dir.join("model.onnx");
            if onnx_path.exists() {
                match OrtEmbedder::load(&onnx_path) {
                    Ok(embedder) => {
                        println!(
                            "[VortexEmbed] Neural embedder loaded ({})",
                            onnx_path.display()
                        );
                        return Box::new(embedder);
                    }
                    Err(e) => {
                        eprintln!(
                            "[VortexEmbed] Failed to load ONNX model: {e}, falling back to hash"
                        );
                    }
                }
            } else {
                println!(
                    "[VortexEmbed] ONNX model not found at {}, using hash embedder",
                    onnx_path.display()
                );
            }
        }
    }

    Box::new(EmbeddingModel::new(NEURAL_DIMENSIONS))
}

#[cfg(feature = "neural-embed")]
pub struct OrtEmbedder {
    session: ort::session::Session,
    tokenizer: tokenizers::Tokenizer,
    dim: usize,
}

#[cfg(feature = "neural-embed")]
impl NeuralEmbedder for OrtEmbedder {
    fn embed(&mut self, text: &str) -> Result<EmbeddingVec> {
        self.embed(text)
    }

    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<EmbeddingVec>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimensions(&self) -> usize {
        self.dim
    }
}

#[cfg(feature = "neural-embed")]
impl OrtEmbedder {
    pub fn load(model_path: &Path) -> Result<Self> {
        use ort::session::Session;

        let session = Session::builder()
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT builder: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT load: {e}")))?;

        let tokenizer_path = model_path
            .parent()
            .map(|p| p.join("tokenizer.json"))
            .unwrap_or_else(|| Path::new("tokenizer.json").to_path_buf());

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(crate::error::VortexAtomsError::Tokenizers)?;

        Ok(Self {
            session,
            tokenizer,
            dim: NEURAL_DIMENSIONS,
        })
    }

    pub fn embed(&mut self, text: &str) -> Result<EmbeddingVec> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(crate::error::VortexAtomsError::Tokenizers)?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let seq_len = ids.len();

        let shape = vec![1_i64, seq_len as i64];

        let input_ids = ort::value::Tensor::from_array((shape.clone(), ids))
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT tensor: {e}")))?;
        let mask = ort::value::Tensor::from_array((shape.clone(), attention_mask.clone()))
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT tensor: {e}")))?;
        let type_ids = ort::value::Tensor::from_array((shape, vec![0_i64; seq_len]))
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT tensor: {e}")))?;

        let outputs = self
            .session
            .run(ort::inputs![input_ids, mask, type_ids])
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT run: {e}")))?;

        let (output_shape, output_data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("ORT output: {e}")))?;

        let dim = output_shape[output_shape.len() - 1] as usize;
        let seq = output_shape[output_shape.len() - 2] as usize;

        let mut embedding = vec![0.0f32; dim];
        let mask_f32: Vec<f32> = attention_mask.iter().map(|&m| m as f32).collect();
        let mut mask_sum = 0.0f32;

        for token_idx in 0..seq {
            let m = mask_f32[token_idx];
            if m > 0.0 {
                for d in 0..dim {
                    embedding[d] += output_data[token_idx * dim + d] * m;
                }
                mask_sum += m;
            }
        }

        if mask_sum > 0.0 {
            for d in 0..dim {
                embedding[d] /= mask_sum;
            }
        }

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }

        Ok(embedding)
    }

    pub fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<EmbeddingVec>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    pub fn dimensions(&self) -> usize {
        self.dim
    }
}
