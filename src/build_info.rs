use std::sync::OnceLock;

/// Crate version from Cargo.toml (`CARGO_PKG_VERSION`).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Compile-time Unix timestamp (seconds) emitted by `build.rs`.
pub fn build_ts() -> &'static str {
    env!("BUILD_TS")
}

/// Short git SHA at build time (or `"unknown"` outside a git checkout).
pub fn git_sha() -> &'static str {
    env!("GIT_SHA")
}

static SIMD_LEVEL: OnceLock<String> = OnceLock::new();

fn detect_simd() -> String {
    let mut features: Vec<&str> = Vec::new();

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            features.push("AVX512F");
        }
        if is_x86_feature_detected!("avx512bw") {
            features.push("AVX512BW");
        }
        if is_x86_feature_detected!("avx2") {
            features.push("AVX2");
        }
        if is_x86_feature_detected!("avx") {
            features.push("AVX");
        }
        if is_x86_feature_detected!("sse4.2") {
            features.push("SSE4.2");
        }
        if is_x86_feature_detected!("sse4.1") {
            features.push("SSE4.1");
        }
        if is_x86_feature_detected!("ssse3") {
            features.push("SSSE3");
        }
        if is_x86_feature_detected!("sse2") {
            features.push("SSE2");
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        features.push("NEON");
    }

    if features.is_empty() {
        "none".to_string()
    } else {
        features.join(" + ")
    }
}

pub fn simd_level() -> String {
    SIMD_LEVEL.get_or_init(detect_simd).clone()
}

pub fn simd_summary() -> String {
    simd_level()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_toml() {
        let v = version();
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
        assert!(!v.is_empty());
        assert!(v.starts_with('0'), "expected 0.x semver, got {v}");
    }

    #[test]
    fn build_ts_is_numeric() {
        let ts = build_ts();
        assert!(!ts.is_empty());
        assert!(
            ts.parse::<u64>().is_ok(),
            "BUILD_TS should be Unix seconds, got {ts}"
        );
    }

    #[test]
    fn git_sha_is_hex_or_unknown() {
        let sha = git_sha();
        assert!(!sha.is_empty());
        assert_eq!(sha.len(), 7, "expected short SHA or 'unknown', got {sha}");
        assert!(
            sha == "unknown" || sha.chars().all(|c| c.is_ascii_hexdigit()),
            "git_sha must be hex or unknown, got {sha}"
        );
    }
}
