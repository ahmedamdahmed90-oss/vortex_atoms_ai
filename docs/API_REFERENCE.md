# Vortex Atoms AI - API Reference

## Base URL

```
http://localhost:8080/v1
```

## Authentication

Bearer-token auth with two least-privilege roles. Tokens are auto-generated
on first run and stored in `%LOCALAPPDATA%\vortex_atoms_ai\auth.json`
(user-profile scoped); explicit values can be set via `vortex.json`
(`auth.api_token` / `auth.admin_token`) or `VORTEX_API_TOKEN` /
`VORTEX_ADMIN_TOKEN`.

| Role | Token prefix | May call |
|------|--------------|----------|
| `user` | `vxt_` | generate, chat, batch, embeddings, knowledge/search, tools, models (read), device, ws |
| `admin` | `vxa_` | everything, plus models/swap, knowledge/import, /v1/admin/* |

Send it as `Authorization: Bearer <token>` (WebSocket: `ws://host/ws?token=<token>`).
The bundled web UI fetches both tokens automatically from the loopback-only
`GET /auth/bootstrap` endpoint. `GET /v1/health` stays public (liveness).

```bash
# read the user token (printed at startup, or from auth.json)
curl http://localhost:8080/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer vxt_..."
```

---

## Endpoints

### Health & Status

#### GET /health

Returns the health status of the server.

**Response:**
```json
{
  "status": "ok",
  "architecture": "llama",
  "device": "Cpu",
  "simd": "AVX + SSE4.2 + SSE4.1 + SSSE3 + SSE2",
  "history_length": 0,
  "uptime_seconds": 120,
  "knowledge_chunks": 10
}
```

| Field | Type | Description |
|-------|------|-------------|
| status | string | Server status ("ok" or error) |
| architecture | string | Model architecture (e.g., "llama") |
| device | string | Compute device ("Cpu" or "Cuda") |
| simd | string | SIMD instruction set support |
| history_length | number | Number of conversations in history |
| uptime_seconds | number | Server uptime in seconds |
| knowledge_chunks | number | Total knowledge chunks loaded |

---

#### GET /device

Returns device information.

**Response:**
```json
{
  "device": "Cpu",
  "simd": "AVX + SSE4.2 + SSE4.1 + SSSE3 + SSE2",
  "compiled_features": ["avx", "sse4.2", "sse4.1", "ssse3"]
}
```

`simd` is detected on the host; `compiled_features` is encoded into the
running SKU (`["sse2"]` for the safe default baseline).

---

### Models

#### GET /models

Returns information about the currently loaded model, the CPU tier, the
tier-aware generation defaults, and the pre-configured economy/quality model
matrix.

**Response (PERF-01 Section 3):**
```json
{
  "architecture": "qwen2",
  "max_seq_len": 4096,
  "max_generation_tokens": 512,
  "model_routing": false,
  "generation_defaults": {
    "model": "eco",
    "temperature": 0.0,
    "max_context": 1024,
    "kv_cache": "f16",
    "tier": "low",
    "repeat_penalty": 1.1
  },
  "models": [
    {"name": "default", "repo": "Qwen/Qwen2.5-0.5B-Instruct-GGUF", "file": "qwen2.5-0.5b-instruct-q4_k_m.gguf", "architecture": "qwen2", "size_params": 500000000, "quant": "Q4_K_M", "max_seq_len": 4096},
    {"name": "eco", "repo": "HuggingFaceTB/SmolLM2-135M-Instruct-GGUF", "file": "smollm2-135m-instruct-q4_0.gguf", "architecture": "llama", "size_params": 135000000, "quant": "Q4_0", "max_seq_len": 2048},
    {"name": "q4_0", "repo": "Qwen/Qwen2.5-0.5B-Instruct-GGUF", "file": "qwen2.5-0.5b-instruct-q4_0.gguf", "architecture": "qwen2", "size_params": 500000000, "quant": "Q4_0", "max_seq_len": 4096}
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| architecture | string | Model architecture of the active engine |
| max_seq_len | number | Context budget of the active engine |
| max_generation_tokens | number | Maximum tokens per generation (512 API ceiling) |
| model_routing | boolean | Whether `model` routing hints are honored (`security.allow_model_routing`) |
| generation_defaults.model | string | Matrix name suggested as default (`eco` on low tier, `q4_0` otherwise) |
| generation_defaults.temperature | number | Default sampling temperature (0.0 = greedy fast path on low tier) |
| generation_defaults.max_context | number | Default context budget (1024 on low tier, 4096 otherwise) |
| generation_defaults.kv_cache | string | KV-cache dtype hint (`f16`) |
| generation_defaults.tier | string | Detected CPU tier (`low` / `standard`) |
| generation_defaults.repeat_penalty | number | Default repeat penalty |
| models[].name | string | Matrix entry name (`default`, `eco`, `q4_0`) |
| models[].repo | string | HuggingFace repo |
| models[].file | string | GGUF filename |
| models[].architecture | string | Model architecture |
| models[].size_params | number | Approximate parameter count |
| models[].quant | string | Quantization scheme |
| models[].max_seq_len | number | Advertised context budget |

---

#### POST /models/swap

Swap the currently loaded model.

**Request:**
```json
{
  "model": "qwen2.5-0.5b-instruct",
  "repo": "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
  "tokenizer": "tokenizer.json",
  "device": "cpu",
  "cuda_device": 0
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| model | string | No | Model filename |
| repo | string | No | HuggingFace repo |
| tokenizer | string | No | Tokenizer filename |
| device | string | No | Device type ("cpu" or "cuda") |
| cuda_device | number | No | CUDA device ordinal |

**Response:**
```json
{
  "status": "ok",
  "architecture": "llama",
  "device": "Cpu"
}
```

#### Model routing (PERF-01 Section 3)

`/generate` and `/chat` accept an optional `model` hint naming a
pre-configured matrix entry (`eco` — SmolLM2-135M Q4_0, or `q4_0` —
Qwen2.5-0.5B Q4_0). Routing is **fail-closed**: unless
`security.allow_model_routing` is `true` in `vortex.json`, any `model` hint
returns `400 model routing is disabled...`. When enabled, the named model is
ensured on disk (SHA-256 manifest verify) and swapped in for the request; the
swap persists and is recorded in the audit log as `models.route`, so an
operator can always see who switched runtime models. Unknown names return
`400`. `/v1/models` advertises the matrix and the tier-aware
`generation_defaults` (low tier defaults to greedy `temperature: 0.0`, the
allocation-light fast path, with a 1024-token context).

---

### Chat

#### POST /chat

Send a chat message and get a response.

> **Stateless per request:** the server rebuilds conversation history from
> the `messages` array on every call. No history carries over between
> requests or clients — send the full conversation each time.

**Request:**
```json
{
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "max_tokens": 1024,
  "temperature": 0.7,
  "stream": false
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| messages | array | Yes | Chat messages array |
| max_tokens | number | No | Maximum tokens to generate |
| temperature | number | No | Sampling temperature (0-2) |
| model | string | No | Routing hint (`eco`/`q4_0`); requires `security.allow_model_routing` |
| stream | boolean | No | Enable streaming (not yet supported) |

**Response:**
```json
{
  "text": "Hello! How can I help you today?",
  "usage": {
    "prompt_tokens": 15,
    "completion_tokens": 10,
    "total_tokens": 25
  }
}
```

---

### Generation

#### POST /generate

Generate text from a prompt.

> **Stateless:** each call starts with a cleared history — prior requests
> (including other clients') never influence the output.

**Request:**
```json
{
  "prompt": "Write a Python function to calculate fibonacci",
  "max_tokens": 2048,
  "temperature": 0.8
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| prompt | string | Yes | Input prompt |
| max_tokens | number | No | Maximum tokens to generate |
| temperature | number | No | Sampling temperature (0-2) |
| model | string | No | Routing hint (`eco`/`q4_0`); requires `security.allow_model_routing` |

**Response:**
```json
{
  "text": "def fibonacci(n):\n    if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)",
  "total_tokens": 45,
  "tokens_per_second": 12.5
}
```

---

#### POST /batch

Generate text for multiple prompts.

> **Isolated items:** history is reset per item, so batch entries cannot
> observe each other through the shared engine.

**Request:**
```json
{
  "prompts": ["Prompt 1", "Prompt 2", "Prompt 3"],
  "max_tokens": 512
}
```

**Response:**
```json
{
  "results": ["Result 1", "Result 2", "Result 3"],
  "total_tokens": 150
}
```

---

### Embeddings

#### POST /embeddings

Generate embeddings for input texts.

**Request:**
```json
{
  "texts": ["Hello world", "How are you?"]
}
```

**Response:**
```json
{
  "embeddings": [[0.1, 0.2, ...], [0.3, 0.4, ...]],
  "dimensions": 384,
  "model": "embedding-model",
  "usage": {
    "prompt_tokens": 10
  }
}
```

---

### Knowledge Base

#### POST /knowledge/search

Search the knowledge base.

**Request:**
```json
{
  "query": "What is machine learning?",
  "limit": 10
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| query | string | Yes | Search query |
| limit | number | No | Maximum results (default: 10) |

**Response:**
```json
{
  "results": [
    {
      "id": "chunk_001",
      "score": 0.89,
      "text": "Machine learning is a subset of AI..."
    }
  ]
}
```

---

#### POST /knowledge/import

Import knowledge text into the knowledge base.

**Request:**
```json
{
  "text": "Your knowledge text here..."
}
```

**Response:**
```json
{
  "chunks_imported": 5,
  "status": "ok"
}
```

---

### Tools

#### GET /tools

List available tools.

**Response:**
```json
{
  "tools": [
    {
      "name": "execute_code",
      "description": "Execute a Rust or Python code snippet and return the result."
    },
    {
      "name": "search_knowledge",
      "description": "Search the knowledge base for relevant information."
    },
    {
      "name": "render_media",
      "description": "Generate or render media content (image, audio, video)."
    },
    {
      "name": "manage_memory",
      "description": "Manage conversation memory: list, clear, or summarize."
    },
    {
      "name": "system_info",
      "description": "Get system information (OS, architecture, CPU cores)."
    }
  ]
}
```

---

#### POST /tools/execute

Execute a tool directly.

**Request:**
```json
{
  "name": "system_info",
  "arguments": {}
}
```

**Response:**
```json
{
  "result": "OS: Windows 11, Arch: x86_64, Cores: 8"
}
```

---

#### POST /tools/call

Execute a tool through the LLM (tool calling).

**Request:**
```json
{
  "prompt": "What is my system info?",
  "max_tokens": 512
}
```

**Response:**
```json
{
  "text": "Your system information is...",
  "tool_calls": [
    {
      "name": "system_info",
      "arguments": {},
      "result": "OS: Windows 11, Arch: x86_64, Cores: 8"
    }
  ]
}
```

---

## WebSocket API

### Connection

```
ws://localhost:8080/ws
```

### Client → Server Messages

#### Generate Request

```json
{
  "type": "generate",
  "prompt": "Hello, world!",
  "max_tokens": 512,
  "temperature": 0.7
}
```

#### Ping

```json
{
  "type": "ping",
  "id": 1,
  "timestamp": 1700000000000
}
```

---

### Server → Client Messages

#### Ready

```json
{
  "type": "ready"
}
```

#### Token (Streaming)

```json
{
  "type": "token",
  "token_id": 42,
  "text": "Hello",
  "position": 0
}
```

#### Done

```json
{
  "type": "done",
  "total_tokens": 50,
  "tokens_per_second": 15.2,
  "full_text": "Hello, world! How can I help you?"
}
```

#### Error

```json
{
  "type": "error",
  "message": "Failed to generate: model not loaded"
}
```

#### Pong

```json
{
  "type": "pong",
  "id": 1,
  "timestamp": 1700000000000
}
```

---

## Error Responses

All endpoints return errors in the following format:

```json
{
  "error": "Description of the error"
}
```

### Admin Control Plane (admin token required)

#### GET /v1/admin/status

Effective security posture (tokens are never included).

#### GET /v1/admin/audit?lines=50

Tail of the privileged-action audit log (model swaps, knowledge imports,
token rotations) as JSON lines.

#### POST /v1/admin/rotate

Regenerates the admin token immediately (old one dies) and returns the new
value. Audited.

#### POST /v1/admin/reload

Re-reads `vortex.json` and hot-swaps the hardening knobs (rate limits, caps,
allowlists, read-only, retention). Tokens and CORS origins need a restart.
Audited.

#### GET /v1/admin/metrics

Request/denial counters (`requests_total`, `denied_401/403/429`) + uptime.

#### POST /v1/admin/sessions/purge

Deletes chat sessions older than `{"days": N}` (default: configured
retention). Returns `{status, removed}`. Audited.

### Unknown API Paths

Any unknown `/v1/*`, `/ws/*` or `/auth/*` path returns `404
{"error": "not found"}` as JSON. Only non-API paths fall through to the
embedded SPA shell (200 + `index.html`), so API clients can never mistake
the UI for a successful call.

#### GET /auth/bootstrap

Loopback-only: returns `{auth_enabled, api_token, admin_token}` to the
bundled same-origin UI. Remote peers get 403.

### HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 400 | Bad Request (invalid JSON / failed validation) |
| 401 | Missing or invalid API token |
| 403 | Admin token required (or loopback-only endpoint) |
| 404 | Not Found |
| 408 | Request Timeout |
| 429 | Too Many Requests (per-IP rate limit) |
| 500 | Internal Server Error |
| 502 | Bad Gateway |
| 503 | Service Unavailable (engine busy — back off, do not hammer) |

---

## Examples

### cURL Examples

```bash
TOKEN="vxt_..."   # from server stdout or %LOCALAPPDATA%\vortex_atoms_ai\auth.json
ADMIN="vxa_..."   # admin token for privileged endpoints

# Health check (public)
curl http://localhost:8080/v1/health

# Generate text
curl -X POST http://localhost:8080/v1/generate \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"prompt": "Hello, world!", "max_tokens": 100}'

# Chat
curl -X POST http://localhost:8080/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"messages": [{"role": "user", "content": "Hello!"}]}'

# Search knowledge
curl -X POST http://localhost:8080/v1/knowledge/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "machine learning", "limit": 5}'

# List tools
curl http://localhost:8080/v1/tools -H "Authorization: Bearer $TOKEN"

# Execute tool
curl -X POST http://localhost:8080/v1/tools/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "system_info", "arguments": {}}'

# Swap model (admin only, allowlisted repos unless opted in)
curl -X POST http://localhost:8080/v1/models/swap \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN" \
  -d '{"repo": "Qwen/Qwen2.5-1.5B-Instruct-GGUF", "model": "qwen2.5-1.5b-instruct-q4_k_m.gguf"}'
```

### JavaScript/TypeScript Examples

```typescript
// Generate text
const response = await fetch('http://localhost:8080/v1/generate', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    prompt: 'Write a Python function',
    max_tokens: 512,
    temperature: 0.7
  })
});
const data = await response.json();
console.log(data.text);

// WebSocket connection (token as query param — browsers can't set headers)
const ws = new WebSocket(`ws://localhost:8080/ws?token=${TOKEN}`);
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'generate',
    prompt: 'Hello!',
    max_tokens: 100
  }));
};
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'token') {
    process.stdout.write(data.text);
  }
};
```

### Python Examples

```python
import requests

# Health check
response = requests.get('http://localhost:8080/v1/health')
print(response.json())

# Generate text
response = requests.post('http://localhost:8080/v1/generate', json={
    'prompt': 'Hello, world!',
    'max_tokens': 100
})
print(response.json()['text'])

# Search knowledge
response = requests.post('http://localhost:8080/v1/knowledge/search', json={
    'query': 'machine learning',
    'limit': 5
})
for result in response.json()['results']:
    print(f"Score: {result['score']}, Text: {result['text']}")
```

---

## Rate Limiting

Per-IP fixed-window limiting (default 600 requests / 10 s, configurable via
`security.rate_limit_per_10s`); excess calls get `429`. Inference itself is
additionally admission-controlled (2 concurrent jobs, `503` fail-fast on
overflow — back off instead of retrying). Every response carries an
`x-request-id` header for log correlation.

---

## Versioning

The API follows semantic versioning. The current version is `v1`.

Future versions will be available at `/v2`, `/v3`, etc.
