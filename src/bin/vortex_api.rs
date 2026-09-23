use std::net::SocketAddr;
use std::sync::Arc;

use vortex_atoms_ai::llm_api;
use vortex_atoms_ai::llm_config::DeviceType;
use vortex_atoms_ai::llm_download::ModelDownloader;
use vortex_atoms_ai::security::{self, SecurityState};
use vortex_atoms_ai::vortex_config::VortexConfig;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // SIMD probe runs first: it only performs CPUID-style detection and exits
    // before model loading, tray setup, or inference runtime creation.
    if args.iter().any(|a| a == "--simd-probe") {
        let probe = vortex_atoms_ai::perf_topology::current_probe();
        println!("{}", vortex_atoms_ai::perf_topology::probe_report(&probe));
        std::process::exit(if probe.compatible { 0 } else { 1 });
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    if args.iter().any(|a| a == "--init-config") {
        let path = std::path::Path::new("vortex.json");
        if path.exists() {
            println!("[Vortex] vortex.json already exists. Remove it first to regenerate.");
        } else {
            let default = VortexConfig::default();
            let json = serde_json::to_string_pretty(&default)
                .expect("vortex_api: cannot serialize default config");
            std::fs::write(path, json).expect("vortex_api: cannot write vortex.json");
            println!("[Vortex] Created default vortex.json");
        }
        return;
    }

    let mut cfg_file = VortexConfig::load();

    // Environment overrides (12-factor for containers): explicit CLI flags
    // still win over everything.
    if let Ok(h) = std::env::var("VORTEX_HOST") {
        if !h.trim().is_empty() {
            cfg_file.server.host = h;
        }
    }
    if let Ok(p) = std::env::var("VORTEX_PORT") {
        if let Ok(port) = p.trim().parse::<u16>() {
            cfg_file.server.port = port;
        }
    }
    if let Ok(t) = std::env::var("VORTEX_API_TOKEN") {
        if !t.trim().is_empty() {
            cfg_file.auth.api_token = Some(t);
        }
    }
    if let Ok(t) = std::env::var("VORTEX_ADMIN_TOKEN") {
        if !t.trim().is_empty() {
            cfg_file.auth.admin_token = Some(t);
        }
    }

    let probe = vortex_atoms_ai::perf_topology::current_probe();
    let infer_threads = vortex_atoms_ai::perf_topology::resolve_infer_threads(
        cfg_file.performance.infer_threads,
        std::env::var("VORTEX_INFER_THREADS").ok().as_deref(),
        vortex_atoms_ai::perf_topology::auto_cpu_count(),
    );
    let async_workers =
        vortex_atoms_ai::perf_topology::resolve_async_workers(cfg_file.performance.async_workers);
    println!(
        "[API] CPU topology: {} (host: {}); infer_threads={infer_threads} async_workers={async_workers}",
        vortex_atoms_ai::perf_topology::probe_report(&probe),
        vortex_atoms_ai::perf_topology::host_features().join(" + "),
    );
    // Let Rayon-based dependencies size their global pool before first use,
    // without overriding an explicit operator setting.
    if std::env::var("RAYON_NUM_THREADS").is_err() {
        std::env::set_var("RAYON_NUM_THREADS", infer_threads.to_string());
    }

    let port: u16 = find_arg(&args, "--port")
        .and_then(|v| v.parse().ok())
        .unwrap_or(cfg_file.server.port);
    let host = find_arg(&args, "--host").unwrap_or(cfg_file.server.host);
    let allow_remote = args.iter().any(|a| a == "--allow-remote")
        || std::env::var("VORTEX_ALLOW_REMOTE")
            .map(|v| v == "1")
            .unwrap_or(false);
    let model_repo = find_arg(&args, "--repo");
    let model_file = find_arg(&args, "--model");
    let tokenizer = find_arg(&args, "--tokenizer");
    let device_type = parse_device(&args);
    let tray_enabled = args.iter().any(|a| a == "--tray");
    let tls = vortex_atoms_ai::llm_api::TlsIdentity {
        cert_path: find_arg(&args, "--tls-cert"),
        key_path: find_arg(&args, "--tls-key"),
    };
    let high_priority = args.iter().any(|a| a == "--high-priority");
    if high_priority {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Threading::{
                GetCurrentProcess, SetPriorityClass, ABOVE_NORMAL_PRIORITY_CLASS,
            };
            unsafe {
                SetPriorityClass(GetCurrentProcess(), ABOVE_NORMAL_PRIORITY_CLASS);
            }
            println!("[API] High priority: process class set to ABOVE_NORMAL");
        }
    }
    if args.iter().any(|a| a == "--read-only") {
        cfg_file.security.read_only = true;
        println!("[API] Read-only mode: swap/import/rotate/reload/session-purge are refused");
    }

    // Single instance: a second server would fight over the port and the
    // model cache — refuse early with a clear message instead.
    let _instance = match security::SingleInstance::acquire("api") {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[API] {e}");
            // Notify the first instance via tray before exiting with code 2.
            if let Err(e) = vortex_atoms_ai::system_tray::tray_flash(
                "Vortex Atoms AI",
                "another instance is already running",
            ) {
                eprintln!("[API] tray flash failed: {e}");
            }
            std::process::exit(2);
        }
    };

    println!("╔══════════════════════════════════════════════════════╗");
    println!("║        Vortex Atoms AI — HTTP API Server             ║");
    println!("║  5-Kernel Matrix REST API (OpenAI-compatible)       ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // ---- Security plane -------------------------------------------------
    // Tokens: explicit config/env values win, otherwise load-or-generate the
    // persisted pair under %LOCALAPPDATA%\vortex_atoms_ai\auth.json.
    let (api_token, admin_token) = security::ensure_tokens(&mut cfg_file.auth);
    let sec = Arc::new(SecurityState::new(
        cfg_file.security.clone(),
        api_token,
        admin_token,
    ));

    if sec.auth_enabled() {
        println!("[API] Auth: enabled (bearer user/admin roles, strict same-origin CORS)");
    } else {
        println!("[API] Auth: DISABLED — every local caller is trusted (not for shared networks!)");
    }

    // Fail closed: a non-loopback bind without token auth is refused unless
    // the operator explicitly passes --allow-remote (which still requires
    // auth to stay enabled).
    if !security::is_loopback_host(&host) {
        if !sec.auth_enabled() {
            eprintln!(
                "[API] REFUSING to bind {host}:{port}: non-loopback address with auth disabled."
            );
            eprintln!("[API] Fix: keep auth enabled (default) or bind 127.0.0.1.");
            std::process::exit(1);
        }
        if !allow_remote {
            eprintln!("[API] REFUSING to bind {host}:{port}: remote binds need --allow-remote (or VORTEX_ALLOW_REMOTE=1).");
            eprintln!("[API] The bundled UI auto-authenticates; remote browsers need the admin/user token out-of-band.");
            std::process::exit(1);
        }
        println!("[API] Remote bind explicitly allowed (--allow-remote); token auth is enforced.");
    }

    let (quit_tx, quit_rx) = tokio::sync::watch::channel(false);
    if tray_enabled {
        match vortex_atoms_ai::system_tray::spawn(format!("http://{host}:{port}"), quit_tx) {
            Some(_) => println!("[API] System tray enabled (stop via tray menu)"),
            None => println!("[API] Tray unavailable — rebuild with feature 'tray'"),
        }
    }

    let downloader = ModelDownloader::new(ModelDownloader::default_cache_dir());
    println!("[API] Preparing model...");

    let mut llm_cfg = match downloader.ensure_model(
        model_repo.as_deref(),
        model_file.as_deref(),
        tokenizer.as_deref(),
    ) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("[API] Failed to prepare model: {e}");
            eprintln!("[API] Fallback to vortex.json config paths...");
            let mut fallback = cfg_file.llm.clone();
            if let Some(f) = &model_file {
                fallback.model_path = std::path::PathBuf::from(f);
            }
            if let Some(t) = &tokenizer {
                fallback.tokenizer_path = std::path::PathBuf::from(t);
            }
            if fallback.model_path.exists() && fallback.tokenizer_path.exists() {
                fallback
            } else {
                eprintln!("[API] No valid model found. Use --repo or edit vortex.json");
                std::process::exit(1);
            }
        }
    };

    // Pass prefault config from vortex.json performance section.
    llm_cfg.prefault = cfg_file.performance.prefault;
    if llm_cfg.prefault {
        println!("[API] Prefault: enabled (will run in background)");
    }

    if find_arg(&args, "--device").is_none() {
        let dt = device_type;
        if dt != DeviceType::Cpu || llm_cfg.device_type != DeviceType::Cpu {
            llm_cfg.device_type = dt;
        }
    } else {
        llm_cfg.device_type = device_type;
    }
    println!("[API] Device: {}", device_label(&llm_cfg.device_type));

    if let Some(temp) = find_arg(&args, "--temperature").and_then(|v| v.parse::<f64>().ok()) {
        llm_cfg.sampling.temperature = Some(temp);
    }

    let addr: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[API] Invalid address '{host}:{port}': {e}");
            std::process::exit(1);
        }
    };

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(async_workers)
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[API] Failed to create tokio runtime: {e}");
            std::process::exit(1);
        }
    };
    rt.block_on(async {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let mut shutdown_tx = Some(shutdown_tx);
        let server = llm_api::start_server_with_tls(&llm_cfg, addr, sec, tls, shutdown_rx);
        tokio::pin!(server);
        let mut stop = quit_rx;
        // Ctrl+C is captured so in-flight requests drain instead of dying
        // mid-inference (previously the runtime was killed instantly).
        let mut ctrl_c = Box::pin(tokio::signal::ctrl_c());
        loop {
            tokio::select! {
                res = &mut server => {
                    if let Err(e) = res {
                        eprintln!("[API] Server error: {e}");
                        std::process::exit(1);
                    }
                    break;
                }
                _ = stop.changed() => {
                    if *stop.borrow_and_update() {
                        println!("[API] Stop requested from system tray — draining (30s max)");
                        graceful_stop(&mut shutdown_tx, server.as_mut()).await;
                        break;
                    }
                }
                _ = &mut ctrl_c => {
                    println!("[API] Ctrl+C — draining in-flight requests (30s max)");
                    graceful_stop(&mut shutdown_tx, server.as_mut()).await;
                    break;
                }
            }
        }
    });
}

/// Fire the graceful-shutdown signal, then give in-flight requests up to 30s
/// to drain before the runtime is torn down.
async fn graceful_stop(
    tx: &mut Option<tokio::sync::oneshot::Sender<()>>,
    server: std::pin::Pin<&mut impl std::future::Future<Output = vortex_atoms_ai::Result<()>>>,
) {
    if let Some(t) = tx.take() {
        let _ = t.send(());
    }
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), server).await;
}

fn parse_device(args: &[String]) -> DeviceType {
    let dev = find_arg(args, "--device").unwrap_or_else(|| "cpu".to_string());
    match dev.to_lowercase().as_str() {
        "cuda" => {
            let ord: usize = find_arg(args, "--cuda-device")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            DeviceType::Cuda(ord)
        }
        _ => DeviceType::Cpu,
    }
}

fn device_label(dt: &DeviceType) -> &'static str {
    match dt {
        DeviceType::Cpu => "CPU",
        DeviceType::Cuda(_) => "CUDA",
    }
}

fn find_arg(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn print_usage() {
    println!("Vortex Atoms AI — HTTP API Server");
    println!();
    println!("Usage: vortex_api [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --host <HOST>          Bind address (default: 127.0.0.1 or from vortex.json)");
    println!("  --port <PORT>          Port number (default: 8080 or from vortex.json)");
    println!("  --allow-remote         Permit non-loopback binds (requires token auth; fail-closed otherwise)");
    println!("  --read-only            Kiosk mode: refuse swap/import/rotate/reload/session-purge");
    println!("  --tls-cert <FILE>      PEM certificate for https (needs --tls-key)");
    println!("  --tls-key <FILE>       PEM private key for https (needs --tls-cert)");
    println!("  --repo <REPO>          HuggingFace repo");
    println!("  --model <FILE>         GGUF model filename");
    println!("  --tokenizer <FILE>     Tokenizer filename");
    println!("  --device <TYPE>        Device type: cpu or cuda (default: cpu)");
    println!("  --cuda-device <N>      CUDA device ordinal (default: 0)");
    println!("  --temperature <T>      Sampling temperature");
    println!("  --tray                  System-tray icon (rebuild with feature 'tray')");
    println!("  --high-priority        Set process priority to ABOVE_NORMAL (Windows)");
    println!("  --simd-probe           Print compiled SKU/host CPU compatibility and exit");
    println!("  --init-config          Generate default vortex.json");
    println!("  --help, -h             Show this help");
    println!();
    println!("First run (no arguments) loads vortex.json if present.");
    println!("CLI flags override vortex.json values.");
    println!();
    println!("API Endpoints:");
    println!("  POST /v1/generate       — Text generation");
    println!("  POST /v1/chat           — Chat completion");
    println!("  POST /v1/batch          — Batch generation");
    println!("  POST /v1/embeddings     — Generate embeddings");
    println!("  POST /v1/tools/execute  — Execute a tool directly");
    println!("  POST /v1/tools/call     — LLM + tool calling");
    println!("  POST /v1/models/swap    — Swap model runtime");
    println!("  POST /v1/knowledge/search   — Semantic search");
    println!("  POST /v1/knowledge/import   — Import knowledge");
    println!("  GET  /v1/health         — Health check");
    println!("  GET  /v1/device         — Device & SIMD info");
    println!("  GET  /v1/models         — Model info");
    println!("  GET  /v1/tools          — List available tools");
    println!("  WS   /ws                — WebSocket streaming");
    println!();
    println!("Example:");
    println!("  curl -X POST http://localhost:8080/v1/generate \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"prompt\": \"Hello, who are you?\"}}'");
    println!();
    println!("Config file: vortex.json (optional, JSON format).");
}
