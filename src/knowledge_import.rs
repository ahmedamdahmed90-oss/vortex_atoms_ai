use std::path::Path;

use crate::error::VortexAtomsError;
use crate::llm_embed::VectorStore;
use crate::Result;

pub struct KnowledgeImporter {
    chunk_size: usize,
    chunk_overlap: usize,
}

impl KnowledgeImporter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
        }
    }

    pub fn default_config() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 64,
        }
    }

    pub fn import_text(
        &self,
        store: &mut VectorStore,
        id_prefix: &str,
        text: &str,
    ) -> Result<usize> {
        let chunks = self.chunk_text(text);
        let count = chunks.len();

        for (i, chunk) in chunks.into_iter().enumerate() {
            let id = format!("{id_prefix}_chunk_{i}");
            store.insert(id, chunk)?;
        }

        Ok(count)
    }

    pub fn import_file(&self, store: &mut VectorStore, path: &Path) -> Result<usize> {
        let content = std::fs::read_to_string(path).map_err(VortexAtomsError::Io)?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        self.import_text(store, stem, &content)
    }

    pub fn import_directory(&self, store: &mut VectorStore, dir: &Path) -> Result<usize> {
        let mut total = 0;

        let entries = std::fs::read_dir(dir).map_err(VortexAtomsError::Io)?;

        for entry in entries {
            let entry = entry.map_err(VortexAtomsError::Io)?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(
                        ext,
                        "txt"
                            | "md"
                            | "rs"
                            | "py"
                            | "js"
                            | "ts"
                            | "json"
                            | "toml"
                            | "yaml"
                            | "yml"
                            | "html"
                            | "css"
                            | "c"
                            | "cpp"
                            | "h"
                            | "java"
                            | "kt"
                            | "swift"
                            | "go"
                            | "rb"
                            | "php"
                            | "scala"
                            | "r"
                            | "m"
                            | "mm"
                            | "pl"
                            | "lua"
                            | "zig"
                            | "nim"
                            | "dart"
                            | "tex"
                            | "bib"
                            | "rst"
                            | "csv"
                            | "xml"
                            | "ini"
                            | "cfg"
                            | "conf"
                            | "sh"
                            | "bat"
                            | "ps1"
                            | "sql"
                            | "graphql"
                            | "proto"
                            | "gradle"
                            | "cmake"
                    ) {
                        total += self.import_file(store, &path).unwrap_or(0);
                    }
                }
            }
        }

        Ok(total)
    }

    fn chunk_text(&self, text: &str) -> Vec<String> {
        if text.len() <= self.chunk_size {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = (start + self.chunk_size).min(text.len());
            let chunk = text[start..end].to_string();
            chunks.push(chunk);

            start += self.chunk_size - self.chunk_overlap;
            if start >= text.len() {
                break;
            }
        }

        chunks
    }
}

pub fn build_rag_context(store: &VectorStore, query: &str, max_chunks: usize) -> Result<String> {
    let results = store.search(query, max_chunks)?;

    if results.is_empty() {
        return Ok(String::new());
    }

    let mut context = String::from("Relevant context:\n\n");
    for (i, result) in results.iter().enumerate() {
        context.push_str(&format!(
            "[{}]\nScore: {:.3}\n{}\n\n",
            i + 1,
            result.score,
            result.text
        ));
    }

    Ok(context)
}
