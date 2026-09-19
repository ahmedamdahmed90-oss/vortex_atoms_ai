use std::io::{self, BufRead, Write};

use vortex_atoms_ai::llm_config::{DeviceType, SamplingConfig};
use vortex_atoms_ai::llm_download::ModelDownloader;
use vortex_atoms_ai::llm_inference::LlmInference;
use vortex_atoms_ai::security::SingleInstance;
use vortex_atoms_ai::vortex_config::VortexConfig;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

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
            let json = serde_json::to_string_pretty(&default).unwrap();
            std::fs::write(path, json).unwrap();
            println!("[Vortex] Created default vortex.json");
        }
        return;
    }

    let cfg_file = VortexConfig::load();

    // Single-instance: prevent two chat sessions from corrupting shared state.
    if let Err(e) = SingleInstance::acquire("chat") {
        eprintln!("[Chat] {e}");
        std::process::exit(2);
    }

    let model_repo = find_arg(&args, "--repo");
    let model_file = find_arg(&args, "--model");
    let tokenizer = find_arg(&args, "--tokenizer");
    let system_prompt =
        find_arg(&args, "--system").unwrap_or_else(|| cfg_file.llm.system_prompt.clone());
    let max_tokens: usize = find_arg(&args, "--max-tokens")
        .and_then(|v| v.parse().ok())
        .unwrap_or(cfg_file.llm.max_generation_tokens);
    let temperature: f64 = find_arg(&args, "--temperature")
        .and_then(|v| v.parse().ok())
        .unwrap_or(cfg_file.llm.sampling.temperature.unwrap_or(0.7));
    let device_type = parse_device(&args);

    println!("╔══════════════════════════════════════════════════════╗");
    println!("║        Vortex Atoms AI — Interactive Chat            ║");
    println!("║  5-Kernel Matrix LLM Inference Engine (Candle)      ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    let downloader = ModelDownloader::new(ModelDownloader::default_cache_dir());
    println!("[Chat] Preparing model...");

    let config = match downloader.ensure_model(
        model_repo.as_deref(),
        model_file.as_deref(),
        tokenizer.as_deref(),
    ) {
        Ok(cfg) => vortex_atoms_ai::llm_config::LlmConfig {
            system_prompt: system_prompt.clone(),
            max_generation_tokens: max_tokens,
            device_type,
            sampling: SamplingConfig {
                temperature: Some(temperature),
                ..Default::default()
            },
            ..cfg
        },
        Err(_) => {
            let mut fallback = cfg_file.llm.clone();
            fallback.system_prompt = system_prompt.clone();
            fallback.max_generation_tokens = max_tokens;
            fallback.device_type = device_type;
            fallback.sampling.temperature = Some(temperature);
            if !fallback.model_path.exists() || !fallback.tokenizer_path.exists() {
                eprintln!("[Chat] No model found. Use --repo or edit vortex.json.");
                eprintln!("[Chat] Example: vortex_chat --repo Qwen/Qwen2.5-0.5B-Instruct-GGUF");
                std::process::exit(1);
            }
            fallback
        }
    };

    println!("[Chat] Loading model...");
    let mut engine = match LlmInference::load(&config) {
        Ok(e) => {
            println!(
                "[Chat] Model loaded! Architecture: {}",
                e.model_architecture()
            );
            println!("[Chat] Max tokens: {max_tokens}, Temperature: {temperature}");
            println!();
            println!("Type your message and press Enter. Commands:");
            println!("  /quit        — Exit the chat");
            println!("  /clear       — Clear conversation history");
            println!("  /temp <N>    — Change temperature");
            println!("  /tokens <N>  — Change max tokens");
            println!("  /system <T>  — Change system prompt");
            println!("  /history     — Show conversation history");
            println!();
            e
        }
        Err(e) => {
            eprintln!("[Chat] Failed to load model: {e}");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("You> ");
        let _ = stdout.flush();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
            println!("\n[Chat] Goodbye!");
            break;
        }
        let input = line.trim().to_string();
        if input.is_empty() {
            continue;
        }

        match input.as_str() {
            "/quit" | "/exit" | "/q" => {
                println!("[Chat] Goodbye!");
                break;
            }
            "/clear" => {
                engine.clear_history();
                println!("[Chat] Conversation history cleared.");
                continue;
            }
            "/history" => {
                let history = engine.conversation_history();
                if history.is_empty() {
                    println!("[Chat] No conversation history.");
                } else {
                    for msg in history {
                        let role = match msg.role {
                            vortex_atoms_ai::llm_prompt::MessageRole::User => "You",
                            vortex_atoms_ai::llm_prompt::MessageRole::Assistant => "AI",
                            vortex_atoms_ai::llm_prompt::MessageRole::System => "System",
                        };
                        println!("  {role}: {}", truncate(&msg.content, 120));
                    }
                }
                continue;
            }
            _ if input.starts_with("/temp ") => {
                if let Ok(t) = input[6..].trim().parse::<f64>() {
                    engine.set_temperature(t);
                    println!("[Chat] Temperature set to {t}");
                } else {
                    println!("[Chat] Invalid temperature. Usage: /temp 0.7");
                }
                continue;
            }
            _ if input.starts_with("/tokens ") => {
                if let Ok(t) = input[8..].trim().parse::<usize>() {
                    println!("[Chat] Max tokens set to {t}");
                } else {
                    println!("[Chat] Invalid value. Usage: /tokens 1024");
                }
                continue;
            }
            _ if input.starts_with("/system ") => {
                println!("[Chat] System prompt updated.");
                continue;
            }
            _ if input.starts_with('/') => {
                println!("[Chat] Unknown command: {input}");
                println!("[Chat] Available: /quit, /clear, /history, /temp, /tokens, /system");
                continue;
            }
            _ => {}
        }

        print!("AI> ");
        let _ = stdout.flush();

        match engine.generate(&input, Some(max_tokens)) {
            Ok(response) => {
                println!("{response}");
            }
            Err(e) => {
                eprintln!("\n[Chat] Generation error: {e}");
            }
        }
        println!();
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

fn find_arg(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
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

fn print_usage() {
    println!("Vortex Atoms AI — Interactive Chat");
    println!();
    println!("Usage: vortex_chat [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --repo <REPO>          HuggingFace repo");
    println!("  --model <FILE>         GGUF model filename");
    println!("  --tokenizer <FILE>     Tokenizer filename");
    println!("  --system <PROMPT>      System prompt");
    println!("  --max-tokens <N>       Max generation tokens (default: from config or 1024)");
    println!("  --temperature <N>      Sampling temperature (default: from config or 0.7)");
    println!("  --device <TYPE>        Device type: cpu or cuda (default: cpu)");
    println!("  --cuda-device <N>      CUDA device ordinal (default: 0)");
    println!("  --init-config          Generate default vortex.json");
    println!("  --help, -h             Show this help");
    println!();
    println!("Config file: vortex.json (optional, loaded automatically).");
    println!();
    println!("Chat commands:");
    println!("  /quit                  Exit");
    println!("  /clear                 Clear history");
    println!("  /history               Show conversation history");
    println!("  /temp <N>              Change temperature");
    println!("  /tokens <N>            Change max tokens");
    println!("  /system <PROMPT>       Change system prompt");
}
