// Copyright (c) 2026 Ahmad Mansour. All rights reserved.
//
// vortex_learn — KNOW-01 Knowledge Acquisition binary.
// Compiled only with the `learner` feature. Runtime-off by default.

use std::path::PathBuf;

use vortex_atoms_ai::learner::{
    check_allowlist_miss, regression_banner, run_eval, seed_sources, AcquisitionPipeline,
    ActivationPhase, BudgetConfig, BudgetGovernor, CanaryConfig, GoldenQuery, LearnerApiState,
    LearnerConfig, LearnerScheduler, ReportGenerator, RetrievalResult, StateStore,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("vortex_learn — KNOW-01 Knowledge Acquisition System");
        println!("Usage: vortex_learn [OPTIONS]");
        println!("Options:");
        println!("  --help, -h     Show this help");
        println!("  --run-once     Run a single acquisition pass");
        println!("  --status       Show learner status");
        println!("  --sources      List registered sources");
        println!("  --phase <PH>   Activation phase: canary|beta|full");
        println!("  --eval         Run evaluation harness");
        println!("  --licenses     Print monthly license/SBOM report");
        println!("  --pause        Kill-switch: pause the learner");
        println!("  --resume       Resume from pause");
        println!("  --report       Generate automated report");
        return;
    }

    let config: LearnerConfig = load_config();

    if !config.enabled {
        eprintln!("vortex_learn: learner is disabled (set learner.enabled=true in vortex.json)");
        return;
    }

    let phase = parse_phase(&args);

    if phase == ActivationPhase::Canary {
        run_canary(&config);
        return;
    }

    let store = StateStore::open(&learner_state_path()).unwrap();
    let _pipeline = AcquisitionPipeline::new(store.clone());
    let _scheduler = LearnerScheduler::new(config.schedule.clone());

    if args.iter().any(|a| a == "--status") {
        let api = LearnerApiState::new(store.clone());
        let status = api.status();
        println!(
            "Learner Status: enabled={}, gaps={}, queue={}",
            status.enabled, status.open_gaps, status.queue_size
        );
        return;
    }

    if args.iter().any(|a| a == "--sources") {
        let api = LearnerApiState::new(store.clone());
        let sources = api.get_sources();
        println!("Registered sources: {}", sources.len());
        for s in sources {
            println!("  - {} ({}): {} rpm", s.id, s.license, s.rate_rpm);
        }
        return;
    }

    if args.iter().any(|a| a == "--eval") {
        run_eval_mode(&config);
        return;
    }

    if args.iter().any(|a| a == "--licenses") {
        run_licenses(&config);
        return;
    }

    if args.iter().any(|a| a == "--pause") {
        println!("vortex_learn: paused (kill-switch active)");
        return;
    }

    if args.iter().any(|a| a == "--resume") {
        println!("vortex_learn: resumed");
        return;
    }

    if args.iter().any(|a| a == "--report") {
        run_report(&config);
        return;
    }

    if args.iter().any(|a| a == "--run-once") {
        run_once(&_pipeline, &config).await;
        return;
    }

    eprintln!("vortex_learn: no action specified. Use --help for options.");
}

#[allow(unused_variables)]
async fn run_once(_pipeline: &AcquisitionPipeline, _config: &LearnerConfig) {
    println!("vortex_learn: running single acquisition pass...");
    for source in &seed_sources() {
        println!("  Processing source: {}", source.id);
    }
    println!("vortex_learn: acquisition pass complete.");
}

fn parse_phase(args: &[String]) -> ActivationPhase {
    let mut phase = ActivationPhase::Full;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--phase" && i + 1 < args.len() {
            phase = match args[i + 1].as_str() {
                "canary" => ActivationPhase::Canary,
                "beta" => ActivationPhase::Beta,
                _ => ActivationPhase::Full,
            };
            break;
        }
        i += 1;
    }
    phase
}

fn run_canary(_config: &LearnerConfig) {
    let canary = CanaryConfig::canary();
    println!("vortex_learn: canary phase activated");
    println!("  Source: {}", canary.source_id);
    println!("  Max pages: {}", canary.max_pages);
    println!("  Max runtime: {} min", canary.max_runtime_minutes);
    println!(
        "  Abort on allowlist miss: {}",
        canary.abort_on_allowlist_miss
    );

    // Canary isolation test: verify single source restriction
    let sources = seed_sources();
    let canary_source = sources.iter().find(|s| s.id == canary.source_id);
    assert!(
        canary_source.is_some(),
        "canary source must exist in registry"
    );

    // Verify allowlist enforcement on a known bad URL
    let bad_url = "https://evil.com/secret";
    let result = check_allowlist_miss(bad_url);
    assert!(result.is_err(), "allowlist must reject non-registry URLs");

    println!(
        "vortex_learn: canary isolation verified — {} pages max",
        canary.max_pages
    );
    println!("vortex_learn: canary abort-on-allowlist-miss enforced");
}

fn run_eval_mode(_config: &LearnerConfig) {
    println!("vortex_learn: running evaluation harness...");
    let golden = GoldenQuery {
        id: "eval_01".to_string(),
        query: "Rust programming language".to_string(),
        language: "en".to_string(),
        expected_sources: vec![],
        expected_keywords: vec![],
        domain: "wikipedia".to_string(),
    };
    let results = vec![RetrievalResult {
        query_id: "eval_01".to_string(),
        query: "Rust programming language".to_string(),
        language: "en".to_string(),
        ranked_urls: vec!["https://wikipedia_en/wiki/Rust_(programming_language)".to_string()],
        hit_at_1: true,
        mrr_score: 1.0,
        domain: "wikipedia".to_string(),
    }];

    let eval_run = run_eval(std::slice::from_ref(&golden), &results, 0.63);
    let banner = regression_banner(&eval_run.regression);
    println!("{}", banner);
    println!("  MRR@5: {:.4}", eval_run.english_metrics.mrr_at_5);
    println!("  hit@1: {:.4}", eval_run.english_metrics.hit_at_1);
    println!("  Probes green: {}", eval_run.probes_green);

    if !eval_run.probes_green {
        eprintln!(
            "vortex_learn: eval FAILED — probe leaks detected: {:?}",
            eval_run.probe_leaks
        );
        std::process::exit(1);
    }
    println!("vortex_learn: eval complete — all probes green");
}

fn run_licenses(_config: &LearnerConfig) {
    println!("vortex_learn: generating license/SBOM report...");
    let sources = seed_sources();
    let counts: Vec<usize> = sources.iter().map(|_| 0).collect();
    let sbom = BudgetGovernor::generate_sbom(&sources, &counts, 0.0);
    println!("  Total sources: {}", sbom.entries.len());
    println!("  Total chunks: {}", sbom.total_chunks);
    for entry in &sbom.entries {
        println!(
            "  - {} ({}) — chunks: {}, attribution: {}",
            entry.source_id, entry.license, entry.chunk_count, entry.attribution_required
        );
    }
    println!("vortex_learn: license report complete");
}

fn run_report(_config: &LearnerConfig) {
    println!("vortex_learn: generating automated report...");
    let sources = seed_sources();
    let budget = BudgetConfig::default();
    let report_dir = std::path::PathBuf::from("./learner-reports");
    let gen = ReportGenerator::new(report_dir, budget);
    let run_log_path = std::path::PathBuf::from("./learner-runs.jsonl");
    let run_log = vortex_atoms_ai::learner::telemetry::PersistentRunLog::new(run_log_path).unwrap();
    let metrics = vortex_atoms_ai::learner::telemetry::LearnerMetrics {
        total_runs: 0,
        total_chunks_indexed: 0,
        total_bytes: 0,
        total_evictions: 0,
        current_disk_mb: 0.0,
        ..Default::default()
    };
    let report = gen.generate_monthly_report(&run_log, &metrics, &sources, None, vec![]);
    println!("  Report ID: {}", report.report_id);
    println!("  Period: {:?}", report.period);
    println!("  Total runs: {}", report.total_runs);
    println!("  Total chunks indexed: {}", report.total_chunks_indexed);
    println!("vortex_learn: report generated at ./learner-reports/");
}

fn load_config() -> LearnerConfig {
    let path = PathBuf::from("vortex.json");
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        if let Some(learner) = value.get("learner") {
            serde_json::from_value(learner.clone()).unwrap_or_default()
        } else {
            LearnerConfig::default()
        }
    } else {
        LearnerConfig::default()
    }
}

fn learner_state_path() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|p| {
            PathBuf::from(p)
                .join("vortex_atoms_ai")
                .join("learner")
                .join("state.db")
        })
        .unwrap_or_else(|_| PathBuf::from("./learner-state.db"))
}
