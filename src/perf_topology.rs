// Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
//! CPU SKU selection and inference thread topology for weak hardware.
//!
//! This module keeps performance work measurable and safe: compile-time CPU
//! SKUs are reported explicitly, host compatibility is checked before model
//! code runs, and thread-pool sizing is validated without panics. It does not
//! change inference numerics.

use crate::error::{Result, VortexAtomsError};

/// Maximum inference threads accepted from configuration or environment.
pub const MAX_INFER_THREADS: usize = 1024;
/// Default and fallback async worker count for the Tokio runtime.
pub const DEFAULT_ASYNC_WORKERS: usize = 2;
/// Maximum async workers accepted from configuration.
pub const MAX_ASYNC_WORKERS: usize = 16;

/// Release CPU optimization tiers.
///
/// `Baseline` is the only safe default artifact. `Sse41` and `Avx1` are
/// opt-in SKUs built with additional target features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuSku {
    /// Safe x86-64 baseline (SSE2).
    Baseline,
    /// Opt-in SSE4.1/SSSE3 optimized build.
    Sse41,
    /// Opt-in AVX-optimized build.
    Avx1,
}

impl CpuSku {
    /// Short stable identifier used by build artifacts and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Sse41 => "sse41",
            Self::Avx1 => "avx1",
        }
    }

    /// Host CPU features required before this compiled SKU may run.
    pub fn required_host_features(self) -> &'static [&'static str] {
        match self {
            Self::Baseline => &[],
            Self::Sse41 => &["sse4.1"],
            Self::Avx1 => &["avx"],
        }
    }

    /// SKU selected by the compiler target features for this binary.
    pub fn from_compiled() -> Self {
        if cfg!(target_feature = "avx") {
            Self::Avx1
        } else if cfg!(target_feature = "sse4.1") {
            Self::Sse41
        } else {
            Self::Baseline
        }
    }
}

/// SKU this binary was compiled with.
pub fn compiled_sku() -> CpuSku {
    CpuSku::from_compiled()
}

/// Compile-time CPU features enabled for this binary.
pub fn compiled_features() -> Vec<String> {
    let mut features = Vec::new();
    if cfg!(target_feature = "avx") {
        features.push("avx".to_string());
    }
    if cfg!(target_feature = "sse4.2") {
        features.push("sse4.2".to_string());
    }
    if cfg!(target_feature = "sse4.1") {
        features.push("sse4.1".to_string());
    }
    if cfg!(target_feature = "ssse3") {
        features.push("ssse3".to_string());
    }
    if features.is_empty() {
        features.push("sse2".to_string());
    }
    features
}

/// Host CPU features detected at runtime.
///
/// This only executes CPUID-style detection; it never executes model or AVX
/// workload code.
pub fn host_features() -> Vec<String> {
    let mut features = Vec::new();

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            features.push("avx2".to_string());
        }
        if is_x86_feature_detected!("avx") {
            features.push("avx".to_string());
        }
        if is_x86_feature_detected!("sse4.2") {
            features.push("sse4.2".to_string());
        }
        if is_x86_feature_detected!("sse4.1") {
            features.push("sse4.1".to_string());
        }
        if is_x86_feature_detected!("ssse3") {
            features.push("ssse3".to_string());
        }
        if is_x86_feature_detected!("sse2") {
            features.push("sse2".to_string());
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        features.push("neon".to_string());
    }

    features
}

/// Result of checking a compiled SKU against detected host CPU features.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimdProbe {
    /// SKU encoded into the running executable.
    pub compiled: CpuSku,
    /// Host features observed at runtime.
    pub host: Vec<String>,
    /// True when every required compiled feature is present on the host.
    pub compatible: bool,
    /// Required features missing from the host.
    pub missing: Vec<String>,
}

/// Check a compiled SKU against an explicit host feature list.
///
/// The list comparison is intentionally pure so SKU policy can be unit tested
/// without depending on the machine running the tests.
pub fn evaluate_probe(compiled: CpuSku, host: &[String]) -> SimdProbe {
    let mut missing = Vec::new();
    for required in compiled.required_host_features() {
        if !host.iter().any(|have| have == required) {
            missing.push((*required).to_string());
        }
    }
    SimdProbe {
        compiled,
        host: host.to_vec(),
        compatible: missing.is_empty(),
        missing,
    }
}

/// Check this executable against the current host CPU.
pub fn current_probe() -> SimdProbe {
    evaluate_probe(compiled_sku(), &host_features())
}

/// Stable one-line probe report for scripts and startup logs.
pub fn probe_report(probe: &SimdProbe) -> String {
    let host = if probe.host.is_empty() {
        "none".to_string()
    } else {
        probe.host.join(" + ")
    };
    let missing = if probe.missing.is_empty() {
        "-".to_string()
    } else {
        probe.missing.join(",")
    };
    format!(
        "sku={} host={} compatible={} missing={}",
        probe.compiled.as_str(),
        host,
        u8::from(probe.compatible),
        missing
    )
}

/// CPU capability tier used to choose generation defaults.
///
/// Weak hardware ("low") cannot pay for stochastic sampling + long contexts;
/// the tier-appropriate defaults are exposed to clients through `/v1/models`
/// so the engine stays fast and deterministic by default where it matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CpuTier {
    /// No AVX2 (or NEON on aarch64): economy defaults (greedy, short context).
    Low,
    /// AVX2-class SIMD available: comfortable defaults (sampled, 4K context).
    Standard,
}

impl CpuTier {
    /// Stable short identifier used in logs and the models endpoint.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Standard => "standard",
        }
    }
}

/// Classify a host feature list into a CPU tier.
///
/// Rule of thumb: no AVX2 (and no NEON) means "low" — the CPU is too weak for
/// stochastic sampling without hairline latency, so generation defaults to
/// greedy decoding with a small context budget.
pub fn detect_tier(features: &[String]) -> CpuTier {
    if features
        .iter()
        .any(|feature| feature == "avx2" || feature == "neon")
    {
        CpuTier::Standard
    } else {
        CpuTier::Low
    }
}

/// Tier of the host this binary is running on.
pub fn current_tier() -> CpuTier {
    detect_tier(&host_features())
}

/// Generation defaults advertised to clients for a CPU tier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TierDefaults {
    /// Matrix model name suggested as the default for this class of hardware.
    pub model: &'static str,
    /// Default sampling temperature (0.0 = greedy/fast path).
    pub temperature: f64,
    /// Default context budget in tokens.
    pub max_context: usize,
    /// KV-cache dtype hint for this tier.
    pub kv_cache: &'static str,
    /// Default repeat penalty.
    pub repeat_penalty: f32,
}

/// Return the tier-appropriate generation defaults.
///
/// `Low` tier prefers the tiny `eco` model, greedy decoding (temperature 0.0,
/// the allocation-light fast path) and a 1024-token context. `Standard` tier
/// prefers the `q4_0` 0.5B model with sampled decoding and 4K context.
pub fn tier_defaults(tier: CpuTier) -> TierDefaults {
    match tier {
        CpuTier::Low => TierDefaults {
            model: "eco",
            temperature: 0.0,
            max_context: 1024,
            kv_cache: "f16",
            repeat_penalty: 1.1,
        },
        CpuTier::Standard => TierDefaults {
            model: "q4_0",
            temperature: 0.8,
            max_context: 4096,
            kv_cache: "f16",
            repeat_penalty: 1.1,
        },
    }
}

/// Logical CPU count used when inference threads are set to automatic.
pub fn auto_cpu_count() -> usize {
    match std::thread::available_parallelism() {
        Ok(count) => count.get(),
        Err(_) => 4,
    }
}

/// Parse an optional thread-count override without panicking.
pub fn parse_thread_count_option(raw: Option<&str>, max: usize) -> Option<usize> {
    let text = raw?.trim();
    let value = text.parse::<usize>().ok()?;
    if (1..=max).contains(&value) {
        Some(value)
    } else {
        None
    }
}

/// Resolve inference threads: environment wins, then config, then automatic.
///
/// Invalid values are ignored so a typo can never wedge the server at boot.
pub fn resolve_infer_threads(
    configured: Option<usize>,
    env_override: Option<&str>,
    auto_cpus: usize,
) -> usize {
    if let Some(value) = parse_thread_count_option(env_override, MAX_INFER_THREADS) {
        return value;
    }
    if let Some(value) = configured {
        if (1..=MAX_INFER_THREADS).contains(&value) {
            return value;
        }
    }
    let auto = if auto_cpus == 0 { 4 } else { auto_cpus };
    auto.clamp(1, MAX_INFER_THREADS)
}

/// Resolve Tokio async workers, defaulting to two on weak hardware.
pub fn resolve_async_workers(configured: Option<usize>) -> usize {
    match configured {
        Some(value) if (1..=MAX_ASYNC_WORKERS).contains(&value) => value,
        _ => DEFAULT_ASYNC_WORKERS,
    }
}

/// Validate shapes for row-wise dot-product benchmarking.
fn validate_dot_rows(matrix: &[Vec<f32>], vector: &[f32]) -> Result<()> {
    if vector.is_empty() {
        return Err(VortexAtomsError::InvalidTensorSpec(
            "dot-product vector must not be empty".to_string(),
        ));
    }
    for (index, row) in matrix.iter().enumerate() {
        if row.len() != vector.len() {
            return Err(VortexAtomsError::InvalidTensorSpec(format!(
                "row {index} has length {}, expected {}",
                row.len(),
                vector.len()
            )));
        }
    }
    Ok(())
}

/// Deterministic per-row dot product.
///
/// The accumulation order is sequential within each row, so parallel and
/// single-threaded evaluation remain bit-identical.
fn dot_product(row: &[f32], vector: &[f32]) -> f32 {
    row.iter()
        .zip(vector.iter())
        .map(|(value, weight)| value * weight)
        .sum()
}

/// Prefault mapped model pages in a background thread.
///
/// Touches pages at a 1 MiB stride with a trivial XOR accumulate and
/// `std::hint::black_box` to prevent optimisation. Returns a join handle
/// so the caller can await completion if desired, but the primary purpose
/// is to start the work without blocking the health endpoint.
pub fn spawn_prefault(mmap: &memmap2::Mmap, stride_bytes: usize) -> std::thread::JoinHandle<()> {
    let len = mmap.len();
    let stride = stride_bytes.max(1);
    let slice = unsafe { std::slice::from_raw_parts(mmap.as_ptr(), len) };
    std::thread::spawn(move || {
        let mut acc: u8 = 0;
        let mut offset = 0;
        while offset < len {
            // SAFETY: slice is derived from the same mmap; offset is bounded.
            let byte = unsafe { *slice.get_unchecked(offset) };
            acc ^= byte;
            std::hint::black_box(acc);
            offset += stride;
        }
        // Final black_box prevents the loop from being optimised away.
        std::hint::black_box(acc);
    })
}

/// Hint the OS to prefetch the next layer's pages while current layer computes.
///
/// On Windows uses `PrefetchVirtualMemory` (best-effort, no error if missing);
/// on Unix uses `madvise(WILLNEED)`. No-op if `mmap` is empty or prefetch
/// is disabled via config `layer_prefetch:false`.
pub fn prefetch_next_layer(mmap: &memmap2::Mmap, offset: usize, len: usize) {
    if mmap.is_empty() || len == 0 {
        return;
    }
    let total = mmap.len();
    let start = offset.min(total);
    let end = (start + len).min(total);
    if start >= end {
        return;
    }
    #[cfg(windows)]
    {
        // Best-effort: touch one byte per 4 KiB page to fault it in.
        // True PrefetchVirtualMemory would need winapi; this portable touch
        // achieves the same double-buffer effect without extra deps.
        let slice = unsafe { std::slice::from_raw_parts(mmap.as_ptr(), total) };
        let mut acc: u8 = 0;
        let mut p = start & !4095usize;
        while p < end {
            acc ^= unsafe { *slice.get_unchecked(p) };
            std::hint::black_box(acc);
            p += 4096;
        }
    }
    #[cfg(unix)]
    {
        let ptr = unsafe { mmap.as_ptr().add(start) as *mut libc::c_void };
        let page_len = end - start;
        unsafe {
            libc::madvise(ptr, page_len, libc::MADV_WILLNEED);
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (offset, len);
    }
}

/// Time the prefault operation synchronously (for bench endpoint).
pub fn measure_prefault(mmap: &memmap2::Mmap, stride_bytes: usize) -> u128 {
    let len = mmap.len();
    let stride = stride_bytes.max(1);
    let slice = unsafe { std::slice::from_raw_parts(mmap.as_ptr(), len) };
    let start = std::time::Instant::now();
    let mut acc: u8 = 0;
    let mut offset = 0;
    while offset < len {
        let byte = unsafe { *slice.get_unchecked(offset) };
        acc ^= byte;
        std::hint::black_box(acc);
        offset += stride;
    }
    std::hint::black_box(acc);
    start.elapsed().as_millis()
}

/// Single-threaded row-wise dot products.
fn dot_rows_single(matrix: &[Vec<f32>], vector: &[f32]) -> Vec<f32> {
    matrix.iter().map(|row| dot_product(row, vector)).collect()
}

/// Representative row-wise dot-product workload for thread-topology checks.
///
/// With the `par-matmul` feature, rows are evaluated with Rayon when more
/// than one thread is requested. Without that feature, the same validated
/// single-threaded path is used. This is a benchmarking/topology harness; it
/// is not a replacement for quantized model kernels.
#[cfg(feature = "par-matmul")]
pub fn dot_rows(matrix: &[Vec<f32>], vector: &[f32], threads: usize) -> Result<Vec<f32>> {
    validate_dot_rows(matrix, vector)?;
    if threads <= 1 || matrix.len() < 2 {
        return Ok(dot_rows_single(matrix, vector));
    }
    {
        use rayon::prelude::*;
        Ok(matrix
            .par_iter()
            .map(|row| dot_product(row, vector))
            .collect())
    }
}

/// Representative row-wise dot-product workload without Rayon.
///
/// The optional `par-matmul` feature is disabled, so callers always receive
/// the validated single-threaded result.
#[cfg(not(feature = "par-matmul"))]
pub fn dot_rows(matrix: &[Vec<f32>], vector: &[f32], threads: usize) -> Result<Vec<f32>> {
    let _ = threads;
    validate_dot_rows(matrix, vector)?;
    Ok(dot_rows_single(matrix, vector))
}
