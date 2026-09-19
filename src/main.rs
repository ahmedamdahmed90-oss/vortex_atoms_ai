use std::time::Duration;
use vortex_atoms_ai::{FiveKernelMatrix, FiveKernelMatrixConfig, KernelConfig};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--chat" || a == "-c") {
        eprintln!("[Main] Launching interactive chat...");
        eprintln!("[Main] For chat mode, run: cargo run --bin vortex_chat --release");
        return;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Vortex Atoms AI — 5-Kernel Matrix Runtime");
        println!();
        println!("Usage: vortex_atoms_ai [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --chat, -c       Launch interactive chat (use vortex_chat binary instead)");
        println!("  --help, -h       Show this help");
        println!();
        println!("Binaries:");
        println!("  vortex_atoms_ai      5-Kernel Matrix runtime (default)");
        println!("  vortex_chat          Interactive LLM chat");
        println!("  vortex_dashboard     GUI dashboard (requires --features dashboard)");
        return;
    }

    let config = KernelConfig::default();

    println!(
        "{} 5-Kernel Matrix ready: device={:?}, max_inflight_requests={}",
        config.identity, config.device, config.max_inflight_requests
    );
    println!("mmap-backed model loading remains available through VortexAtomsKernel::new(...).");

    let matrix = FiveKernelMatrix::spawn(FiveKernelMatrixConfig::default());
    let mut events = matrix.subscribe_events();

    if let Err(error) = matrix
        .submit_ui_input(
            "demo_session",
            "Run a Rust code logic analysis module and calculate the sum of 1, 2, and 3.",
        )
        .await
    {
        eprintln!("failed to submit demo UI input: {error}");
    }

    if let Err(error) = matrix.request_purge(0).await {
        eprintln!("failed to request supervisor purge: {error}");
    }

    tokio::time::sleep(Duration::from_millis(80)).await;

    while let Ok(event) = events.try_recv() {
        println!("IKC event: {event:?}");
    }

    matrix.shutdown().await;
}
