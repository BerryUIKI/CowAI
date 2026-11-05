# ADR 0016: Model Provider Protocols

- **Status**: Accepted
- **Date**: 2026-11-05
- **Related**: `docs/ADR/0009-desktop-ipc-and-state.md`, `docs/ADR/0012-local-model-configuration.md`
- **Extends**: ADR-0012 (provider storage and selection)

## Context

ADR-0012 stored providers as OpenAI-compatible endpoints, and the desktop built
a single Chat Completions client. In practice users point CowBot at endpoints
that speak other protocols: Claude Code-compatible relays expose the Anthropic
Messages API (`/v1/messages`), and Codex-style endpoints expose the OpenAI
Responses API (`/v1/responses`). A correctly configured endpoint therefore
failed with an HTTP error because the client always called
`/v1/chat/completions`.

[CC Switch](https://github.com/farion1231/cc-switch) demonstrates the expected
interaction: pick the protocol family first, then a provider preset, then supply
only the key (or a custom base URL). No code is shared with it.

## Decision

### 1. Protocol field and compatibility

`ProviderConfig` gains `protocol` with the values `openai`, `anthropic`, and
`openai-responses`; `openai` is the serde default. Files written before this
change load as OpenAI providers, so the configuration version stays 1 and no
migration is required. Protocol belongs to the provider record in `config.json`,
not to workspace data.

### 2. Clients

`cowbot-llm` gains two `LlmProvider` implementations:

- `AnthropicProvider` speaks the Messages API: system messages become the
  top-level `system` prompt, assistant tool calls become `tool_use` blocks,
  consecutive tool results collapse into one user message of `tool_result`
  blocks, and tools use `input_schema`. The key is sent as both `x-api-key` and
  `Authorization: Bearer` so the official API and Claude Code-style relays work
  without another setting. `max_tokens` is fixed at 4096 because the API
  requires it.
- `ResponsesProvider` speaks the Responses API: tool calls are `function_call`
  input items, tool results are `function_call_output` items, and tools are flat
  function definitions. Response `output` items map back to text and tool calls.

Both clients accept a base URL with or without `/v1`, or the full endpoint URL,
so a URL copied from provider documentation works unchanged. The OpenAI Chat
Completions client is unchanged.

### 3. Desktop interaction

- The provider editor selects a protocol first, then offers presets: OpenAI,
  DeepSeek, Ollama, and Custom for Chat Completions; Claude and Custom for
  Anthropic; OpenAI Responses and Custom for Responses.
- Provider cards and the active summary show the protocol.
- `test_model_provider` uses the selected protocol.
- Settings moves from the primary navigation to the sidebar footer and becomes
  a sectioned page (Model, Team, Workspace), keeping workspace-level
  configuration out of the work navigation.

## Alternatives considered

- **Auto-detect the protocol by probing endpoints.** Adds latency and ambiguity,
  and a failed probe still needs a manual choice; rejected.
- **One fixed provider per protocol with no custom base URL.** Excludes local
  and self-hosted endpoints; rejected.
- **A generic plugin registry for protocols.** Useful once third-party
  protocols become a requirement, but three built-in values cover the current
  set without a dynamic loading surface; deferred.
- **Keep the OpenAI-only client and document relays.** The reported failure is
  exactly a protocol mismatch, so documentation alone does not solve it.

## Consequences

- Each protocol has its own URL shape, auth headers, and tool-call mapping, with
  unit coverage in `cowbot-llm` and the desktop config layer.
- Anthropic requests use a fixed `max_tokens`; a per-provider override is
  follow-up work.
- Streaming and token accounting remain absent for all three protocols.
- Gemini and other protocol families can be added as new enum values and
  clients without changing the storage shape.
