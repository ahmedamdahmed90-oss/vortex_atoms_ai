// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 3: Golden retrieval set for eval harness.
// 20 English queries × 6 seed domains + 10 Arabic queries.
// Each entry has expected source URLs and keywords for retrieval validation.
// Fixtures-only for CI — no external network calls.

use serde::{Deserialize, Serialize};

/// A single golden query with expected retrieval targets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoldenQuery {
    pub id: String,
    pub query: String,
    pub language: String,
    pub expected_sources: Vec<String>,
    pub expected_keywords: Vec<String>,
    pub domain: String,
}

/// Returns the full golden retrieval set: 20 English + 10 Arabic queries.
pub fn golden_queries() -> Vec<GoldenQuery> {
    vec![
        // === English queries — Wikipedia domain ===
        GoldenQuery {
            id: "w01".to_string(),
            query: "Rust programming language memory safety".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "memory".to_string(), "safety".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "w02".to_string(),
            query: "Machine learning algorithms overview".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://en.wikipedia.org/wiki/Machine_learning".to_string()],
            expected_keywords: vec!["Machine".to_string(), "learning".to_string(), "algorithms".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "w03".to_string(),
            query: "Python data structures list tuple dict".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "data".to_string(), "structures".to_string()],
            domain: "wikipedia".to_string(),
        },
        // === English queries — MDN domain ===
        GoldenQuery {
            id: "m01".to_string(),
            query: "JavaScript async await promises".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string()],
            expected_keywords: vec!["JavaScript".to_string(), "async".to_string(), "await".to_string()],
            domain: "mdn".to_string(),
        },
        GoldenQuery {
            id: "m02".to_string(),
            query: "CSS flexbox layout guide".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string()],
            expected_keywords: vec!["CSS".to_string(), "flexbox".to_string(), "layout".to_string()],
            domain: "mdn".to_string(),
        },
        // === English queries — Rust Doc domain ===
        GoldenQuery {
            id: "r01".to_string(),
            query: "Rust standard library HashMap".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "HashMap".to_string(), "standard".to_string()],
            domain: "rust_doc".to_string(),
        },
        GoldenQuery {
            id: "r02".to_string(),
            query: "Rust ownership borrowing rules".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "ownership".to_string(), "borrowing".to_string()],
            domain: "rust_doc".to_string(),
        },
        // === English queries — Python Doc domain ===
        GoldenQuery {
            id: "p01".to_string(),
            query: "Python pip package manager install".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "pip".to_string(), "package".to_string()],
            domain: "python_doc".to_string(),
        },
        GoldenQuery {
            id: "p02".to_string(),
            query: "Python virtual environment setup".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "virtual".to_string(), "environment".to_string()],
            domain: "python_doc".to_string(),
        },
        // === English queries — Gutenberg domain ===
        GoldenQuery {
            id: "g01".to_string(),
            query: "Public domain book literature classics".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://www.gutenberg.org/ebooks/1".to_string()],
            expected_keywords: vec!["public".to_string(), "domain".to_string(), "book".to_string()],
            domain: "gutenberg".to_string(),
        },
        // === Additional English queries for coverage ===
        GoldenQuery {
            id: "e01".to_string(),
            query: "Web development HTML CSS JavaScript basics".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string()],
            expected_keywords: vec!["HTML".to_string(), "CSS".to_string(), "JavaScript".to_string()],
            domain: "mdn".to_string(),
        },
        GoldenQuery {
            id: "e02".to_string(),
            query: "System programming language performance".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["system".to_string(), "programming".to_string(), "performance".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "e03".to_string(),
            query: "Deep learning neural networks overview".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://en.wikipedia.org/wiki/Machine_learning".to_string()],
            expected_keywords: vec!["deep".to_string(), "learning".to_string(), "neural".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "e04".to_string(),
            query: "Python Django web framework tutorial".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "Django".to_string(), "web".to_string()],
            domain: "python_doc".to_string(),
        },
        GoldenQuery {
            id: "e05".to_string(),
            query: "Rust cargo build release profile".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "cargo".to_string(), "build".to_string()],
            domain: "rust_doc".to_string(),
        },
        GoldenQuery {
            id: "e06".to_string(),
            query: "JavaScript DOM manipulation events".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string()],
            expected_keywords: vec!["JavaScript".to_string(), "DOM".to_string(), "events".to_string()],
            domain: "mdn".to_string(),
        },
        GoldenQuery {
            id: "e07".to_string(),
            query: "Classic literature adventure novels public domain".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://www.gutenberg.org/ebooks/1".to_string()],
            expected_keywords: vec!["classic".to_string(), "literature".to_string(), "adventure".to_string()],
            domain: "gutenberg".to_string(),
        },
        GoldenQuery {
            id: "e08".to_string(),
            query: "Wikipedia search engine optimization algorithms".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://en.wikipedia.org/wiki/Machine_learning".to_string()],
            expected_keywords: vec!["Wikipedia".to_string(), "search".to_string(), "algorithms".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "e09".to_string(),
            query: "Python data science numpy pandas matplotlib".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "data".to_string(), "science".to_string()],
            domain: "python_doc".to_string(),
        },
        GoldenQuery {
            id: "e10".to_string(),
            query: "Rust unsafe keyword FFI foreign function interface".to_string(),
            language: "en".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "unsafe".to_string(), "FFI".to_string()],
            domain: "rust_doc".to_string(),
        },
        // === Arabic queries ===
        GoldenQuery {
            id: "a01".to_string(),
            query: "تعلم الآلة خوارزميات الذكاء الاصطناعي".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://en.wikipedia.org/wiki/Machine_learning".to_string()],
            expected_keywords: vec!["Machine".to_string(), "learning".to_string(), "learning".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "a02".to_string(),
            query: "لغة البرمجة بايثون هياكل البيانات".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "data".to_string(), "structures".to_string()],
            domain: "python_doc".to_string(),
        },
        GoldenQuery {
            id: "a03".to_string(),
            query: "JavaScript غير متزامن وعد ووعود".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string()],
            expected_keywords: vec!["JavaScript".to_string(), "async".to_string(), "await".to_string()],
            domain: "mdn".to_string(),
        },
        GoldenQuery {
            id: "a04".to_string(),
            query: "Rust ملكية واستعارة قواعد اللغة".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "ownership".to_string(), "borrowing".to_string()],
            domain: "rust_doc".to_string(),
        },
        GoldenQuery {
            id: "a05".to_string(),
            query: "تثبيت حزم بايثون بايب".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "pip".to_string(), "package".to_string()],
            domain: "python_doc".to_string(),
        },
        GoldenQuery {
            id: "a06".to_string(),
            query: "تعلّم الآلة الشبكات العصبية".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://en.wikipedia.org/wiki/Machine_learning".to_string()],
            expected_keywords: vec!["deep".to_string(), "learning".to_string(), "neural".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "a07".to_string(),
            query: "تطوير الويب HTML CSS JavaScript".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string()],
            expected_keywords: vec!["HTML".to_string(), "CSS".to_string(), "JavaScript".to_string()],
            domain: "mdn".to_string(),
        },
        GoldenQuery {
            id: "a08".to_string(),
            query: "محرك البحث ويكيبيديا تحسين الخوارزميات".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://en.wikipedia.org/wiki/Machine_learning".to_string()],
            expected_keywords: vec!["Wikipedia".to_string(), "search".to_string(), "algorithms".to_string()],
            domain: "wikipedia".to_string(),
        },
        GoldenQuery {
            id: "a09".to_string(),
            query: "علوم البيانات بايثون numpy pandas".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Python_(programming_language)".to_string()],
            expected_keywords: vec!["Python".to_string(), "data".to_string(), "science".to_string()],
            domain: "python_doc".to_string(),
        },
        GoldenQuery {
            id: "a10".to_string(),
            query: "Rust الكلمة الآمنة واجهة الدالة الأجنبية".to_string(),
            language: "ar".to_string(),
            expected_sources: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
            expected_keywords: vec!["Rust".to_string(), "unsafe".to_string(), "FFI".to_string()],
            domain: "rust_doc".to_string(),
        },
    ]
}

/// Returns a single golden query by ID for testing.
pub fn golden_query_by_id(id: &str) -> Option<GoldenQuery> {
    golden_queries().iter().find(|q| q.id == id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_queries_count() {
        let queries = golden_queries();
        assert_eq!(queries.len(), 30);
        let english = queries.iter().filter(|q| q.language == "en").count();
        let arabic = queries.iter().filter(|q| q.language == "ar").count();
        assert_eq!(english, 20);
        assert_eq!(arabic, 10);
    }

    #[test]
    fn golden_query_has_expected_sources() {
        for q in golden_queries() {
            assert!(!q.expected_sources.is_empty(), "query {} must have expected sources", q.id);
        }
    }

    #[test]
    fn golden_query_has_expected_keywords() {
        for q in golden_queries() {
            assert!(!q.expected_keywords.is_empty(), "query {} must have expected keywords", q.id);
        }
    }

    #[test]
    fn all_domains_covered() {
        let queries = golden_queries();
        let domains: std::collections::HashSet<_> = queries.iter().map(|q| q.domain.as_str()).collect();
        assert!(domains.contains("wikipedia"));
        assert!(domains.contains("mdn"));
        assert!(domains.contains("rust_doc"));
        assert!(domains.contains("python_doc"));
        assert!(domains.contains("gutenberg"));
    }

    #[test]
    fn all_languages_covered() {
        let queries = golden_queries();
        let languages: std::collections::HashSet<_> = queries.iter().map(|q| q.language.as_str()).collect();
        assert!(languages.contains("en"));
        assert!(languages.contains("ar"));
    }

    #[test]
    fn golden_query_by_id_works() {
        let q = golden_query_by_id("w01").unwrap();
        assert_eq!(q.query, "Rust programming language memory safety");
        let missing = golden_query_by_id("nonexistent");
        assert!(missing.is_none());
    }
}
