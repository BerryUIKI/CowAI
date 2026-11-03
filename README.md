# 🍇 BerryBot

> **Open-Source, High-Performance Multi-Agent Collaboration Platform in Rust**

BerryBot is a modern, modular multi-agent platform designed for collaborative software engineering and automation. Built in Rust for speed, safety, and deterministic concurrency, BerryBot combines the structured role-and-task delegation of **[CrewAI](https://github.com/crewAIInc/crewAI)** with the channel-based, event-driven collaboration patterns of **[Mattermost](https://github.com/mattermost/mattermost)**.

---

## 🌟 Key Features

- **⚡ Blazing Fast & Memory Safe**: Written in 100% safe Rust on top of the Tokio async ecosystem. Single binary deployment with zero runtime overhead.
- **🤖 CrewAI-Style Orchestration**:
  - **Agents**: Configured with `role`, `goal`, `backstory`, capabilities, and iteration limits.
  - **Tasks**: Explicit descriptions, expected deliverables, and prerequisite context chaining.
  - **Crews**: Group agents into sequential or hierarchical workflows.
- **💬 Mattermost-Inspired Collaboration**:
  - Asynchronous event bus (`berry-bus`) supporting pub/sub event distribution.
  - Structured channels and threaded messages for human-in-the-loop supervision and agent collaboration.
- **🛡️ Secure Code Sandbox**:
  - `berry-tools` includes a subprocess sandbox for executing code, running shell commands, and manipulating files with timeout protection and output truncation.
- **🔌 Pluggable LLM Providers**:
  - First-class support for OpenAI, DeepSeek, Ollama, OpenRouter, and any OpenAI-compatible API, plus local mock providers for deterministic testing.

---

## 🏗️ Architecture

```
BerryBot/
├── Cargo.toml                  # Workspace root manifest
├── CLAUDE.md                   # AI developer guidelines
├── crates/
│   ├── berry-core/             # Core domain models (Agent, Task, Channel, Tool, Event)
│   ├── berry-bus/              # Tokio broadcast event bus and pub/sub routing
│   ├── berry-tools/            # Code execution sandbox & filesystem tools
│   ├── berry-llm/              # OpenAI-compatible & Mock LLM providers
│   ├── berry-agent/            # Agent runtime loop & Crew orchestration
│   └── berry-cli/              # Command-line interface (`berry`)
```

---

## 🚀 Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) (1.75+ recommended)

### Build the Project
```bash
cargo build --workspace
```

### Run Tests
```bash
cargo test --workspace
```

### Inspect Available Tools
```bash
cargo run -p berry-cli -- tools
```

### Run an Autonomous Multi-Agent Task
```bash
# Dry-run mode (using built-in mock LLM)
cargo run -p berry-cli -- run --task "Build a REST API endpoint for user authentication" --dry-run

# Live execution with OpenAI / DeepSeek / Ollama
export OPENAI_API_KEY="your-api-key"
cargo run -p berry-cli -- run --task "Implement a snake game in Python" --model gpt-4o

# Using local Ollama
cargo run -p berry-cli -- run \
  --task "Analyze log files" \
  --base-url "http://localhost:11434/v1" \
  --model "llama3"
```

---

## 🤝 Roadmap & Community

- [x] Cargo workspace modular architecture
- [x] Tokio broadcast event bus (`berry-bus`)
- [x] Pluggable subprocess code execution sandbox (`berry-tools`)
- [x] CrewAI-inspired Agent, Task, and Crew abstractions (`berry-agent`)
- [x] OpenAI / DeepSeek / Ollama provider client (`berry-llm`)
- [ ] Docker container sandbox backend
- [ ] Webhook & REST API server for Mattermost / Slack bot integrations
- [ ] Interactive Web / TUI multi-agent collaboration dashboard

---

## 📄 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
