use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::VortexAtomsError;
use crate::llm_embed::VectorStore;
use crate::Result;

/// Maximum single-file size accepted by directory import (16 MiB).
/// Larger files are reported as skipped, never read into memory.
pub const MAX_IMPORT_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// Structured result for directory imports: every file is accounted for,
/// so partial failures can never vanish silently.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportReport {
    pub succeeded_files: usize,
    pub failed_files: usize,
    pub skipped_files: usize,
    pub chunks: usize,
    pub errors: Vec<String>,
}

impl ImportReport {
    pub fn has_failures(&self) -> bool {
        self.failed_files > 0
    }

    pub fn summary(&self) -> String {
        format!(
            "{} file(s) imported ({} chunks), {} failed, {} skipped",
            self.succeeded_files, self.chunks, self.failed_files, self.skipped_files
        )
    }
}

/// Text extensions accepted by directory import. Binary formats (`.tcz`,
/// `.bin`) belong to the fragment registry, not the text importer.
fn is_supported_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some(
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
        )
    )
}

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

    /// Import a directory, returning the total chunk count.
    ///
    /// Kept for backwards compatibility; per-file failures are counted but
    /// not surfaced. Prefer [`import_directory_report`](Self::import_directory_report)
    /// when failure observability matters.
    pub fn import_directory(&self, store: &mut VectorStore, dir: &Path) -> Result<usize> {
        Ok(self.import_directory_report(store, dir)?.chunks)
    }

    /// Import a directory with a full per-file accounting.
    ///
    /// Safety rules (defense in depth behind the API path sandbox):
    /// - symlinks are never followed (skipped + counted);
    /// - files larger than [`MAX_IMPORT_FILE_BYTES`] are skipped unread;
    /// - unsupported extensions are skipped (previously silent, now counted);
    /// - unreadable entries and per-file import errors are recorded as
    ///   failures with file-name-only messages (no full paths leak).
    pub fn import_directory_report(
        &self,
        store: &mut VectorStore,
        dir: &Path,
    ) -> Result<ImportReport> {
        let mut report = ImportReport::default();

        let entries = std::fs::read_dir(dir).map_err(VortexAtomsError::Io)?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    report.failed_files += 1;
                    report
                        .errors
                        .push(format!("unreadable directory entry: {e}"));
                    continue;
                }
            };
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "?".to_string());

            // Never follow symlinks: an attacker-controlled directory could
            // otherwise redirect reads outside the sandboxed tree.
            if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
                report.skipped_files += 1;
                report.errors.push(format!("skipped symlink: {name}"));
                continue;
            }

            if !path.is_file() {
                report.skipped_files += 1;
                continue;
            }

            if !is_supported_extension(&path) {
                report.skipped_files += 1;
                continue;
            }

            match std::fs::metadata(&path) {
                Ok(meta) if meta.len() > MAX_IMPORT_FILE_BYTES => {
                    report.skipped_files += 1;
                    report.errors.push(format!(
                        "skipped oversized file ({} bytes): {name}",
                        meta.len()
                    ));
                    continue;
                }
                Err(e) => {
                    report.failed_files += 1;
                    report.errors.push(format!("cannot stat {name}: {e}"));
                    continue;
                }
                _ => {}
            }

            match self.import_file(store, &path) {
                Ok(n) => {
                    report.succeeded_files += 1;
                    report.chunks += n;
                }
                Err(e) => {
                    report.failed_files += 1;
                    report
                        .errors
                        .push(format!("failed to import {name}: {}", e.public_message()));
                }
            }
        }

        Ok(report)
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
    fn report_valid_and_unsupported_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello world ".repeat(50)).unwrap();
        std::fs::write(dir.path().join("b.md"), "# title\n\nbody ".repeat(50)).unwrap();
        std::fs::write(dir.path().join("c.exe"), b"binary").unwrap();

        let mut store = VectorStore::new(16);
        let report = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        assert_eq!(report.succeeded_files, 2);
        assert_eq!(report.skipped_files, 1); // c.exe
        assert_eq!(report.failed_files, 0);
        assert!(report.chunks > 0);
        assert!(!report.has_failures());
        assert!(report.summary().contains("2 file(s) imported"));
    }

    #[test]
    fn report_empty_and_unicode_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("empty.txt"), "").unwrap();
        std::fs::write(dir.path().join("arabic.txt"), "الذكاء الاصطناعي ".repeat(60)).unwrap();

        let mut store = VectorStore::new(16);
        let report = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        assert_eq!(report.succeeded_files, 2);
        assert_eq!(report.failed_files, 0);
    }

    #[test]
    fn report_partial_failure_is_observable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("good.txt"), "valid content ".repeat(50)).unwrap();
        // Invalid UTF-8: read_to_string fails -> failed_files, not silent zero.
        std::fs::write(dir.path().join("bad.txt"), b"\xff\xfe\x00invalid").unwrap();

        let mut store = VectorStore::new(16);
        let report = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        assert_eq!(report.succeeded_files, 1);
        assert_eq!(report.failed_files, 1);
        assert_eq!(report.errors.len(), 1);
        assert!(report.has_failures());
    }

    #[test]
    fn report_duplicate_import_counts_both() {
        // Documents current no-dedup behavior: re-import is observable
        // through the report rather than silently merged.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("dup.txt"), "same content ".repeat(50)).unwrap();

        let mut store = VectorStore::new(16);
        let first = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        let second = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        assert_eq!(first.chunks, second.chunks);
        assert_eq!(store.len(), first.chunks * 2);
    }

    #[test]
    fn report_malformed_inputs() {
        let mut store = VectorStore::new(16);
        // Not a directory -> Err (propagated, not swallowed).
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(importer()
            .import_directory_report(&mut store, &file)
            .is_err());
        assert!(importer()
            .import_directory_report(&mut store, &dir.path().join("missing"))
            .is_err());
    }

    #[test]
    fn report_skips_oversized_files() {
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big.txt");
        let chunk = vec![b'a'; 1024 * 1024];
        let f = std::fs::File::create(&big).unwrap();
        use std::io::Write;
        let mut f = f;
        for _ in 0..(MAX_IMPORT_FILE_BYTES / 1024 / 1024 + 1) {
            f.write_all(&chunk).unwrap();
        }
        drop(f);

        let mut store = VectorStore::new(16);
        let report = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        assert_eq!(report.succeeded_files, 0);
        assert_eq!(report.skipped_files, 1);
        assert!(store.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn report_never_follows_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.txt");
        std::fs::write(&target, "secret ".repeat(50)).unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();

        let mut store = VectorStore::new(16);
        let report = importer()
            .import_directory_report(&mut store, dir.path())
            .unwrap();
        assert_eq!(report.succeeded_files, 1);
        assert_eq!(report.skipped_files, 1);
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
