use std::sync::OnceLock;

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
