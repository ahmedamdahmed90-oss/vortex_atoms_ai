use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::VortexAtomsError;
use crate::llm_config::{model_sizes, LlmConfig};
use crate::Result;

/// Refuse to keep model files larger than this (supply-chain / disk-fill cap).
pub const MAX_MODEL_FILE_BYTES: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Serialize, Deserialize, Default)]
struct ManifestEntry {
    sha256: String,
    bytes: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct ModelManifest {
    files: HashMap<String, ManifestEntry>,
}

fn manifest_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(".vortex_manifest.json")
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .map_err(|e| VortexAtomsError::Gguf(format!("hash read failed: {e}")))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Adopt-or-verify: first sighting records the hash; later loads must match,
/// otherwise the cache is treated as tampered. Also enforces the size cap.
fn adopt_or_verify(cache_dir: &Path, file_name: &str, path: &Path) -> Result<()> {
    let meta = std::fs::metadata(path).map_err(VortexAtomsError::Io)?;
    if meta.len() > MAX_MODEL_FILE_BYTES {
        let _ = std::fs::remove_file(path);
        return Err(VortexAtomsError::Gguf(format!(
            "model file exceeds size cap of {MAX_MODEL_FILE_BYTES} bytes"
        )));
    }
    let mpath = manifest_path(cache_dir);
    let mut manifest = if mpath.exists() {
        std::fs::read_to_string(&mpath)
            .ok()
            .and_then(|c| serde_json::from_str::<ModelManifest>(&c).ok())
            .unwrap_or_default()
    } else {
        ModelManifest::default()
    };
    let digest = sha256_file(path)?;
    match manifest.files.get(file_name) {
        Some(entry) if entry.sha256 == digest => Ok(()),
        Some(_) => Err(VortexAtomsError::Gguf(
            "cached model hash mismatch — cache may be tampered, delete it to re-download"
                .to_string(),
        )),
        None => {
            manifest.files.insert(
                file_name.to_string(),
                ManifestEntry {
                    sha256: digest,
                    bytes: meta.len(),
                },
            );
            if let Ok(json) = serde_json::to_string_pretty(&manifest) {
                let _ = std::fs::write(&mpath, json);
            }
            Ok(())
        }
    }
}

pub(crate) const DEFAULT_MODEL_REPO: &str = "Qwen/Qwen2.5-0.5B-Instruct-GGUF";
pub(crate) const DEFAULT_TOKENIZER_REPO: &str = "Qwen/Qwen2.5-0.5B-Instruct";
pub(crate) const DEFAULT_MODEL_FILE: &str = "qwen2.5-0.5b-instruct-q4_k_m.gguf";
pub(crate) const DEFAULT_TOKENIZER_FILE: &str = "tokenizer.json";
const EMBED_MODEL_REPO: &str = "BAAI/bge-small-en-v1.5";
const EMBED_ONNX_FILE: &str = "onnx/model.onnx";
const EMBED_TOKENIZER_FILE: &str = "tokenizer.json";

/// Pre-configured model definitions for the economy/quality matrix.
pub struct ModelDefinition {
    pub repo: &'static str,
    pub gguf_file: &'static str,
    pub arch: &'static str,
    pub size_params: u64,
    pub quant: &'static str,
    /// Advertised context budget (`/v1/models` `max_seq_len`).
    pub max_seq_len: usize,
}

pub const MODELS_1_5B: &[ModelDefinition] = &[
    ModelDefinition {
        repo: "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
        gguf_file: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        arch: "qwen2",
        size_params: model_sizes::P1_5B,
        quant: "Q4_K_M",
        max_seq_len: 4096,
    },
    ModelDefinition {
        repo: "hugging-quants/Llama-3.2-1B-Instruct-GGUF",
        gguf_file: "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
        arch: "llama",
        size_params: model_sizes::P1_5B,
        quant: "Q4_K_M",
        max_seq_len: 4096,
    },
];

pub const MODELS_3B: &[ModelDefinition] = &[ModelDefinition {
    repo: "Qwen/Qwen2.5-3B-Instruct-GGUF",
    gguf_file: "qwen2.5-3b-instruct-q4_k_m.gguf",
    arch: "qwen2",
    size_params: model_sizes::P3B,
    quant: "Q4_K_M",
    max_seq_len: 4096,
}];

pub const MODELS_7B: &[ModelDefinition] = &[
    ModelDefinition {
        repo: "Qwen/Qwen2.5-7B-Instruct-GGUF",
        gguf_file: "qwen2.5-7b-instruct-q4_k_m.gguf",
        arch: "qwen2",
        size_params: model_sizes::P7B,
        quant: "Q4_K_M",
        max_seq_len: 4096,
    },
    ModelDefinition {
        repo: "hugging-quants/Meta-Llama-3.1-8B-Instruct-GGUF",
        gguf_file: "Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf",
        arch: "llama",
        size_params: model_sizes::P8B,
        quant: "Q4_K_M",
        max_seq_len: 4096,
    },
];

/// Economy model: SmolLM2-135M Q4_0 (~77 MB). The low-tier default for weak CPUs.
pub const MODEL_ECO: ModelDefinition = ModelDefinition {
    repo: "HuggingFaceTB/SmolLM2-135M-Instruct-GGUF",
    gguf_file: "smollm2-135m-instruct-q4_0.gguf",
    arch: "llama",
    size_params: model_sizes::P0_135B,
    quant: "Q4_0",
    max_seq_len: 2048,
};

/// Quality model: Qwen2.5-0.5B Q4_0 (the 0.5B instruct GGUF in Q4_0 packing).
pub const MODEL_Q4_0: ModelDefinition = ModelDefinition {
    repo: "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
    gguf_file: "qwen2.5-0.5b-instruct-q4_0.gguf",
    arch: "qwen2",
    size_params: model_sizes::P0_5B,
    quant: "Q4_0",
    max_seq_len: 4096,
};

/// Economy/quality matrix used by the routing allowlist and `/v1/models`.
pub const MODEL_ECO_SLOT: &[ModelDefinition] = &[MODEL_ECO, MODEL_Q4_0];

/// Resolve a named matrix entry (`eco`, `q4_0`) or `None` for unknown names.
pub fn resolve_matrix_def(name: &str) -> Option<&'static ModelDefinition> {
    match name {
        "eco" => Some(&MODEL_ECO),
        "q4_0" => Some(&MODEL_Q4_0),
        _ => None,
    }
}

pub struct ModelDownloader {
    cache_dir: PathBuf,
}

impl ModelDownloader {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
        }
    }

    pub fn default_cache_dir() -> PathBuf {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(appdata)
                .join("vortex_atoms_ai")
                .join("models")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".vortex_atoms_ai").join("models")
        } else {
            PathBuf::from(".").join("models")
        }
    }

    pub fn ensure_model(
        &self,
        repo: Option<&str>,
        model_file: Option<&str>,
        tokenizer_file: Option<&str>,
    ) -> Result<LlmConfig> {
        let model_repo = repo.unwrap_or(DEFAULT_MODEL_REPO);
        let model_name = model_file.unwrap_or(DEFAULT_MODEL_FILE);
        let tok_name = tokenizer_file.unwrap_or(DEFAULT_TOKENIZER_FILE);

        // Filenames join the cache dir: reject traversal/absolute names so a
        // malicious request cannot write outside the cache. Repo ids are
        // restricted to the conservative `org/name` alphabet.
        crate::security::check_model_file_name(model_name).map_err(VortexAtomsError::Gguf)?;
        crate::security::check_model_file_name(tok_name).map_err(VortexAtomsError::Gguf)?;
        crate::security::check_hf_repo(model_repo).map_err(VortexAtomsError::Gguf)?;
        if let Some(r) = repo {
            if r != model_repo {
                crate::security::check_hf_repo(r).map_err(VortexAtomsError::Gguf)?;
            }
        }

        std::fs::create_dir_all(&self.cache_dir).map_err(VortexAtomsError::Io)?;

        let model_path = self.cache_dir.join(model_name);
        let tokenizer_path = self.cache_dir.join(tok_name);

        if !model_path.exists() {
            // Refuse to start a ~0.5 GB download onto a nearly-full volume.
            // (Warm caches boot regardless of free space.)
            crate::security::check_disk_space(&self.cache_dir, 1024 * 1024 * 1024)?;
            println!("[VortexDownload] Model not found locally. Downloading from HuggingFace...");
            println!("[VortexDownload] Repo: {model_repo}, File: {model_name}");
            download_file(model_repo, model_name, &model_path)?;
            println!(
                "[VortexDownload] Model downloaded to: {}",
                model_path.display()
            );
        } else {
            println!("[VortexDownload] Model found: {}", model_path.display());
        }
        // Adopt-or-verify integrity (plus size cap) on every load.
        adopt_or_verify(&self.cache_dir, model_name, &model_path)?;

        if !tokenizer_path.exists() {
            println!("[VortexDownload] Tokenizer not found locally. Downloading...");

            if download_file(model_repo, tok_name, &tokenizer_path).is_err() {
                println!("[VortexDownload] Tokenizer not in GGUF repo, trying main model repo...");
                let tok_repo = repo.unwrap_or(DEFAULT_TOKENIZER_REPO);
                crate::security::check_hf_repo(tok_repo).map_err(VortexAtomsError::Gguf)?;
                download_file(tok_repo, tok_name, &tokenizer_path)?;
            }

            println!(
                "[VortexDownload] Tokenizer downloaded to: {}",
                tokenizer_path.display()
            );
        } else {
            println!(
                "[VortexDownload] Tokenizer found: {}",
                tokenizer_path.display()
            );
        }
        adopt_or_verify(&self.cache_dir, tok_name, &tokenizer_path)?;

        Ok(LlmConfig {
            model_path,
            tokenizer_path,
            ..LlmConfig::default()
        })
    }

    pub fn ensure_model_by_def(&self, def: &ModelDefinition) -> Result<LlmConfig> {
        let model_file = def.gguf_file;
        let tok_name = DEFAULT_TOKENIZER_FILE;

        // Same filename/repo guards as `ensure_model`: a matrix entry must never
        // escape the cache directory or reference an untrusted repo.
        crate::security::check_model_file_name(model_file).map_err(VortexAtomsError::Gguf)?;
        crate::security::check_model_file_name(tok_name).map_err(VortexAtomsError::Gguf)?;
        crate::security::check_hf_repo(def.repo).map_err(VortexAtomsError::Gguf)?;

        std::fs::create_dir_all(&self.cache_dir).map_err(VortexAtomsError::Io)?;

        let model_path = self.cache_dir.join(model_file);
        let tokenizer_path = self.cache_dir.join(tok_name);

        if !model_path.exists() {
            crate::security::check_disk_space(&self.cache_dir, 1024 * 1024 * 1024)?;
            println!("[VortexDownload] Model not found locally. Downloading from HuggingFace...");
            println!("[VortexDownload] Repo: {}, File: {}", def.repo, model_file);
            download_file(def.repo, model_file, &model_path)?;
            println!(
                "[VortexDownload] Model downloaded to: {}",
                model_path.display()
            );
        } else {
            println!("[VortexDownload] Model found: {}", model_path.display());
        }
        // Adopt-or-verify integrity (plus size cap) on every load, exactly as
        // the default downloader does — matrix swaps get the same SHA-256
        // manifest guarantee as the bootstrap model.
        adopt_or_verify(&self.cache_dir, model_file, &model_path)?;

        if !tokenizer_path.exists() {
            println!("[VortexDownload] Tokenizer not found. Downloading from main repo...");
            download_file(def.repo, tok_name, &tokenizer_path)?;
            println!(
                "[VortexDownload] Tokenizer downloaded to: {}",
                tokenizer_path.display()
            );
        }
        adopt_or_verify(&self.cache_dir, tok_name, &tokenizer_path)?;

        Ok(LlmConfig {
            model_path,
            tokenizer_path,
            model_size_params: def.size_params,
            model_quantization: def.quant.to_string(),
            max_seq_len: def.max_seq_len,
            ..LlmConfig::default()
        })
    }

    pub fn ensure_embed_model(&self) -> Result<std::path::PathBuf> {
        let embed_dir = self.cache_dir.join("bge-small");
        std::fs::create_dir_all(&embed_dir).map_err(VortexAtomsError::Io)?;

        let onnx_path = embed_dir.join("model.onnx");
        let tokenizer_path = embed_dir.join("tokenizer.json");

        if !onnx_path.exists() {
            println!("[VortexDownload] Embedding model not found. Downloading from HuggingFace...");
            download_file(EMBED_MODEL_REPO, EMBED_ONNX_FILE, &onnx_path)?;
            println!(
                "[VortexDownload] Embedding ONNX model downloaded to: {}",
                onnx_path.display()
            );
        }

        if !tokenizer_path.exists() {
            download_file(EMBED_MODEL_REPO, EMBED_TOKENIZER_FILE, &tokenizer_path)?;
            println!(
                "[VortexDownload] Embedding tokenizer downloaded to: {}",
                tokenizer_path.display()
            );
        }

        Ok(embed_dir)
    }
}

fn download_file(repo: &str, filename: &str, dest: &std::path::Path) -> Result<()> {
    let api = hf_hub::api::sync::Api::new()
        .map_err(|e| VortexAtomsError::Gguf(format!("HF Hub API init failed: {e}")))?;

    let repo_api = api.repo(hf_hub::Repo::new(repo.to_string(), hf_hub::RepoType::Model));

    let path = repo_api.get(filename).map_err(|e| {
        VortexAtomsError::Gguf(format!("HF Hub download failed for {filename}: {e}"))
    })?;

    std::fs::copy(&path, dest).map_err(VortexAtomsError::Io)?;

    Ok(())
}
