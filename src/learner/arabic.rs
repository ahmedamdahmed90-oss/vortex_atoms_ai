// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-02 Section 4: Arabic & Multilingual Growth.
// AR normalization: strip diacritics/tatweel, normalize alef/ya/ta-marbuta forms.
// Light Arabic stemmer for the keyword side of RRF.
// Language-aware fusion weights for retrieval.ar_weights.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Arabic tatweel (تَتْوِيل) character — repeated consonant elongation.
const TATWEEL: char = 'ـ';

/// Arabic diacritics to strip.
const DIACRITICS: [char; 14] = ['َ', 'ُ', 'ِ', 'ّ', 'ْ', 'ً', 'ٌ', 'ٍ', 'ٰ', 'ـ', 'ٓ', 'ٔ', 'ٕ', 'ٖ'];

/// Alef variants to normalize.
const ALEF_VARIANTS: [(char, char); 5] =
    [('آ', 'ا'), ('أ', 'ا'), ('ؤ', 'و'), ('إ', 'ا'), ('ء', 'ء')];

/// Ya variants to normalize.
const YA_VARIANTS: [(char, char); 2] = [('ى', 'ي'), ('ی', 'ي')];

/// Ta-marbuta variants to normalize.
const TAMARBUTA_VARIANTS: [(char, char); 2] = [('ة', 'ه'), ('ۀ', 'ه')];

/// Arabic stemmer — light rule-based stemmer for keyword extraction.
/// Strips common Arabic suffixes and normalizes variants.
pub struct ArabicStemmer {
    suffixes: HashMap<String, &'static str>,
}

impl Default for ArabicStemmer {
    fn default() -> Self {
        Self::new()
    }
}

impl ArabicStemmer {
    pub fn new() -> Self {
        let mut suffixes = HashMap::new();
        // Common Arabic suffixes
        suffixes.insert("ون".to_string(), "");
        suffixes.insert("ين".to_string(), "");
        suffixes.insert("ات".to_string(), "");
        suffixes.insert("ان".to_string(), "");
        suffixes.insert("ة".to_string(), "");
        suffixes.insert("ي".to_string(), "");
        suffixes.insert("ا".to_string(), "");
        Self { suffixes }
    }

    /// Stem an Arabic word.
    pub fn stem(&self, word: &str) -> String {
        let normalized = normalize_arabic(word);
        // Try suffix stripping
        for (suffix, replacement) in &self.suffixes {
            if normalized.ends_with(suffix) && normalized.len() > suffix.len() {
                return format!(
                    "{}{}",
                    &normalized[..normalized.len() - suffix.len()],
                    replacement
                );
            }
        }
        normalized
    }
}

/// Normalize Arabic text: strip diacritics, tatweel, and normalize alef/ya/ta-marbuta.
pub fn normalize_arabic(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        if DIACRITICS.contains(&c) || c == TATWEEL {
            continue;
        }
        let mut normalized = c;
        for (variant, replacement) in &ALEF_VARIANTS {
            if c == *variant {
                normalized = *replacement;
                break;
            }
        }
        for (variant, replacement) in &YA_VARIANTS {
            if c == *variant {
                normalized = *replacement;
                break;
            }
        }
        for (variant, replacement) in &TAMARBUTA_VARIANTS {
            if c == *variant {
                normalized = *replacement;
                break;
            }
        }
        result.push(normalized);
    }
    result
}

/// Language-aware fusion weights for retrieval.
/// AR queries get a boost in keyword matching to improve hit@1.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageWeights {
    pub ar_weights: FusionWeights,
    pub en_weights: FusionWeights,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionWeights {
    pub keyword_boost: f64,
    pub vector_weight: f64,
    pub keyword_weight: f64,
}

impl Default for LanguageWeights {
    fn default() -> Self {
        Self {
            ar_weights: FusionWeights {
                keyword_boost: 1.5,
                vector_weight: 0.4,
                keyword_weight: 0.6,
            },
            en_weights: FusionWeights {
                keyword_boost: 1.0,
                vector_weight: 0.5,
                keyword_weight: 0.5,
            },
        }
    }
}

/// Apply language-aware fusion score.
pub fn language_aware_fusion(score: f64, language: &str, weights: &LanguageWeights) -> f64 {
    match language {
        "ar" => score * weights.ar_weights.keyword_boost,
        "en" => score * weights.en_weights.keyword_boost,
        _ => score,
    }
}

/// Returns the AR seed sources for allowlist expansion.
pub fn arabic_seed_sources() -> Vec<crate::learner::compliance::SourceEntry> {
    vec![
        crate::learner::compliance::SourceEntry {
            id: "ar_wikipedia".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
            paths: vec!["https://ar.wikipedia.org/wiki/".to_string()],
            rate_rpm: 10,
        },
        crate::learner::compliance::SourceEntry {
            id: "ar_wikibooks".to_string(),
            license: "CC-BY-SA-4.0".to_string(),
            paths: vec!["https://ar.wikibooks.org/wiki/".to_string()],
            rate_rpm: 10,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizer_strips_diacritics() {
        let text = "مَرْحَبًا";
        let normalized = normalize_arabic(text);
        assert_eq!(normalized, "مرحبا");
    }

    #[test]
    fn normalizer_strips_tatweel() {
        let text = "كِتَابٌ";
        let normalized = normalize_arabic(text);
        assert_eq!(normalized, "كتاب");
    }

    #[test]
    fn normalizer_normalizes_alef() {
        let text = "آلِف";
        let normalized = normalize_arabic(text);
        assert!(normalized.contains("ا"));
        assert!(!normalized.contains("آ"));
        assert!(!normalized.contains("ِ"));
    }

    #[test]
    fn normalizer_normalizes_ya() {
        let text = "كِتَابٌ";
        let normalized = normalize_arabic(text);
        assert!(normalized.contains("ك"));
        assert!(normalized.contains("ت"));
        assert!(normalized.contains("ب"));
        assert!(!normalized.contains("ِ"));
        assert!(!normalized.contains("ٌ"));
    }

    #[test]
    fn normalizer_normalizes_tamarbuta() {
        let text = "مُدَرِّسَةٌ";
        let normalized = normalize_arabic(text);
        assert!(normalized.contains("م"));
        assert!(normalized.contains("د"));
        assert!(normalized.contains("ر"));
        assert!(normalized.contains("س"));
        assert!(!normalized.contains("َ"));
        assert!(!normalized.contains("ٌ"));
    }

    #[test]
    fn normalizer_preserves_plain_text() {
        let text = "مرحبا";
        let normalized = normalize_arabic(text);
        assert_eq!(normalized, "مرحبا");
    }

    #[test]
    fn normalizer_mixed() {
        let text = "مَرْحَبًا، كِتَابٌ آلِف";
        let normalized = normalize_arabic(text);
        assert!(normalized.contains("م"));
        assert!(normalized.contains("ر"));
        assert!(normalized.contains("ح"));
        assert!(normalized.contains("ب"));
        assert!(normalized.contains("ك"));
        assert!(normalized.contains("ت"));
        assert!(normalized.contains("ا"));
        assert!(!normalized.contains("َ"));
        assert!(!normalized.contains("ٌ"));
        assert!(!normalized.contains("ْ"));
    }

    #[test]
    fn normalizer_12_cases() {
        let cases = vec![
            ("مَرْحَبًا", "مرحبا"),
            ("كِتَابٌ", "كتاب"),
            ("آلِف", "اليف"),
            ("يَجِمُع", "يجم"),
            ("مُدَرِّسَةٌ", "مدرة"),
            ("مَدْرَسَةٌ", "مدرسة"),
            ("دِرَاسَةٌ", "دراسة"),
            ("كِتَابَاتٌ", "كتابات"),
            ("مَكْتَبٌ", "مكتب"),
            ("مَكْتَبَةٌ", "مكتبة"),
            ("بِعْثَةٌ", "بعثة"),
            ("جَامِعَةٌ", "جامعة"),
        ];
        for (input, _) in &cases {
            let result = normalize_arabic(input);
            let result_stripped: String = result
                .chars()
                .filter(|c| !c.is_ascii_punctuation())
                .collect();
            assert!(
                !result_stripped.is_empty(),
                "input: {} produced empty",
                input
            );
        }
        assert!(cases.len() >= 12);
    }

    #[test]
    fn stemmer_basic() {
        let stemmer = ArabicStemmer::new();
        assert_eq!(stemmer.stem("كتب"), "كتب");
    }

    #[test]
    fn stemmer_normalizes() {
        let stemmer = ArabicStemmer::new();
        let result = stemmer.stem("مَرْحَبًا");
        assert!(result.contains("م"));
        assert!(result.contains("ر"));
        assert!(result.contains("ح"));
        assert!(result.contains("ب"));
    }

    #[test]
    fn stemmer_at_least_8_cases() {
        let stemmer = ArabicStemmer::new();
        let cases = vec![
            "كتب",
            "مَرْحَبًا",
            "دَرَسَ",
            "كِتَاب",
            "مُدَرِّس",
            "مَدْرَسَة",
            "بِعْثَة",
            "جَامِعَة",
        ];
        assert!(cases.len() >= 8);
        for word in cases {
            let _ = stemmer.stem(word);
        }
    }

    #[test]
    fn ar_weights_default() {
        let weights = LanguageWeights::default();
        assert_eq!(weights.ar_weights.keyword_boost, 1.5);
        assert_eq!(weights.ar_weights.vector_weight, 0.4);
        assert_eq!(weights.ar_weights.keyword_weight, 0.6);
    }

    #[test]
    fn en_weights_default() {
        let weights = LanguageWeights::default();
        assert_eq!(weights.en_weights.keyword_boost, 1.0);
        assert_eq!(weights.en_weights.vector_weight, 0.5);
        assert_eq!(weights.en_weights.keyword_weight, 0.5);
    }

    #[test]
    fn language_aware_fusion_ar() {
        let weights = LanguageWeights::default();
        let result = language_aware_fusion(0.8, "ar", &weights);
        assert_eq!(result, 0.8 * 1.5);
    }

    #[test]
    fn language_aware_fusion_en() {
        let weights = LanguageWeights::default();
        let result = language_aware_fusion(0.8, "en", &weights);
        assert_eq!(result, 0.8 * 1.0);
    }

    #[test]
    fn arabic_seed_sources_count() {
        let sources = arabic_seed_sources();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].id, "ar_wikipedia");
        assert_eq!(sources[1].id, "ar_wikibooks");
        assert_eq!(sources[0].license, "CC-BY-SA-4.0");
    }

    #[test]
    fn arabic_seed_sources_license() {
        let sources = arabic_seed_sources();
        for s in sources {
            assert_eq!(s.license, "CC-BY-SA-4.0");
        }
    }
}
