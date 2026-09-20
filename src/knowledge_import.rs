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

        // Step is saturating so `chunk_overlap >= chunk_size` (or a zero
        // chunk_size) cannot underflow `usize` or stall the loop.
        let step = self.chunk_size.saturating_sub(self.chunk_overlap).max(1);

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            // Snap `end` back to a UTF-8 char boundary so multibyte text
            // (Arabic, CJK, emoji) never panics on byte slicing.
            let mut end = (start + self.chunk_size).min(text.len());
            while end > start && !text.is_char_boundary(end) {
                end -= 1;
            }
            if end == start {
                // Degenerate window narrower than one char: extend forward
                // to the next boundary instead of emitting an empty chunk.
                end = start + 1;
                while end < text.len() && !text.is_char_boundary(end) {
                    end += 1;
                }
            }
            chunks.push(text[start..end].to_string());

            if end == text.len() {
                break;
            }
            // Advance, then snap `start` forward to a boundary as well.
            start = start.saturating_add(step);
            while start < text.len() && !text.is_char_boundary(start) {
                start += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn importer() -> KnowledgeImporter {
        KnowledgeImporter::new(32, 8)
    }

    fn assert_lossless(text: &str) {
        let chunks = importer().chunk_text(text);
        assert!(!chunks.is_empty());
        // Every chunk must be valid UTF-8 (guaranteed by String) and the
        // concatenation of chunk contents must cover the whole input.
        let joined_len: usize = chunks.iter().map(|c| c.len()).sum();
        assert!(
            joined_len >= text.len(),
            "chunking lost bytes: {joined_len} < {}",
            text.len()
        );
    }

    #[test]
    fn chunk_arabic_no_panic() {
        let text =
            "الذكاء الاصطناعي المحلي يعمل دون اتصال بالإنترنت ويحافظ على خصوصية البيانات بشكل كامل"
                .repeat(4);
        assert_lossless(&text);
    }

    #[test]
    fn chunk_mixed_arabic_english_no_panic() {
        let text = "Vortex Atoms AI نموذج محلي first يعمل offline مع دعم كامل للعربية ".repeat(6);
        assert_lossless(&text);
    }

    #[test]
    fn chunk_cjk_emoji_no_panic() {
        let text = "日本語テスト🎉混合文字列中文测试🚀 ".repeat(8);
        assert_lossless(&text);
    }

    #[test]
    fn chunk_combining_and_long_unicode_no_panic() {
        let text = "é".repeat(200) + &"مرحبا ".repeat(50);
        assert_lossless(&text);
    }

    #[test]
    fn chunk_empty_and_short() {
        assert_eq!(importer().chunk_text(""), vec![String::new()]);
        assert_eq!(importer().chunk_text("hi"), vec!["hi".to_string()]);
    }

    #[test]
    fn chunk_degenerate_configs_terminate() {
        // overlap >= size and zero size must not hang or underflow.
        let text = "السلام عليكم ورحمة الله وبركاته ".repeat(10);
        for cfg in [
            KnowledgeImporter::new(16, 16),
            KnowledgeImporter::new(16, 64),
            KnowledgeImporter::new(0, 0),
            KnowledgeImporter::new(1, 0),
        ] {
            let chunks = cfg.chunk_text(&text);
            assert!(!chunks.is_empty());
        }
    }
}
