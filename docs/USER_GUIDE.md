# Vortex Atoms AI - User Guide

## Table of Contents

- [Introduction](#introduction)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Using the Chat](#using-the-chat)
- [Using the Dashboard](#using-the-dashboard)
- [Keyboard Shortcuts](#keyboard-shortcuts)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)

---

## Introduction

Vortex Atoms AI is a local AI assistant with a powerful 5-Kernel Matrix architecture. It provides:

- **Intelligent Chat**: Natural language conversations with streaming responses
- **Knowledge Base**: Import and search your own knowledge
- **Advanced Tools**: Code execution, system info, media rendering
- **Model Management**: Load and swap AI models
- **Privacy-First**: All processing happens locally

---

## Quick Start

### Prerequisites

- **Rust** 1.78+ ([Install](https://rustup.rs/))
- **Node.js** 20+ ([Install](https://nodejs.org/))
- **2GB+ RAM** (4GB recommended)
- **5GB+ disk space** (for models)

### Step 1: Clone the Repository

```bash
git clone https://github.com/mansour2024/vortex_atoms_ai.git
cd vortex_atoms_ai
```

### Step 2: Start the Backend

```bash
cargo run --release --bin vortex_api
```

The server will start on `http://localhost:8080`.

### Step 3: Start the Frontend

```bash
cd frontend
npm install
npm run dev
```

The web interface will be available at `http://localhost:5173`.

### Step 4: Open in Browser

Navigate to `http://localhost:5173` and start chatting!

---

## Installation

### Method 1: From Source

1. Install Rust: https://rustup.rs/
2. Install Node.js: https://nodejs.org/
3. Clone the repository
4. Build and run:

```bash
# Build backend
cargo build --release --bin vortex_api

# Install frontend dependencies
cd frontend
npm install

# Start backend (from project root)
cargo run --release --bin vortex_api

# Start frontend (from frontend directory)
npm run dev
```

### Method 2: Using Docker

1. Install Docker: https://docs.docker.com/get-docker/
2. Clone the repository
3. Build and run:

```bash
# Build and start all services
docker-compose up -d

# View logs
docker-compose logs -f vortex-api

# Stop services
docker-compose down
```

### Method 3: Using Docker (Development)

```bash
# Start with hot-reload for development
docker-compose --profile dev up -d

# View logs
docker-compose logs -f frontend-dev
```

---

## Using the Chat

### Starting a Conversation

1. Open the web interface at `http://localhost:5173`
2. Type your message in the input field
3. Press Enter or click Send

### Example Prompts

- "Write a Python function to calculate fibonacci numbers"
- "Explain the concept of machine learning"
- "What are the benefits of using Rust?"
- "Help me debug this code: [paste code]"

### Features

- **Streaming Response**: See tokens as they're generated
- **Markdown Support**: Responses include formatted text, code blocks, and lists
- **Code Highlighting**: Code blocks are syntax-highlighted
- **Copy to Clipboard**: Click the copy button on any message

### Chat Controls

| Action | Method |
|--------|--------|
| Send message | Enter key |
| New line | Shift+Enter |
| New chat | Click "New" or Ctrl+N |
| Clear chat | Ctrl+Shift+L |
| Regenerate | Click regenerate button on assistant message |

---

## Using the Dashboard

Access the dashboard by clicking the "Dashboard" tab in the sidebar.

### Overview Tab

- **Server Health**: Status, architecture, device info
- **Uptime**: Server uptime counter
- **Knowledge**: Total knowledge chunks loaded

### Models Tab

- **Current View**: View active model details
- **Swap Model**: Switch to a different model
- **Refresh**: Reload model information

### Knowledge Tab

- **Search**: Search your knowledge base
- **Import**: Add new knowledge text
- **Stats**: View knowledge statistics

### Tools Tab

- **List Tools**: View available tools
- **Execute**: Run tools directly
- **View Results**: See tool output

### Logs Tab

- **View Logs**: System and error logs
- **Filter**: Filter by log level
- **Clear**: Clear log history
- **Export**: Export logs to file

### Settings Tab

- **Language**: Switch between Arabic and English
- **Theme**: Light, Dark, or System
- **API URL**: Configure backend URL
- **WebSocket URL**: Configure WebSocket URL
- **Test Connection**: Verify connection to backend
- **Access Tokens** (remote servers only): paste the user/admin tokens from
  the server host. On your own machine tokens are picked up automatically.
  Memory-only — never stored; re-paste after each page reload.

### Security Tab

- **Protection Mode**: auth state, model allowlist, read-only and
  loopback-only-admin badges, live request/denial counters.
- **Admin Token**: rotate the admin token (old one dies immediately).
  Needs the admin token itself.
- **Session Privacy**: on-demand purge of aged chat sessions
  (auto-purge also runs at startup; files are encrypted on Windows).
- **Audit Log**: tail of privileged actions (model swaps, imports, rotations).

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New chat |
| `Ctrl+Enter` | Send message |
| `Ctrl+Shift+L` | Clear chat |
| `Ctrl+Shift+D` | Toggle dark/light mode |
| `Escape` | Close modal/popup |

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VORTEX_HOST` | `127.0.0.1` | Server bind address |
| `VORTEX_PORT` | `8080` | Server port |
| `RUST_LOG` | `info` | Log level |
| `VITE_API_URL` | `http://localhost:8080/v1` | Frontend API URL |
| `VITE_WS_URL` | `ws://localhost:8080/ws` | Frontend WebSocket URL |

### Configuration File (vortex.json)

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 8080
  },
  "llm": {
    "model_path": "models/model.gguf",
    "tokenizer_path": "models/tokenizer.json",
    "device_type": "cpu",
    "max_seq_len": 4096,
    "max_generation_tokens": 2048,
    "system_prompt": "You are Vortex Atoms AI, a helpful assistant.",
    "seed": 42,
    "model_size_params": 500000000,
    "model_quantization": "Q4_K_M",
    "sampling": {
      "temperature": 0.8,
      "top_p": 0.95,
      "repeat_penalty": 1.1,
      "repeat_last_n": 64
    }
  }
}
```

### Loading a Model

1. Download a GGUF model (e.g., from HuggingFace)
2. Place it in the `models/` directory
3. Update `vortex.json` with the model path
4. Restart the server

### Supported Models

- LLaMA (2 & 3)
- Mistral
- Qwen
- Phi
- Gemma
- Any GGUF-compatible model

---

## Troubleshooting

### Server Won't Start

**Problem**: Port already in use

```bash
# Find process using port 8080
# Linux/Mac:
lsof -i :8080
# Windows:
netstat -ano | findstr :8080

# Kill the process or change the port
cargo run --release --bin vortex_api -- --port 8081
```

**Problem**: Model file not found

```
Error: Failed to prepare model: file not found
```

Solution:
1. Check the model path in `vortex.json`
2. Ensure the file exists and is readable
3. Use absolute paths if needed

### Frontend Won't Connect

**Problem**: "Server unreachable" error

Solution:
1. Verify the backend is running: `curl http://localhost:8080/v1/health`
2. Check the API URL in browser settings
3. Ensure no firewall is blocking the connection

### Out of Memory

**Problem**: System runs out of memory

Solution:
1. Use a smaller model
2. Reduce `max_seq_len` in config
3. Close other applications
4. Add swap space if needed

### Slow Responses

**Problem**: Generation is very slow

Solution:
1. Check CPU usage
2. Use a smaller model
3. Reduce `max_generation_tokens`
4. Consider using a CUDA-enabled GPU

---

## FAQ

### Q: Is my data sent to external servers?

**A**: No. All processing happens locally on your machine. Your conversations and data never leave your computer.

### Q: Can I use my own models?

**A**: Yes! Download any GGUF model and update the configuration. The system will automatically download models from HuggingFace if you specify a repo.

### Q: How much RAM do I need?

**A**: Minimum 2GB, recommended 4GB+. Larger models require more RAM. The system uses memory-mapped files to reduce RAM usage.

### Q: Can I use a GPU?

**A**: Yes! Compile with the `cuda` feature:
```bash
cargo build --release --features cuda --bin vortex_api
```

### Q: How do I add custom knowledge?

**A**: Use the Dashboard → Knowledge tab → Import, or use the API:
```bash
curl -X POST http://localhost:8080/v1/knowledge/import \
  -H "Content-Type: application/json" \
  -d '{"text": "Your knowledge here..."}'
```

### Q: Can I use this commercially?

**A**: Yes! The project is licensed under MIT/Apache 2.0. See LICENSE for details.

### Q: How do I update to the latest version?

```bash
git pull origin main
cargo build --release --bin vortex_api
cd frontend && npm install
```

---

## Kiosk Mode (--read-only)

Launch the server in kiosk mode to prevent any state mutations (model swap, knowledge import, config reload, session purge):

```bash
vortex_atoms_ai --read-only
```

This sets `security.read_only=true`. All mutating admin endpoints return 403. The `SecurityTab` UI disables rotate/reload/purge buttons.

## Single-Instance Protection

The server uses a named mutex (`vortex_atoms_ai`) to prevent multiple instances from corrupting shared state. A second launch exits with code 2 and notifies the first instance via `tray_flash`. No manual intervention needed.

## Verify Release Artifacts

After building or downloading release artifacts, verify integrity:

```powershell
# Generate SHA-256 sidecars for all dist/ artifacts
& .\tools\generate_sha256.ps1 -ReleaseDir "dist"

# Verify all artifacts against SBOM and SHA-256 sidecars
& .\tools\verify_release.ps1 -ReleaseDir "dist" -Strict
```

The `SBOM.json` lists all 715 Cargo dependencies with checksums. `PROVENANCE.md` and `provenance.json` document the build provenance chain.

---

## Support

- **GitHub Issues**: Report bugs and request features
- **GitHub Discussions**: Ask questions and share ideas
- **Documentation**: Check the docs folder for more information

---

Thank you for using Vortex Atoms AI!
