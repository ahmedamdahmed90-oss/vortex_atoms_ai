# Vortex Atoms AI - Architecture Documentation

## Overview

Vortex Atoms AI is built on a **5-Kernel Matrix** architecture — a system of independent, specialized processing kernels that work together to provide AI capabilities.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (React)                         │
│  ┌─────────┐  ┌──────────┐  ┌───────────┐  ┌───────────┐  │
│  │  Chat   │  │Dashboard │  │  Settings │  │  Sidebar  │  │
│  │Interface│  │   Tabs   │  │   Panel   │  │           │  │
│  └─────────┘  └──────────┘  └───────────┘  └───────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP / WebSocket
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     API Server (Axum)                        │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    Router Layer                          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   5-Kernel Matrix                            │
│                                                              │
│  ┌─────────────────┐    ┌─────────────────┐                 │
│  │   Kernel_01     │    │   Kernel_02     │                 │
│  │  UI Interaction │◄──►│ Router & Vector │                 │
│  │                 │    │       DB        │                 │
│  └─────────────────┘    └─────────────────┘                 │
│           │                      │                           │
│           ▼                      ▼                           │
│  ┌─────────────────┐    ┌─────────────────┐                 │
│  │   Kernel_03     │    │   Kernel_04     │                 │
│  │ Code & Logic    │    │ Multimodal &    │                 │
│  │    Expert       │    │     Media       │                 │
│  └─────────────────┘    └─────────────────┘                 │
│           │                      │                           │
│           └──────────┬───────────┘                           │
│                      ▼                                       │
│           ┌─────────────────┐                               │
│           │   Kernel_05     │                               │
│           │  Supervisor &   │                               │
│           │   Watchdog      │                               │
│           └─────────────────┘                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Knowledge Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Bio     │  │Engineering│  │ Finance  │  │Humanities│   │
│  │ Medical  │  │  Expert   │  │  & Math  │  │  Module  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 5-Kernel Matrix

### Kernel_01: UI & Interaction

**Responsibilities:**
- Receive user input from the frontend
- Route requests to appropriate kernels
- Manage conversation state
- Handle WebSocket connections

**Implementation:**
- Tokio mpsc channels for incoming messages
- Broadcast events for status updates
- Session management

### Kernel_02: Router & Vector DB

**Responsibilities:**
- Intent detection and semantic routing
- Knowledge base search
- Model selection and loading
- Knowledge fragment management

**Implementation:**
- Qdrant-compatible in-memory vector database
- Cosine similarity for semantic search
- Knowledge fragment descriptors (.tcz, .bin files)
- Hot token cache for frequently accessed data

### Kernel_03: Code & Logic Expert

**Responsibilities:**
- Code generation and analysis
- Logic module execution
- Knowledge fragment loading and querying
- Tool execution

**Implementation:**
- Memory-mapped knowledge files (memmap2)
- Length-prefixed binary section parsing
- Explicit memory drop after use
- LLM inference via Candle

### Kernel_04: Multimodal & Media

**Responsibilities:**
- Image generation and processing
- Audio processing
- Video frame rendering
- Media content handling

**Implementation:**
- Frame-by-frame processing pipeline
- Media format encoding/decoding
- Streaming output for real-time media

### Kernel_05: Supervisor & Watchdog

**Responsibilities:**
- System health monitoring
- Memory management
- Knowledge fragment cleanup
- Inactive data purging

**Implementation:**
- Periodic health checks
- Explicit `std::mem::drop` on removed fragments
- Resource usage tracking
- Automatic cleanup triggers

---

## Inter-Kernel Communication

### Communication Patterns

```
┌─────────────┐     mpsc channel      ┌─────────────┐
│  Kernel_01  │ ─────────────────────►│  Kernel_02  │
│             │◄─────────────────────│             │
└─────────────┘     mpsc channel      └─────────────┘

┌─────────────┐     broadcast event    ┌─────────────┐
│  Kernel_02  │ ─────────────────────►│  Kernel_03  │
│             │                       │             │
└─────────────┘                       └─────────────┘
```

### Message Types

```rust
enum KernelCommand {
    Shutdown,
    ProcessInput { session_id: String, input: String },
    SearchKnowledge { query: String, limit: usize },
    LoadModel { path: String },
    ExecuteTool { name: String, arguments: serde_json::Value },
    PurgeInactive { threshold: Duration },
}

struct IkcMessage {
    source: KernelId,
    target: KernelId,
    command: KernelCommand,
    timestamp: u64,
}

enum IkcEvent {
    Ready,
    ProcessingComplete { result: String },
    Error { message: String },
    KnowledgeUpdated { chunks: usize },
}
```

### Shared State

```rust
struct VortexAtomsSharedState {
    current_model: Option<ModelInfo>,
    knowledge_index: VectorStore,
    conversation_history: HashMap<String, Vec<Message>>,
    system_metrics: SystemMetrics,
}
```

Protected with `Arc<RwLock<VortexAtomsSharedState>>` for safe concurrent access.

---

## Knowledge System

### Knowledge Fragment Format

Knowledge is stored in compressed binary fragments:

| Format | Extension | Description |
|--------|-----------|-------------|
| TCZ | .tcz | Compressed text with metadata |
| BIN | .bin | Raw binary knowledge data |
| Source | .source.md | Human-readable source |

### Knowledge Fragment Descriptor

```rust
struct KnowledgeFragmentDescriptor {
    id: String,           // Unique identifier
    module_type: String,  // "bio.medical", "finance.math", etc.
    semantic_hints: String, // Keywords for semantic matching
    file_path: String,    // Path to the fragment file
}
```

### Knowledge Loading Flow

```
1. User submits query
        │
        ▼
2. Kernel_02 detects intent via vector similarity
        │
        ▼
3. Matching fragment descriptor found
        │
        ▼
4. Kernel_03 loads fragment via mmap
        │
        ▼
5. Query answered from mapped data
        │
        ▼
6. Fragment explicitly dropped (std::mem::drop)
```

---

## Model Management

### Supported Model Formats

- **GGUF**: Primary format (llama.cpp compatible)
- **Safetensors**: Via Candle conversion
- **ONNX**: Via ort crate (optional feature)

### Model Loading

```rust
struct ModelInfo {
    architecture: String,      // "llama", "mistral", etc.
    max_seq_len: usize,        // Maximum sequence length
    max_generation_tokens: usize, // Max tokens per generation
}

// Model loading via memory mapping
let kernel = VortexAtomsKernel::new(KernelConfig::default(), "weights.bin")?;
let tensor = kernel.load_tensor(&spec)?;
```

### Model Configuration

```json
{
  "llm": {
    "model_path": "models/model.gguf",
    "tokenizer_path": "models/tokenizer.json",
    "device_type": "cpu",
    "max_seq_len": 4096,
    "max_generation_tokens": 2048,
    "sampling": {
      "temperature": 0.8,
      "top_p": 0.95,
      "repeat_penalty": 1.1,
      "repeat_last_n": 64
    }
  }
}
```

---

## Frontend Architecture

### Component Hierarchy

```
App
├── BrowserRouter
│   └── Routes
│       └── Route [/*]
│           └── AppRoutes
│               ├── ToastProvider
│               ├── ErrorBoundary
│               └── RTLLayout
│                   ├── Sidebar
│                   │   └── Nav tabs (Chat/Dashboard)
│                   ├── Header
│                   │   └── Connection status
│                   └── Main Content
│                       ├── ChatInterface (lazy)
│                       │   ├── MessageBubble[]
│                       │   └── ChatInput
│                       └── Dashboard (lazy)
│                           ├── Tabs[]
│                           └── TabContent
└── KeyboardShortcutsHelp
```

### State Management

Uses Zustand for state management with persistence:

```typescript
// Chat Store
interface ChatState {
  messages: Message[];
  currentMessage: string;
  isStreaming: boolean;
  // ...
}

// Settings Store
interface SettingsState {
  language: 'ar' | 'en';
  theme: 'light' | 'dark' | 'system';
  apiUrl: string;
  wsUrl: string;
}
```

### Performance Optimizations

- **Code Splitting**: Lazy loading for Chat and Dashboard
- **Memoization**: React.memo for expensive components
- **Virtual Scrolling**: For long message lists
- **Debounced Input**: For search and settings

---

## Security Considerations

### Current Limitations

1. **No Authentication**: Designed for local use only
2. **No Encryption**: All communication is plaintext (localhost)
3. **No Rate Limiting**: Not suitable for public exposure

### Security Best Practices

1. **Keep on localhost**: Do not expose to the internet without a reverse proxy
2. **Use firewall**: Block external access to port 8080
3. **Update regularly**: Keep dependencies updated for security patches
4. **Monitor logs**: Watch for suspicious activity

### Future Security Features

- [ ] JWT-based authentication
- [ ] HTTPS support
- [ ] Rate limiting
- [ ] Input validation and sanitization
- [ ] Audit logging

---

## Deployment Options

### Single Binary

```bash
# Build optimized binary
cargo build --release --features cuda --bin vortex_api

# Run
./target/release/vortex_api --host 0.0.0.0 --port 8080
```

### Docker

```bash
# Build image
docker build -t vortex-atoms-ai .

# Run container
docker run -p 8080:8080 -v ./models:/app/models vortex-atoms-ai
```

### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f
```

---

## Performance Tuning

### Backend

| Setting | Default | Recommendation |
|---------|---------|----------------|
| `max_seq_len` | 4096 | Reduce for less RAM |
| `max_generation_tokens` | 2048 | Adjust per use case |
| `temperature` | 0.8 | Lower for more focused output |
| `LTO` | thin | Enable for release builds |

### Frontend

- Enable animations disable for low-end devices
- Use compact mode for smaller screens
- Limit message history for better performance

---

## Extending the System

### Adding a New Knowledge Module

1. Create source file: `knowledge/my_module.source.md`
2. Compile to binary: `python3 tools/compile_my_module.py`
3. Register in `vortex.json`
4. Add intent detection in Kernel_02

### Adding a New Tool

1. Define tool in `src/llm_tools.rs`
2. Implement tool logic
3. Add tool definition to `tool_definitions_prompt()`
4. Test via `/v1/tools/execute`

### Adding a New Endpoint

1. Add route in `src/llm_api.rs`
2. Define request/response types
3. Implement handler logic
4. Add tests

---

## Glossary

| Term | Definition |
|------|------------|
| Kernel | Independent processing unit in the 5-Kernel Matrix |
| TCZ | Text Compressed format for knowledge fragments |
| GGUF | GPT-Generated Unified Format for LLM models |
| IKC | Inter-Kernel Communication |
| Vector Store | In-memory vector database for semantic search |
| Memory Mapping | Technique to access files without loading into RAM |
| Token Cache | Cache for frequently accessed tokenized data |
