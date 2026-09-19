# Vortex Atoms AI - Developer Guide

## Table of Contents

- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Adding Features](#adding-features)
- [Testing](#testing)
- [Code Style](#code-style)
- [Debugging](#debugging)
- [Release Process](#release-process)

---

## Development Setup

### Prerequisites

- **Rust** 1.78+ ([Install](https://rustup.rs/))
- **Node.js** 20+ ([Install](https://nodejs.org/))
- **Git** ([Install](https://git-scm.com/))
- **Python 3.8+** (for knowledge compilation tools)

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/mansour2024/vortex_atoms_ai.git
cd vortex_atoms_ai

# Install frontend dependencies
cd frontend
npm install
cd ..

# Verify Rust installation
rustc --version
cargo --version

# Build the project
cargo build --all-features
```

### GGML Backend (optional, `ggml` feature)

> Windows build prerequisites: **CMake ≥3.21** + **MSVC Build Tools 2022** (Desktop development with C++).
> First `cargo build --features ggml` compiles `llama.cpp` native — **takes 3-8 minutes**, expected. Subsequent builds are incremental.

```powershell
# Check prerequisites
cmake --version   # ≥3.21
cl                # MSVC

# Build with GGML (native llama.cpp)
cargo build --features ggml
cargo test --features ggml
cargo run --features ggml --bin vortex_api -- --simd-probe
```

`Cargo.toml`:
```toml
[dependencies]
llama-cpp-2 = { version = "0.1", default-features = false, features = ["native"], optional = true }
[features]
ggml = ["dep:llama-cpp-2"]
```

`cargo check --no-default-features` and `cargo test` (without `ggml`) stay green — `ggml` is opt-in. See `src/inference/mod.rs` for `InferenceBackend` trait.

### IDE Configuration

#### VS Code Extensions

Recommended extensions:
- `rust-lang.rust-analyzer` — Rust language support
- `vadimcn.vscode-lldb` — Debugging
- `bradlc.vscode-tailwindcss` — Tailwind CSS support
- `esbenp.prettier-vscode` — Code formatting
- `dbaeumer.vscode-eslint` — Linting

#### Rust Analyzer Settings

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

---

## Project Structure

```
vortex_atoms_ai/
├── Cargo.toml              # Rust workspace manifest
├── Cargo.lock              # Dependency lock file
├── vortex.json             # Runtime configuration
├── Dockerfile              # Docker build file
├── docker-compose.yml      # Docker Compose configuration
├── .github/                # GitHub Actions workflows
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── docs/                   # Documentation
│   ├── API_REFERENCE.md
│   ├── USER_GUIDE.md
│   ├── DEVELOPER_GUIDE.md
│   └── ARCHITECTURE.md
├── frontend/               # React frontend
│   ├── src/
│   │   ├── components/     # UI components
│   │   │   ├── ui/         # Reusable UI components
│   │   │   ├── chat/       # Chat components
│   │   │   ├── dashboard/  # Dashboard components
│   │   │   └── layout/     # Layout components
│   │   ├── hooks/          # React hooks
│   │   ├── stores/         # Zustand stores
│   │   ├── services/       # API and WebSocket services
│   │   ├── types/          # TypeScript types
│   │   ├── i18n/           # Internationalization
│   │   └── utils/          # Utility functions
│   ├── public/             # Static assets
│   ├── package.json        # Node dependencies
│   ├── vite.config.ts      # Vite configuration
│   ├── vitest.config.ts    # Vitest configuration
│   └── tailwind.config.js  # Tailwind configuration
├── knowledge/              # Knowledge modules
│   ├── bio_medical_module.tcz
│   ├── engineering_expert_module.tcz
│   ├── finance_math_module.tcz
│   └── global_humanities_module.bin
├── src/                    # Rust source code
│   ├── bin/                # Binary entry points
│   │   ├── vortex_api.rs   # HTTP API server
│   │   ├── vortex_chat.rs  # CLI chat client
│   │   └── vortex_dashboard.rs # Native dashboard
│   ├── kernel/             # Kernel implementations
│   ├── llm/                # LLM-related modules
│   │   ├── llm_api.rs      # API server
│   │   ├── llm_config.rs   # Configuration
│   │   ├── llm_inference.rs # Inference engine
│   │   ├── llm_model.rs    # Model management
│   │   ├── llm_tools.rs    # Tool system
│   │   └── ...
│   ├── state.rs            # Shared state
│   ├── ikc.rs              # Inter-kernel communication
│   └── error.rs            # Error types
├── tests/                  # Integration tests
│   └── integration_tests.rs
└── tools/                  # Build tools
    ├── compile_bio_medical_module.py
    ├── compile_engineering_expert_module.py
    ├── compile_finance_math_module.py
    └── compile_global_humanities_module.py
```

---

## Development Workflow

### Running the Development Environment

#### Terminal 1: Backend

```bash
# Run with hot-reload (requires cargo-watch)
cargo watch -x 'run --bin vortex_api'

# Or run directly
cargo run --bin vortex_api
```

#### Terminal 2: Frontend

```bash
cd frontend
npm run dev
```

#### Terminal 3: Tests (optional)

```bash
# Backend tests
cargo test --all-features

# Frontend tests
cd frontend
npm run test:watch
```

### Making Changes

1. **Create a branch**: `git checkout -b feature/your-feature`
2. **Make changes**: Edit code
3. **Test locally**: Run tests
4. **Commit**: `git commit -m "feat: add new feature"`
5. **Push**: `git push origin feature/your-feature`
6. **Create PR**: Open a pull request

---

## Adding Features

### Adding a New API Endpoint

1. Define request/response types in `src/types.rs`:

```rust
#[derive(Serialize, Deserialize)]
pub struct MyRequest {
    pub field: String,
}

#[derive(Serialize, Deserialize)]
pub struct MyResponse {
    pub result: String,
}
```

2. Add handler in `src/llm_api.rs`:

```rust
async fn handle_my_endpoint(
    Json(req): Json<MyRequest>,
) -> impl IntoResponse {
    // Implementation
    let result = process_request(&req).await;
    (StatusCode::OK, Json(MyResponse { result }))
}
```

3. Register route:

```rust
.route("/v1/my-endpoint", post(handle_my_endpoint))
```

4. Add tests:

```rust
#[tokio::test]
async fn test_my_endpoint() {
    // Test implementation
}
```

### Adding a New Knowledge Module

1. Create source file: `knowledge/my_module.source.md`
2. Create compilation script: `tools/compile_my_module.py`
3. Add module descriptor in config
4. Add intent detection in Kernel_02
5. Test the module

### Adding a New Tool

1. Define tool in `src/llm_tools.rs`:

```rust
pub fn tool_definitions_prompt() -> String {
    // Add tool definition
    r#"{
        "name": "my_tool",
        "description": "What my tool does",
        "parameters": {
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        }
    }"#.to_string()
}
```

2. Implement tool execution:

```rust
pub async fn execute_my_tool(args: serde_json::Value) -> Result<String> {
    // Implementation
    Ok("Tool result".to_string())
}
```

3. Add to tool router:

```rust
match tool_name {
    "my_tool" => execute_my_tool(arguments).await,
    // ...
}
```

### Adding a New Frontend Component

1. Create component file: `src/components/ui/MyComponent.tsx`
2. Export from `src/components/ui/index.ts`
3. Add tests: `src/components/ui/__tests__/MyComponent.test.tsx`
4. Use in parent components

---

## Testing

### Backend Tests

```bash
# Run all tests
cargo test --all-features

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run with logging
RUST_LOG=debug cargo test
```

### Frontend Tests

```bash
cd frontend

# Run all tests
npm run test

# Run with coverage
npm run test:coverage

# Run in watch mode
npm run test:watch

# Run specific test
npm run test -- MyComponent
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_tests

# Run with all features
cargo test --all-features --test integration_tests
```

### Writing Good Tests

#### Backend Test Example

```rust
#[tokio::test]
async fn test_health_endpoint() {
    // Arrange
    let app = create_test_app();
    
    // Act
    let response = app.oneshot(
        Request::builder()
            .uri("/v1/health")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();
    
    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}
```

#### Frontend Test Example

```typescript
import { render, screen } from '@testing-library/react'
import { MyComponent } from './MyComponent'

describe('MyComponent', () => {
  it('renders correctly', () => {
    render(<MyComponent title="Test" />)
    expect(screen.getByText('Test')).toBeInTheDocument()
  })
})
```

---

## Code Style

### Rust

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document all public APIs with `///` comments
- Use `Result` for fallible operations
- Prefer `&str` over `String` for function parameters

```rust
/// Processes user input and returns a response
///
/// # Arguments
/// * `input` - The user's input text
/// * `config` - The processing configuration
///
/// # Returns
/// A Result containing the response or an error
///
/// # Example
/// ```
/// let response = process_input("Hello", &config)?;
/// ```
pub fn process_input(input: &str, config: &Config) -> Result<String> {
    // Implementation
}
```

### TypeScript/React

- Use functional components with hooks
- Use TypeScript strict mode
- Follow the existing component structure
- Use `clsx` for conditional classes
- Export components from index files

```typescript
interface MyComponentProps {
  title: string;
  description?: string;
  onSubmit: (data: FormData) => void;
}

export function MyComponent({ title, description, onSubmit }: MyComponentProps) {
  // Implementation
}
```

---

## Debugging

### Backend Debugging

#### Using println!

```rust
println!("Debug: variable = {:?}", variable);
```

#### Using logging

```rust
log::info!("Processing input: {}", input);
log::debug!("State: {:?}", state);
log::error!("Error: {}", error);
```

Run with logging:
```bash
RUST_LOG=debug cargo run --bin vortex_api
```

#### Using LLDB

```bash
# Build with debug info
cargo build --bin vortex_api

# Debug
cargo run --bin vortex_api -- --debug
```

### Frontend Debugging

#### React DevTools

Install React DevTools browser extension for component inspection.

#### Console Logging

```typescript
console.log('Debug:', data);
console.table(arrayData);
console.time('operation');
```

#### Network Inspection

Use browser DevTools Network tab to inspect API calls and WebSocket messages.

---

## Coverage Notes

The frontend suite is kept at 99.49% statements / 99.4% branches / 99.42% functions / 99.88% lines.
The only uncovered lines are intentionally unreachable guard branches and loader artifacts that cannot
be exercised by authentic user behaviour; they are documented here instead of being force-covered with
mocks:

| Location | Why it stays uncovered |
|----------|------------------------|
| `src/App.tsx:11-12` | `lazy()` module-loader arrows (`ChatInterface`/`Dashboard`); they fire only when React resolves the chunk at runtime, which would require a heavy full-app integration render. |
| `src/components/chat/ChatInput.tsx:17` | `if (textareaRef.current)` falsy tower — the ref is always attached before `useEffect` runs; unreachable in real usage. |
| `src/components/dashboard/Dashboard.tsx:37` | V8 statement boundary on the component's `return (`; the component is rendered directly by `dashboard.test.tsx`. |
| `src/components/dashboard/ModelsTab.tsx:22` | `if (!selectedModel) return;` empty-selection tower — the swap button is `disabled={!selectedModel}`, so the path is UI-unreachable. |
| `src/components/dashboard/ToolsTab.tsx:19` | `if (!selectedTool) return;` — the execute modal only renders when a tool is selected. |

Policy: coverage gaps are closed with real interaction tests whenever the behaviour is reachable in
the UI; guards that are structurally unreachable are documented here rather than mocked.

---

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backwards-compatible)
- **PATCH**: Bug fixes (backwards-compatible)

### Creating a Release

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create a tag: `git tag v0.2.0`
4. Push tag: `git push origin v0.2.0`
5. GitHub Actions will create the release automatically

### Release Checklist

- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] Frontend builds successfully
- [ ] Docker image builds successfully

---

## Performance Profiling

### Backend Profiling

```bash
# Build with profiling
cargo build --release --features profiling

# Run with perf (Linux)
perf record -g ./target/release/vortex_api
perf report
```

### Frontend Profiling

Use Chrome DevTools Performance tab to profile React components.

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

---

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Documentation](https://tokio.rs/)
- [React Documentation](https://react.dev/)
- [Tailwind CSS Documentation](https://tailwindcss.com/docs)
- [Candle Documentation](https://github.com/huggingface/candle)
