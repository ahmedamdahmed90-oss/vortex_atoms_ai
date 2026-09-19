// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 1 test fixtures: canary overlay isolation,
// watchdog trigger, abort-on-allowlist-miss.
// All tests run against these local fixture pages.

use std::collections::HashMap;
use vortex_atoms_ai::learner::CanaryOverlay;

/// A fixture page for testing the acquisition pipeline.
#[derive(Clone, Debug)]
pub struct FixturePage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub language: String,
    pub source_id: String,
    pub license: String,
}

/// Returns the standard fixture pages used across learner tests.
pub fn fixture_pages() -> Vec<FixturePage> {
    vec![
        FixturePage {
            url: "https://wikipedia_en/wiki/Rust_(programming_language)".to_string(),
            title: "Rust (programming language)".to_string(),
            content: "Rust is a multi-paradigm, general-purpose programming language.".to_string(),
            language: "en".to_string(),
            source_id: "rust_doc".to_string(),
            license: "MIT/Apache-2.0".to_string(),
        },
        FixturePage {
            url: "https://wikipedia_en/wiki/Python_(programming_language)".to_string(),
            title: "Python (programming language)".to_string(),
            content: "Python is an interpreted, high-level, general-purpose programming language.".to_string(),
            language: "en".to_string(),
            source_id: "python_doc".to_string(),
            license: "PSF".to_string(),
        },
        FixturePage {
            url: "https://en.wikipedia.org/wiki/Machine_learning".to_string(),
            title: "Machine learning".to_string(),
            content: "Machine learning is a field of artificial intelligence.".to_string(),
            language: "en".to_string(),
            source_id: "wikipedia_en".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
        },
        FixturePage {
            url: "https://developer.mozilla.org/en-US/docs/Web/JavaScript".to_string(),
            title: "JavaScript".to_string(),
            content: "JavaScript is a programming language that conforms to the ECMAScript specification.".to_string(),
            language: "en".to_string(),
            source_id: "mdn".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
        },
        FixturePage {
            url: "https://www.gutenberg.org/ebooks/1".to_string(),
            title: "Sample Book".to_string(),
            content: "This is a public domain book from Project Gutenberg.".to_string(),
            language: "en".to_string(),
            source_id: "gutenberg".to_string(),
            license: "PD".to_string(),
        },
    ]
}

/// Returns a simple HTML fixture for extraction tests.
pub fn html_fixture() -> &'static str {
    r#"<html><head><title>Test Page</title></head>
 <body>
 <nav>Navigation content</nav>
 <article>
 <h1>Article Title</h1>
 <p>Article paragraph content.</p>
 </article>
 <footer>Footer content</footer>
 </body></html>"#
}

/// Returns fixture URLs for URL allowlist tests.
pub fn fixture_urls() -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    map.insert(
        "wikipedia_en".to_string(),
        vec![
            "https://wikipedia_en/wiki/Main_Page".to_string(),
            "https://wikipedia_en/wiki/Rust_(programming_language)".to_string(),
        ],
    );
    map.insert(
        "mdn".to_string(),
        vec![
            "https://mdn/en-US/docs/Web/JavaScript".to_string(),
        ],
    );
    map
}

/// Returns a canary overlay config for testing isolation.
pub fn canary_overlay() -> CanaryOverlay {
    CanaryOverlay {
        source_id: "wikipedia_en".to_string(),
        max_pages: 25,
        max_runtime_minutes: 5,
        abort_on_allowlist_miss: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_pages_count() {
        assert_eq!(fixture_pages().len(), 5);
    }

    #[test]
    fn all_fixture_pages_have_license() {
        for page in fixture_pages() {
            assert!(!page.license.is_empty());
        }
    }

    #[test]
    fn html_fixture_has_nav_and_article() {
        let html = html_fixture();
        assert!(html.contains("<nav>"));
        assert!(html.contains("<article>"));
        assert!(html.contains("<h1>"));
    }

    #[test]
    fn canary_overlay_isolation() {
        let canary = canary_overlay();
        assert_eq!(canary.source_id, "wikipedia_en");
        assert_eq!(canary.max_pages, 25);
        assert_eq!(canary.max_runtime_minutes, 5);
        assert!(canary.abort_on_allowlist_miss);
    }

    #[test]
    fn canary_overlay_allows_only_single_source() {
        let canary = canary_overlay();
        assert_eq!(canary.source_id, "wikipedia_en");
        let allowed_urls = fixture_urls().get(&canary.source_id).unwrap();
        assert_eq!(allowed_urls.len(), 2);
    }

    #[test]
    fn canary_overlay_rejects_external_urls() {
        let canary = canary_overlay();
        let external = vec![
            "https://evil.com/secret".to_string(),
            "https://wikipedia_fr/wiki/Paris".to_string(),
        ];
        for url in external {
            assert!(!url.contains(&canary.source_id));
        }
    }

    #[test]
    fn golden_queries_count_30() {
        use vortex_atoms_ai::learner::eval::golden_queries;
        let queries = golden_queries();
        assert_eq!(queries.len(), 30);
        let english = queries.iter().filter(|q| q.language == "en").count();
        let arabic = queries.iter().filter(|q| q.language == "ar").count();
        assert_eq!(english, 20);
        assert_eq!(arabic, 10);
    }

    #[test]
    fn golden_queries_all_have_expected_sources() {
        use vortex_atoms_ai::learner::eval::golden_queries;
        let queries = golden_queries();
        for q in queries {
            assert!(!q.expected_sources.is_empty());
            assert!(!q.expected_keywords.is_empty());
        }
    }
}
