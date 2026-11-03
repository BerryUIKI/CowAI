# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

BerryBot is an open-source, high-performance multi-agent collaboration platform with code execution and automation capabilities, built in Rust. It utilizes an event-driven message bus for decoupled multi-agent communication and modular tool sandboxing.

## Workspace Architecture

The project is structured as a Cargo workspace under `crates/`:

- **`crates/berry-core`**: Core domain types, shared error types (`BerryError`), and foundational traits (`Agent`, `Tool`). Defines event representations (`Event`, `Message`, `ToolCall`).
- **`crates/berry-bus`**: Tokio-backed asynchronous pub/sub event bus supporting broadcast channels, topic filtering, and event subscription handles.
- **`crates/berry-tools`**: Pluggable tool implementations and sandbox execution backends (`SandboxBackend`, `SubprocessExecutor`, `FileSystemTool`).
- **`crates/berry-llm`**: LLM provider abstractions (`LlmProvider`) and implementations for OpenAI-compatible and Anthropic endpoints.
- **`crates/berry-agent`**: Agent runtime loop, state/memory management, role definitions (Planner, Coder, Reviewer), and orchestration logic.
- **`crates/berry-cli`**: Command-line entry point (`berry-cli`), configuration loader, and interactive CLI/TUI runner.

## Common Development Commands

### Building
```bash
cargo build --workspace
cargo build --release --workspace
```

### Running the CLI
```bash
cargo run -p berry-cli -- --help
cargo run -p berry-cli -- run --task "Analyze and write code"
```

### Testing
```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p berry-core
cargo test -p berry-bus
cargo test -p berry-tools

# Run a single specific test
cargo test -p berry-bus -- test_bus_publish_subscribe --exact
```

### Linting & Formatting
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo fmt
```
