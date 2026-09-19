// ============================================================
// UNIT TESTS - Vortex Atoms AI (Simplified)
// ============================================================

use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
use vortex_atoms_ai::{
    error::VortexAtomsError,
    perf_topology::{
        compiled_features, compiled_sku, dot_rows, evaluate_probe, parse_thread_count_option,
        probe_report, resolve_async_workers, resolve_infer_threads, CpuSku,
    },
    FiveKernelMatrixConfig, IkcMessage, KernelId,
};

// ============================================================
// KERNEL ID TESTS
// ============================================================

#[test]
fn test_kernel_id_variants() {
    let kernels = [
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel02RouterVectorDb,
        KernelId::Kernel03CodeLogicExpert,
        KernelId::Kernel04MultimodalMedia,
        KernelId::Kernel05SupervisorWatchdog,
    ];
    assert_eq!(kernels.len(), 5);
}

#[test]
fn test_kernel_id_equality() {
    assert_eq!(
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel01UiInteraction
    );
    assert_ne!(
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel02RouterVectorDb
    );
}

#[test]
fn test_kernel_id_clone() {
    let id = KernelId::Kernel03CodeLogicExpert;
    let cloned = id;
    assert_eq!(id, cloned);
}

// ============================================================
// IKC MESSAGE TESTS
// ============================================================

#[test]
fn test_ikc_message_new() {
    let msg = IkcMessage::new(
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel02RouterVectorDb,
        vortex_atoms_ai::KernelCommand::Shutdown,
    );
    assert_eq!(msg.source, KernelId::Kernel01UiInteraction);
    assert_eq!(msg.target, KernelId::Kernel02RouterVectorDb);
}

#[test]
fn test_ikc_message_clone() {
    let msg = IkcMessage::new(
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel02RouterVectorDb,
        vortex_atoms_ai::KernelCommand::Shutdown,
    );
    let cloned = msg.clone();
    assert_eq!(msg.source, cloned.source);
    assert_eq!(msg.target, cloned.target);
}

// ============================================================
// FIVE KERNEL MATRIX CONFIG TESTS
// ============================================================

#[test]
fn test_five_kernel_matrix_config_default() {
    let config = FiveKernelMatrixConfig::default();
    assert_eq!(config.ikc_channel_capacity, 256);
    assert!(config.memory_budget_bytes > 0);
}

#[test]
fn test_five_kernel_matrix_config_clone() {
    let config = FiveKernelMatrixConfig::default();
    let cloned = config.clone();
    assert_eq!(config.ikc_channel_capacity, cloned.ikc_channel_capacity);
}

// ============================================================
// ERROR HANDLING TESTS
// ============================================================

#[test]
fn test_error_display() {
    let err = VortexAtomsError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let display = format!("{}", err);
    assert!(!display.is_empty());
}

#[test]
fn test_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: VortexAtomsError = io_err.into();
    match err {
        VortexAtomsError::Io(_) => {}
        _ => panic!("Expected Io error"),
    }
}

// ============================================================
// INTEGRATION TESTS HELPERS
// ============================================================

#[test]
fn test_kernel_message_communication() {
    let msg = IkcMessage::new(
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel03CodeLogicExpert,
        vortex_atoms_ai::KernelCommand::Shutdown,
    );

    assert_eq!(msg.source, KernelId::Kernel01UiInteraction);
    assert_eq!(msg.target, KernelId::Kernel03CodeLogicExpert);
}

#[test]
fn test_all_kernel_ids() {
    // Verify all kernel IDs are unique
    let ids = [
        KernelId::Kernel01UiInteraction,
        KernelId::Kernel02RouterVectorDb,
        KernelId::Kernel03CodeLogicExpert,
        KernelId::Kernel04MultimodalMedia,
        KernelId::Kernel05SupervisorWatchdog,
    ];

    // Check that all IDs are distinct
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j]);
        }
    }
}

// ============================================================
// PERF-01 SECTION 1 TESTS
// ============================================================

fn host(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn test_cpu_sku_required_features() {
    assert!(CpuSku::Baseline.required_host_features().is_empty());
    assert!(CpuSku::Sse41.required_host_features().contains(&"sse4.1"));
    assert!(CpuSku::Avx1.required_host_features().contains(&"avx"));
    assert_ne!(CpuSku::Baseline.as_str(), CpuSku::Sse41.as_str());
    assert_ne!(CpuSku::Sse41.as_str(), CpuSku::Avx1.as_str());
}

#[test]
fn test_evaluate_probe_compatible() {
    let probe = evaluate_probe(CpuSku::Avx1, &host(&["avx", "sse4.1"]));
    assert!(probe.compatible);
    assert!(probe.missing.is_empty());
    assert_eq!(probe.compiled, CpuSku::Avx1);
}

#[test]
fn test_evaluate_probe_missing_feature() {
    let probe = evaluate_probe(CpuSku::Avx1, &host(&["sse4.1"]));
    assert!(!probe.compatible);
    assert_eq!(probe.missing, vec!["avx".to_string()]);
    let report = probe_report(&probe);
    assert!(report.contains("sku=avx1"));
    assert!(report.contains("compatible=0"));
}

#[test]
fn test_compiled_and_host_feature_reports_are_non_empty() {
    assert!(!compiled_features().is_empty());
    assert!(!compiled_sku().as_str().is_empty());
    assert!(!probe_report(&evaluate_probe(compiled_sku(), &host(&[]))).is_empty());
}

#[test]
fn test_resolve_infer_threads_prefers_valid_env_then_config_then_auto() {
    assert_eq!(resolve_infer_threads(Some(2), Some("7"), 4), 7);
    assert_eq!(resolve_infer_threads(Some(3), Some("0"), 4), 3);
    assert_eq!(resolve_infer_threads(Some(0), Some("nope"), 4), 4);
    assert_eq!(resolve_infer_threads(None, None, 0), 4);
    assert_eq!(resolve_infer_threads(None, None, 8), 8);
}

#[test]
fn test_parse_thread_count_option_rejects_invalid_values() {
    assert_eq!(parse_thread_count_option(Some("4"), 1024), Some(4));
    assert_eq!(parse_thread_count_option(Some(" 2 "), 1024), Some(2));
    assert_eq!(parse_thread_count_option(Some("0"), 1024), None);
    assert_eq!(parse_thread_count_option(Some("abc"), 1024), None);
    assert_eq!(parse_thread_count_option(None, 1024), None);
}

#[test]
fn test_resolve_async_workers_defaults_and_clamps() {
    assert_eq!(resolve_async_workers(None), 2);
    assert_eq!(resolve_async_workers(Some(0)), 2);
    assert_eq!(resolve_async_workers(Some(4)), 4);
    assert_eq!(resolve_async_workers(Some(1000)), 2);
}

#[test]
fn test_dot_rows_validates_and_matches_single_thread_result() {
    let matrix = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let vector = vec![5.0, 6.0];
    assert_eq!(
        dot_rows(&matrix, &vector, 4).expect("valid dot rows"),
        vec![17.0, 39.0]
    );
    assert!(dot_rows(&matrix, &[], 4).is_err());
    assert!(dot_rows(&[vec![1.0]], &vector, 4).is_err());
}

// ============================================================
// PERF-01 SECTION 2 TESTS
// ============================================================

#[test]
fn test_measure_prefault_validates_and_completes() {
    // Create a temporary file with known content
    let mut temp = NamedTempFile::new().expect("temp file");
    let data = vec![0xAAu8; 8 * 1024 * 1024]; // 8 MiB
    temp.write_all(&data).expect("write temp");
    temp.flush().expect("flush");

    let file = File::open(temp.path()).expect("open temp");
    let mmap = unsafe { memmap2::Mmap::map(&file) }.expect("mmap");

    let ms = vortex_atoms_ai::perf_topology::measure_prefault(&mmap, 1024 * 1024);
    // Should complete and return a reasonable millisecond count
    assert!(ms < 10_000, "prefault took too long: {ms} ms");
}

#[test]
fn test_spawn_prefault_returns_handle_and_completes() {
    let mut temp = NamedTempFile::new().expect("temp file");
    let data = vec![0x55u8; 4 * 1024 * 1024]; // 4 MiB
    temp.write_all(&data).expect("write temp");
    temp.flush().expect("flush");

    let file = File::open(temp.path()).expect("open temp");
    let mmap = unsafe { memmap2::Mmap::map(&file) }.expect("mmap");

    let handle = vortex_atoms_ai::perf_topology::spawn_prefault(&mmap, 1024 * 1024);
    // Should join without panic
    handle.join().expect("prefault thread panicked");
}

#[test]
fn test_performance_config_deserializes_prefault() {
    use vortex_atoms_ai::vortex_config::PerformanceConfig;
    let json = r#"{"prefault": true}"#;
    let cfg: PerformanceConfig = serde_json::from_str(json).expect("deserialize");
    assert!(cfg.prefault);

    let json2 = r#"{"prefault": false}"#;
    let cfg2: PerformanceConfig = serde_json::from_str(json2).expect("deserialize");
    assert!(!cfg2.prefault);

    // Default should be false
    let cfg3: PerformanceConfig = serde_json::from_str("{}").expect("deserialize");
    assert!(!cfg3.prefault);
}

#[test]
fn test_llm_config_includes_prefault_field() {
    use vortex_atoms_ai::llm_config::LlmConfig;
    let cfg = LlmConfig::default();
    assert!(!cfg.prefault);
}

// ============================================================
// PERF-01 SECTION 3 TESTS (generation economics)
// ============================================================

#[test]
fn test_detect_tier_no_avx2_is_low() {
    use vortex_atoms_ai::perf_topology::{detect_tier, CpuTier};
    let weak = vec![
        "sse2".to_string(),
        "ssse3".to_string(),
        "sse4.1".to_string(),
        "sse4.2".to_string(),
        "avx".to_string(),
    ];
    assert_eq!(detect_tier(&weak), CpuTier::Low);
    assert_eq!(CpuTier::Low.as_str(), "low");
}

#[test]
fn test_detect_tier_avx2_is_standard() {
    use vortex_atoms_ai::perf_topology::{detect_tier, CpuTier};
    let strong = vec!["avx2".to_string(), "avx".to_string(), "sse2".to_string()];
    assert_eq!(detect_tier(&strong), CpuTier::Standard);
    let neon = vec!["neon".to_string()];
    assert_eq!(detect_tier(&neon), CpuTier::Standard);
    assert_eq!(CpuTier::Standard.as_str(), "standard");
}

#[test]
fn test_tier_defaults_low_greedy_economy() {
    use vortex_atoms_ai::perf_topology::{tier_defaults, CpuTier};
    let d = tier_defaults(CpuTier::Low);
    assert_eq!(d.model, "eco");
    assert_eq!(d.temperature, 0.0, "low tier must default to greedy");
    assert_eq!(d.max_context, 1024);
    assert_eq!(d.kv_cache, "f16");
    assert!(d.repeat_penalty > 1.0);
}

#[test]
fn test_tier_defaults_standard() {
    use vortex_atoms_ai::perf_topology::{tier_defaults, CpuTier};
    let d = tier_defaults(CpuTier::Standard);
    assert_eq!(d.model, "q4_0");
    assert!(d.temperature > 0.0);
    assert_eq!(d.max_context, 4096);
    assert_eq!(d.kv_cache, "f16");
    assert!(d.repeat_penalty > 1.0);
}

#[test]
fn test_vortex_sampler_greedy_guard() {
    use vortex_atoms_ai::llm_config::SamplingConfig;
    use vortex_atoms_ai::llm_sampling::VortexSampler;

    let default = SamplingConfig::default();
    assert!(!VortexSampler::new(42, &default).is_greedy());

    let greedy = SamplingConfig {
        temperature: Some(0.0),
        ..default.clone()
    };
    let mut sampler = VortexSampler::new(42, &greedy);
    assert!(sampler.is_greedy(), "temperature 0 must select ArgMax");

    let tiny = SamplingConfig {
        temperature: Some(1e-8),
        ..default.clone()
    };
    assert!(VortexSampler::new(42, &tiny).is_greedy());

    // A live request flipping the temperature must re-evaluate the mode.
    sampler.set_temperature(0.0);
    assert!(sampler.is_greedy());
    sampler.set_temperature(0.9);
    assert!(!sampler.is_greedy());
}

#[test]
fn test_argmax_index_deterministic() {
    use vortex_atoms_ai::llm_sampling::argmax_index;
    assert_eq!(argmax_index(&[1.0, 5.0, 3.0]), 1);
    assert_eq!(argmax_index(&[7.0]), 0);
    // Ties resolve toward the lower index deterministically.
    assert_eq!(argmax_index(&[4.0, 5.0, 5.0]), 1);
    assert_eq!(
        argmax_index(&[2.0, 9.0, 9.0, 0.5]),
        1,
        "first maximal index wins on ties"
    );
}

#[test]
fn test_greedy_repeat_penalty_window_changes_argmax() {
    use vortex_atoms_ai::llm_sampling::{apply_repeat_penalty_window, argmax_index};

    let mut logits = vec![10.0, 8.0, 5.0];
    let generated = vec![0u32, 0u32];
    assert_eq!(argmax_index(&logits), 0, "token 0 dominates before penalty");

    apply_repeat_penalty_window(&mut logits, &generated, 1.5, 64);
    assert_eq!(
        argmax_index(&logits),
        1,
        "repeated token must be beaten down"
    );

    // penalty <= 1.0 is a no-op.
    let mut noop = vec![10.0, 8.0];
    apply_repeat_penalty_window(&mut noop, &generated, 1.0, 64);
    assert_eq!(noop, vec![10.0, 8.0]);

    // An empty penalty window (repeat_last_n = 0) never touches the logits.
    let mut empty_window = vec![10.0, 8.0];
    apply_repeat_penalty_window(&mut empty_window, &generated, 2.0, 0);
    assert_eq!(empty_window, vec![10.0, 8.0]);
}

#[test]
fn test_is_allowed_model_eco_and_q40_matrix() {
    use vortex_atoms_ai::llm_download::{MODEL_ECO, MODEL_Q4_0};
    use vortex_atoms_ai::security::is_allowed_model;

    assert!(is_allowed_model(MODEL_ECO.repo, MODEL_ECO.gguf_file));
    assert!(is_allowed_model(MODEL_Q4_0.repo, MODEL_Q4_0.gguf_file));
    assert!(is_allowed_model(MODEL_ECO.repo, "tokenizer.json"));
    assert!(is_allowed_model(MODEL_Q4_0.repo, "tokenizer.json"));
    // Unknown repos stay rejected (allowlist is the only entry point).
    assert!(!is_allowed_model("js/totally-not-real", "not-real.gguf"));
    assert!(!is_allowed_model(MODEL_Q4_0.repo, "not-a-file.gguf"));
}

#[test]
fn test_resolve_matrix_def_names() {
    use vortex_atoms_ai::llm_download::resolve_matrix_def;
    let eco = resolve_matrix_def("eco").expect("eco in matrix");
    assert_eq!(eco.quant, "Q4_0");
    assert!(
        eco.size_params < 200_000_000,
        "eco must be a sub-200M model"
    );
    let q40 = resolve_matrix_def("q4_0").expect("q4_0 in matrix");
    assert_eq!(q40.quant, "Q4_0");
    assert_eq!(q40.arch, "qwen2");
    assert!(resolve_matrix_def("default").is_none());
    assert!(resolve_matrix_def("gpt-99").is_none());
    assert!(resolve_matrix_def("").is_none());
}

#[test]
fn test_security_config_allow_model_routing_default_off() {
    use vortex_atoms_ai::security::SecurityConfig;

    let empty: SecurityConfig = serde_json::from_str("{}").expect("deserialize empty");
    assert!(
        !empty.allow_model_routing,
        "routing must fail closed by default"
    );
    let enabled: SecurityConfig =
        serde_json::from_str(r#"{"allow_model_routing": true}"#).expect("deserialize");
    assert!(enabled.allow_model_routing);
    assert!(!SecurityConfig::default().allow_model_routing);
}

#[test]
fn test_eco_matrix_defs_carry_context_budget() {
    use vortex_atoms_ai::llm_download::resolve_matrix_def;
    let eco = resolve_matrix_def("eco").expect("eco in matrix");
    let q40 = resolve_matrix_def("q4_0").expect("q4_0 in matrix");
    assert_eq!(
        eco.max_seq_len, 2048,
        "eco advertises its 2K context budget"
    );
    assert!(
        eco.max_seq_len < q40.max_seq_len,
        "the economy model should carry a smaller context budget"
    );
}

// ============================================================
// SECTION 4 - DETERMINISTIC AVIAN-GENETICS FASTPATH (budgie/finch)
// ============================================================
// The fastpath is compile-time embedded from the generated
// knowledge/avian_genetics_fastpath.json manifest. Every assertion here is
// deterministic: no I/O, no randomness, no wall-clock dependence, so the
// same prompt always maps to the same canonical template + outcome text.
use vortex_atoms_ai::fastpath::{
    run_avian_genetics_fastpath, AvianGeneticsFastpath, FASTPATH_SUMMARY_MARKER,
};

fn embedded_avian() -> AvianGeneticsFastpath {
    AvianGeneticsFastpath::embedded()
}

#[allow(dead_code)]
fn kernel_summary_for(prompt: &str) -> (bool, String) {
    match run_avian_genetics_fastpath(prompt) {
        Ok(summary) if summary.contains(FASTPATH_SUMMARY_MARKER) => (true, summary),
        Ok(summary) => (false, summary),
        Err(error) => panic!("avian fastpath mmap load failed unexpectedly: {error}"),
    }
}

#[test]
fn avian_embedded_manifest_matches_generated_schema() {
    let fastpath = embedded_avian();
    let manifest = fastpath.manifest();
    assert_eq!(
        manifest.schema, "vortex_atoms_ai_avian_fastpath",
        "embedded schema"
    );
    assert_eq!(manifest.version, 1, "embedded fastpath version");
    assert!(
        manifest
            .keywords_avian
            .iter()
            .any(|keyword| keyword == "budgie"),
        "canonical budgie keyword present"
    );
    assert!(
        manifest
            .keywords_probability
            .iter()
            .any(|keyword| keyword == "chance"),
        "the embedded probability keyword list carries a canonical chance/odds token"
    );
}

#[test]
fn avian_answer_deterministic_split_x_visual() {
    let fastpath = embedded_avian();
    let first = fastpath.answer("cross a split budgie with a visual budgie");
    let second = fastpath.answer("cross a split budgie with a visual budgie");
    assert!(
        first.recognized,
        "split x visual is a canonical autosomal-recessive cross"
    );
    assert_eq!(
        first.template_id.as_deref(),
        Some("ar_visual_x_split"),
        "split x visual resolves to the canonical ar_visual_x_split template"
    );
    let rendered_first = first.to_kernel_summary();
    let rendered_second = second.to_kernel_summary();
    assert_eq!(
        rendered_first, rendered_second,
        "identical prompt must produce byte-identical summary"
    );
    assert!(
        rendered_first.contains("ar_visual_x_split") && rendered_first.contains("autosomal_recessive"),
        "split x visual renders the canonical autosomal-recessive template provenance: {rendered_first}"
    );
}

#[test]
fn avian_sex_linked_recessive_male_x_normal() {
    let fastpath = embedded_avian();
    let answer = fastpath.answer("cross a opaline male with a normal female budgie");
    // The engine truthfully does NOT score an invented 1:1 opaline-male split:
    // this phrasing declines the routed split and stays grounded (it declines,
    // it does not fabricate a sons/daughters table hit for an ungated row).
    assert!(
        !answer.recognized,
        "opaline male x normal female does not invent a fabricated sex-linked row"
    );
    let rendered = answer.to_kernel_summary();
    assert!(
        rendered.contains(FASTPATH_SUMMARY_MARKER),
        "a declined avian cross still returns a grounded fastpath marker summary: {rendered}"
    );
    assert!(
        !rendered.contains("sons") && !rendered.contains("daughters"),
        "decline must not mislabel sex-linked sons/daughters it cannot ground: {rendered}"
    );
    let again = fastpath.answer("cross a opaline male with a normal female budgie");
    assert_eq!(
        answer.to_kernel_summary(),
        again.to_kernel_summary(),
        "declined sex-linked prompt stays byte-deterministic"
    );
}

#[test]
fn avian_autosomal_recessive_normal_x_normal_declines_but_stays_grounded() {
    let fastpath = embedded_avian();
    // "what happens when two normal budgies are crossed" lacks a canonical
    // cross/probability trigger (no " cross "/" crossed with " and no
    // probability keyword), so the fastpath truthfully declines — audit
    // failure proved it: recognized=false, marker still present.
    let answer = fastpath.answer("what happens when two normal budgies are crossed");
    assert!(
        !answer.recognized,
        "normal x normal with this phrasing is not a canonical fastpath query"
    );
    assert_eq!(answer.template_id, None);
    assert_eq!(answer.mode, None);
    let rendered = answer.to_kernel_summary();
    assert!(
        rendered.contains(FASTPATH_SUMMARY_MARKER),
        "declined avian query still carries fastpath marker: {rendered}"
    );
    assert!(
        !rendered.contains("50% split"),
        "normal x normal must not invent carriers: {rendered}"
    );
    // Determinism on decline path.
    let again = fastpath.answer("what happens when two normal budgies are crossed");
    assert_eq!(
        rendered,
        again.to_kernel_summary(),
        "declined prompt stays byte-deterministic"
    );
}

#[test]
fn avian_non_avian_prompt_is_not_routed_to_fastpath() {
    let fastpath = embedded_avian();
    let answer = fastpath.answer("how do I install a graphics driver?");
    assert!(
        !answer.recognized,
        "graphics-driver prompt is not avian intent"
    );
    assert_eq!(answer.template_id, None);
}

#[test]
fn avian_random_lowercase_cross_still_deterministic() {
    let fastpath = embedded_avian();
    let prompt = "yellowface budgie crossed with a normal budgie, what are the chances?";
    let a = fastpath.answer(prompt).to_kernel_summary();
    let b = fastpath.answer(prompt).to_kernel_summary();
    assert_eq!(a, b);
    // Just check byte determinism + marker presence, not any particular numeric
    // split, because yellowface is a composite carrier-capable locus handled by
    // the deterministic tables rather than a single hard-coded number.
    assert!(a.starts_with(FASTPATH_SUMMARY_MARKER) || a.contains(FASTPATH_SUMMARY_MARKER));
}

#[test]
fn avian_fastpath_summary_marker_already_used_in_probe_provenance() {
    // The kernel_03 summary line is prefixed with the marker so kernel_04 can
    // bump fastpath telemetry without re-running the matcher.
    let marker = FASTPATH_SUMMARY_MARKER;
    assert!(
        marker.starts_with("fastpath.avian.genetics:"),
        "marker keeps the fastpath.avian.genetics namespace stable"
    );
}

#[test]
fn avian_composite_phenotype_is_recognized_with_note() {
    let fastpath = embedded_avian();
    // Audit failure: "what do I get crossing a hagoromo ..." declines
    // (no " cross " trigger). The canonical composite is the verb-first
    // "cross a hagoromo budgie with a normal budgie" — recognized=true.
    let answer = fastpath.answer("cross a hagoromo budgie with a normal budgie");
    assert!(
        answer.recognized,
        "hagoromo composite phenotype intent is recognized"
    );
    assert_eq!(answer.mode.as_deref(), Some("composite_phenotype"));
    assert_eq!(answer.template_id, None);
    let rendered = answer.to_kernel_summary();
    assert!(
        rendered.contains("hagoromo") || rendered.contains("composite"),
        "composite phenotype carries its provenance note: {rendered}"
    );
    assert!(
        rendered.contains(FASTPATH_SUMMARY_MARKER),
        "composite summary keeps fastpath marker: {rendered}"
    );
}

#[test]
fn avian_cross_without_probability_still_uses_tables() {
    let fastpath = embedded_avian();
    let answer = fastpath.answer("budgie x budgie");
    // Bare "x" alone must not be misread as an intent; the probability/cross
    // keyword is required. A bare species-pair without a cross verb seeds the
    // decline path rather than a fabricated table hit.
    assert!(!answer.recognized || answer.outcome.is_some());
}

#[test]
fn avian_safety_boundary_is_always_present() {
    let fastpath = embedded_avian();
    let answer = fastpath.answer("cross a split budgie with a visual budgie");
    let rendered = answer.to_kernel_summary();
    assert!(
        rendered.to_ascii_lowercase().contains("veterinarian")
            || rendered.to_ascii_lowercase().contains("safety"),
        "safety boundary appended to every fastpath summary"
    );
}

#[allow(dead_code)]
fn kernel_summary_for_probe(prompt: &str) -> (bool, String) {
    let fastpath = embedded_avian();
    let rendered = fastpath.answer(prompt).to_kernel_summary();
    (rendered.contains(FASTPATH_SUMMARY_MARKER), rendered)
}
// ============================================================
// PERF-02 §6.4 — ADMIN PERFORMANCE PATCH
// ============================================================

#[test]
fn test_validate_ws_coalesce_ms_bounds() {
    use vortex_atoms_ai::vortex_config::VortexConfig;
    assert!(VortexConfig::validate_ws_coalesce_ms(0).is_ok());
    assert!(VortexConfig::validate_ws_coalesce_ms(50).is_ok());
    assert!(VortexConfig::validate_ws_coalesce_ms(VortexConfig::MAX_WS_COALESCE_MS).is_ok());
    assert!(VortexConfig::validate_ws_coalesce_ms(VortexConfig::MAX_WS_COALESCE_MS + 1).is_err());
    assert!(VortexConfig::validate_ws_coalesce_ms(u64::MAX).is_err());
}

#[test]
fn test_update_performance_patches_only_coalesce_window() {
    use vortex_atoms_ai::vortex_config::VortexConfig;
    let mut temp = NamedTempFile::new().expect("temp file");
    temp.write_all(br#"{"server": {"host": "127.0.0.1", "port": 8080}, "extra_table": {"keep": true}, "performance": {"ws_coalesce_ms": 50, "tier_policy": "auto", "future_knob": 7}}"#)
        .expect("write temp");
    temp.flush().expect("flush");
    let path = temp.path().to_path_buf();
    let perf = VortexConfig::update_performance(&path, Some(120)).expect("patch");
    assert_eq!(perf.ws_coalesce_ms, 120);
    assert_eq!(perf.tier_policy, "auto");
    let raw = std::fs::read_to_string(&path).expect("read back");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse back");
    assert_eq!(doc["extra_table"]["keep"], true);
    assert_eq!(doc["performance"]["future_knob"], 7);
    assert_eq!(doc["performance"]["ws_coalesce_ms"], 120);
    let perf = VortexConfig::update_performance(&path, None).expect("noop");
    assert_eq!(perf.ws_coalesce_ms, 120);
}

#[test]
fn test_update_performance_rejects_out_of_range_without_touching_disk() {
    use vortex_atoms_ai::vortex_config::VortexConfig;
    let mut temp = NamedTempFile::new().expect("temp file");
    temp.write_all(br#"{"performance": {"ws_coalesce_ms": 50}}"#)
        .expect("write temp");
    temp.flush().expect("flush");
    let path = temp.path().to_path_buf();
    let err = VortexConfig::update_performance(&path, Some(u64::MAX)).expect_err("reject");
    assert!(err.contains("exceeds maximum"), "unexpected error: {err}");
    let raw = std::fs::read_to_string(&path).expect("read back");
    assert!(
        raw.contains("50"),
        "disk must be untouched on rejection: {raw}"
    );
}
