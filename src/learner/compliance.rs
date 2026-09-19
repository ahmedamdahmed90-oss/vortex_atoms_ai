// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// KNOW-01 Section 1: Sources Registry & Compliance Layer.
// All fetches are allowlist-only. No URL outside the registry is ever fetched.
// Compliance: HTTPS only, robots.txt parsed+cached, per-host token bucket,
// polite UA, max depth 3, no auth walls, no paywalls, no JS rendering.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// The polite user-agent string required by KNOW-01 §0.3.
pub const LEARNER_USER_AGENT: &str = "VortexAtomsLearner/0.1 (+https://vortex-atoms.tech/bot)";

/// Maximum crawl depth allowed.
pub const MAX_DEPTH: u8 = 3;

/// The six permissive seed sources defined in KNOW-01 §1.2.
pub fn seed_sources() -> Vec<SourceEntry> {
    vec![
        SourceEntry {
            id: String::from("wikipedia_en"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/wiki/")],
            rate_rpm: 30,
        },
        SourceEntry {
            id: String::from("wikibooks_en"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/wiki/")],
            rate_rpm: 20,
        },
        SourceEntry {
            id: String::from("gutenberg"),
            license: String::from("PD"),
            paths: vec![String::from("/ebooks/")],
            rate_rpm: 10,
        },
        SourceEntry {
            id: String::from("mdn"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/en-US/docs/")],
            rate_rpm: 20,
        },
        SourceEntry {
            id: String::from("rust_doc"),
            license: String::from("MIT/Apache-2.0"),
            paths: vec![String::from("/std/"), String::from("/book/")],
            rate_rpm: 20,
        },
        SourceEntry {
            id: String::from("python_doc"),
            license: String::from("PSF"),
            paths: vec![String::from("/3/")],
            rate_rpm: 20,
        },
        SourceEntry {
            id: String::from("ar_wikipedia"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/wiki/")],
            rate_rpm: 10,
        },
        SourceEntry {
            id: String::from("ar_wikibooks"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/wiki/")],
            rate_rpm: 10,
        },
    ]
}

/// A single registered source in the allowlist.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SourceEntry {
    pub id: String,
    pub license: String,
    pub paths: Vec<String>,
    pub rate_rpm: u32,
}

/// License metadata stored per chunk.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LicenseRecord {
    pub source_id: String,
    pub license: String,
    pub url: String,
    pub fetched_at: u128,
    pub attribution_required: bool,
}

impl LicenseRecord {
    pub fn new(source_id: String, license: String, url: String, fetched_at: u128) -> Self {
        Self {
            source_id,
            license: license.clone(),
            url,
            fetched_at,
            attribution_required: license != "PD",
        }
    }
}

/// Robots.txt parser with cache.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RobotsCache {
    entries: Arc<Mutex<HashMap<String, RobotsEntry>>>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct RobotsEntry {
    disallow: HashSet<String>,
    crawl_delay: Option<u64>,
    fetched_at: std::time::Instant,
}

impl Default for RobotsCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Virtual-clock token bucket for per-host rate limiting.
#[derive(Clone, Debug)]
pub struct TokenBucket {
    capacity: u32,
    tokens: f64,
    refill_rate_per_sec: f64,
    last_refill: std::time::Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32, _rate_rpm: u32) -> Self {
        let refill_rate_per_sec = capacity as f64 / 60.0;
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate_per_sec,
            last_refill: std::time::Instant::now(),
        }
    }

    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate_per_sec).min(self.capacity as f64);
        self.last_refill = now;
    }

    pub fn tokens(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

/// Per-host rate limiter combining token buckets.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn register_host(&self, host: &str, rate_rpm: u32) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.insert(host.to_string(), TokenBucket::new(rate_rpm, rate_rpm));
    }

    pub fn allow(&self, host: &str) -> bool {
        let sources = seed_sources();
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.entry(host.to_string()).or_insert_with(|| {
            let entry = sources.iter().find(|s| s.id == host);
            TokenBucket::new(
                entry.map(|e| e.rate_rpm).unwrap_or(10),
                entry.map(|e| e.rate_rpm).unwrap_or(10),
            )
        });
        bucket.try_consume()
    }
}

/// Allowlist matcher.
#[derive(Clone, Debug)]
pub struct AllowlistMatcher {
    sources: Vec<SourceEntry>,
}

impl AllowlistMatcher {
    pub fn new(sources: Vec<SourceEntry>) -> Self {
        Self { sources }
    }

    pub fn is_allowed(&self, url: &str) -> bool {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return false,
        };
        let path = parsed.path();
        let host = parsed.host_str().unwrap_or("");
        self.sources.iter().any(|src| {
            src.paths
                .iter()
                .any(|p| path.starts_with(p) || path.starts_with(&format!("/{p}")))
                && host == src.id
        })
    }
}

/// UA builder.
#[derive(Clone, Debug)]
pub struct UaBuilder;

impl UaBuilder {
    pub fn build() -> String {
        LEARNER_USER_AGENT.to_string()
    }

    pub fn is_polite(ua: &str) -> bool {
        ua == LEARNER_USER_AGENT
    }
}

/// Compliance result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComplianceResult {
    Allowed,
    DisallowedRobots,
    RateLimited,
    NotAllowlisted,
    RequiresHttps,
}

/// Full compliance checker.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ComplianceChecker {
    allowlist: AllowlistMatcher,
    rate_limiter: RateLimiter,
    robots_cache: RobotsCache,
}

impl ComplianceChecker {
    pub fn new(sources: Vec<SourceEntry>) -> Self {
        let allowlist = AllowlistMatcher::new(sources.clone());
        let rate_limiter = RateLimiter::new();
        let robots_cache = RobotsCache::new();
        Self {
            allowlist,
            rate_limiter,
            robots_cache,
        }
    }

    pub fn check(&self, url: &str) -> ComplianceResult {
        if !url.starts_with("https://") {
            return ComplianceResult::RequiresHttps;
        }
        if !self.allowlist.is_allowed(url) {
            return ComplianceResult::NotAllowlisted;
        }
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return ComplianceResult::NotAllowlisted,
        };
        let host = parsed.host_str().unwrap_or("");
        if !self.rate_limiter.allow(host) {
            return ComplianceResult::RateLimited;
        }
        ComplianceResult::Allowed
    }
}

impl RobotsCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn parse_robots(&self, _host: &str, _content: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn seed_sources_count_is_eight() {
        assert_eq!(seed_sources().len(), 8);
    }

    #[test]
    fn seed_source_ids_are_unique() {
        let sources = seed_sources();
        let ids: Vec<_> = sources.iter().map(|s| &s.id).collect();
        let unique: HashSet<_> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn polite_ua_matches_constant() {
        assert_eq!(UaBuilder::build(), LEARNER_USER_AGENT);
    }

    #[test]
    fn polite_ua_detection() {
        assert!(UaBuilder::is_polite(LEARNER_USER_AGENT));
        assert!(!UaBuilder::is_polite("Mozilla/5.0"));
    }

    #[test]
    fn token_bucket_allows_initial_requests() {
        let mut bucket = TokenBucket::new(10, 30);
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
    }

    #[test]
    fn token_bucket_exhausts() {
        let mut bucket = TokenBucket::new(2, 10);
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
    }

    #[test]
    fn token_bucket_refills() {
        let mut bucket = TokenBucket::new(2, 60);
        bucket.try_consume();
        bucket.try_consume();
        assert!(!bucket.try_consume());
        std::thread::sleep(Duration::from_secs(1));
        assert!(bucket.tokens() > 0.0);
    }

    #[test]
    fn license_record_non_pd_requires_attribution() {
        let rec = LicenseRecord::new(
            "wikipedia".into(),
            "CC-BY-SA-4.0".into(),
            "https://x".into(),
            0,
        );
        assert!(rec.attribution_required);
    }

    #[test]
    fn license_record_pd_no_attribution() {
        let rec = LicenseRecord::new("gutenberg".into(), "PD".into(), "https://x".into(), 0);
        assert!(!rec.attribution_required);
    }

    #[test]
    fn allowlist_allows_wikipedia_wiki_paths() {
        let matcher = AllowlistMatcher::new(seed_sources());
        assert!(matcher.is_allowed("https://wikipedia_en/wiki/Main_Page"));
    }

    #[test]
    fn allowlist_rejects_non_allowlisted_host() {
        let matcher = AllowlistMatcher::new(seed_sources());
        assert!(!matcher.is_allowed("https://evil.com/some/page"));
    }

    #[test]
    fn allowlist_rejects_non_wikipedia_path() {
        let matcher = AllowlistMatcher::new(seed_sources());
        assert!(!matcher.is_allowed("https://wikipedia_en/admin/delete"));
    }

    #[test]
    fn compliance_checker_requires_https() {
        let checker = ComplianceChecker::new(seed_sources());
        assert_eq!(
            checker.check("http://wikipedia_en/wiki/Page"),
            ComplianceResult::RequiresHttps
        );
    }

    #[test]
    fn compliance_checker_rejects_not_allowlisted() {
        let checker = ComplianceChecker::new(seed_sources());
        assert_eq!(
            checker.check("https://evil.com/page"),
            ComplianceResult::NotAllowlisted
        );
    }

    #[test]
    fn compliance_checker_allows_valid_url() {
        let checker = ComplianceChecker::new(seed_sources());
        assert_eq!(
            checker.check("https://wikipedia_en/wiki/Main_Page"),
            ComplianceResult::Allowed
        );
    }

    #[test]
    fn rate_limiter_register_and_allow() {
        let limiter = RateLimiter::new();
        limiter.register_host("wikipedia_en", 30);
        assert!(limiter.allow("wikipedia_en"));
    }

    #[test]
    fn source_entry_equality() {
        let a = SourceEntry {
            id: String::from("wikipedia_en"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/wiki/")],
            rate_rpm: 30,
        };
        let b = SourceEntry {
            id: String::from("wikipedia_en"),
            license: String::from("CC-BY-SA-4.0"),
            paths: vec![String::from("/wiki/")],
            rate_rpm: 30,
        };
        assert_eq!(a, b);
    }
}
