# Contributing to Vortex Atoms AI

Thank you for your interest in contributing to Vortex Atoms AI! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/vortex_atoms_ai.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Test your changes
6. Submit a Pull Request

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check the [existing issues](../../issues) to avoid duplicates. When creating a bug report, include:

- **Clear title and description**
- **Steps to reproduce**
- **Expected behavior**
- **Actual behavior**
- **Environment details** (OS, Rust version, Node version)
- **Screenshots** if applicable

### Suggesting Features

Feature requests are welcome! Please provide:

- **Clear use case**
- **Detailed description**
- **Potential implementation ideas**
- **Examples** if possible

### Contributing Code

1. **Find an issue** to work on (or create one)
2. **Comment on the issue** to let others know you're working on it
3. **Follow coding standards**
4. **Write tests** for new features
5. **Update documentation** if needed
6. **Submit a Pull Request**

## Development Setup

### Backend (Rust)

**Prerequisites:**
- Rust 1.78+ [Install](https://rustup.rs/)
- Cargo

```bash
# Build the project
cargo build --release

# Run tests
cargo test --all-features

# Run with dashboard feature
cargo run --features dashboard --bin vortex_dashboard

# Run API server
cargo run --bin vortex_api
```

### Frontend (React/TypeScript)

**Prerequisites:**
- Node.js 20+ [Install](https://nodejs.org/)
- npm

```bash
cd frontend

# Install dependencies
npm install

# Run development server
npm run dev

# Build for production
npm run build

# Run linter
npm run lint
```

## Coding Standards

### Rust

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document all public APIs with `///` comments
- Use meaningful variable and function names

```rust
// Good
/// Processes user input and routes to appropriate kernel
async fn process_input(session_id: &str, input: &str) -> Result<Response> {
    // Implementation
}

// Bad
async fn proc(s: &str, i: &str) -> Result<Response> {
    // Implementation
}
```

### TypeScript/React

- Follow [TypeScript Deep Dive](https://basarat.gitbook.io/typescript/)
- Use functional components with hooks
- Use TypeScript strict mode
- Follow the existing component structure
- Use `clsx` for conditional classes

```tsx
// Good
interface ButtonProps {
  variant?: 'primary' | 'secondary';
  children: React.ReactNode;
}

export function Button({ variant = 'primary', children }: ButtonProps) {
  return <button className={clsx('btn', `btn-${variant}`)}>{children}</button>;
}

// Bad
export function Button(props: any) {
  return <button>{props.children}</button>;
}
```

## Testing

### Backend Tests

```bash
# Run all tests
cargo test --all-features

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Frontend Tests

```bash
cd frontend

# Run all tests
npm run test

# Run with coverage
npm run test:coverage

# Run E2E tests
npm run test:e2e
```

## Documentation

- Update README.md for user-facing changes
- Update API_REFERENCE.md for API changes
- Add inline code comments for complex logic
- Update knowledge module docs when adding new modules

## Pull Request Process

1. **Update your fork** with the latest changes from `main`
2. **Resolve any merge conflicts**
3. **Ensure all tests pass**
4. **Update documentation** if needed
5. **Fill out the PR template** completely
6. **Request review** from maintainers
7. **Address review comments**
8. **Wait for approval** and merge

### PR Title Format

```
feat: add new knowledge module for physics
fix: resolve memory leak in vector store
docs: update API reference for v1.2.0
test: add integration tests for Kernel_03
refactor: simplify WebSocket reconnection logic
```

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
How were the changes tested?

## Checklist
- [ ] Code follows style guidelines
- [ ] Tests pass locally
- [ ] Documentation updated
- [ ] No new warnings
```

## Community

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and ideas
- **Pull Requests**: For code contributions

## Recognition

Contributors will be recognized in our README.md and release notes.

---

Thank you for contributing to Vortex Atoms AI!
