# ADR 0021: Vendor-First Model Providers with Model Discovery

- **Status**: Accepted
- **Date**: 2026-11-07
- **Related**: `docs/ADR/0009-desktop-ipc-and-state.md`, `docs/ADR/0012-local-model-configuration.md`
- **Extends**: ADR-0016 (provider protocols)

## Context

ADR-0016 made the wire protocol a provider field, but the editor still asked
users to choose a protocol first: three protocol cards, then that protocol's
presets. The layout matched the storage model rather than the task. Users know
which vendor they want, not which protocol it speaks, and one vendor may expose
several protocols. Switching protocol discarded the base URL, model, and name,
because the handler replaced the whole draft with the new protocol's first
preset.

Providers also stored exactly one model. A single key commonly reaches several
models, so testing another model meant saving a second provider with the same
base URL and key, or retyping the model and hoping the spelling was right. The
endpoint's own catalogue was invisible even though the protocols CowBot speaks
expose model listing endpoints.

[CC Switch](https://github.com/farion1231/cc-switch) shows the expected flow: a
searchable, grouped vendor catalogue with Custom first, then one form where the
protocol is a field. No code is shared with it. Issues #59 and #60 define the
behaviour; this record covers the decisions behind it.

## Decision

### 1. Vendor catalogue

**Add provider** opens a searchable catalogue instead of a form. Entries are
grouped as model vendors, third-party platforms and relays, cloud providers,
and local runtimes. **Custom** is pinned at the top and opens a blank form.
Search matches the entry name, description, base URL, and protocol label.

Selecting an entry pre-fills a form with the vendor's name, protocol, base URL,
and default model. The catalogue is static data in the frontend; it is a
convenience over the same create-provider command, not a new storage concept.

Deliberately excluded from the form because CowBot cannot honour them: OAuth
account sign-in, a free-text note, and User-Agent, header, or request-body
overrides. The editor only offers settings that change how the client behaves.

### 2. Protocol as a form field

The protocol is a dropdown inside the form. Changing it updates only the
protocol: the name, base URL, model list, and active model stay as the user
entered them. A saved provider keeps the name the user typed, and providers
saved before the catalogue existed load into the same form with their stored
name, protocol, URL, and model unchanged.

The first-run onboarding reuses the same component, so it shows the catalogue
too.

### 3. Several models per provider

`ProviderConfig` gains `models: Vec<String>` with `#[serde(default)]`. `model`
remains the active model and the only value the agent runtime reads, so
execution is unchanged.

- On load, every provider's list is normalized: entries are trimmed, blanks and
  duplicates are dropped, and the active model is added when missing. A legacy
  `config.json` without `models` therefore loads with `models = [model]`.
- `make_provider` applies the same normalization on save. Files stay at
  configuration version 1; older builds ignore the unknown field.
- `ProviderView` exposes the list to the frontend, and `ProviderDraft` accepts
  it on save. A blank API key still keeps the stored key, so switching the
  active model never requires retyping the secret.

### 4. Model discovery

`LlmProvider` gains `list_models` with a default implementation that reports
model discovery as unsupported, so providers without a catalogue need no code
and callers can fall back to manual entry.

- `OpenAiCompatibleProvider` calls `GET {base_url}/models`.
- `ResponsesProvider` calls `GET {base_url}/models`.
- `AnthropicProvider` calls `GET {base_url}/v1/models` with `anthropic-version`
  and sends the key as `x-api-key` plus a bearer token, matching its chat
  requests so official endpoints and relays both work.

The Anthropic and Responses clients derive the URL from the shared endpoint
helper, which accepts a bare host, a host with `/v1`, or the full chat endpoint
URL. Response parsing is a pure function over the shared `data[].id` shape used
by all three APIs; it skips entries without an id and collapses duplicates, and
is covered by unit tests with sample JSON.

The `fetch_provider_models` Tauri command resolves the API key exactly like
`test_model_provider`: a typed key wins, otherwise the key stored for the
provider being edited is used. The request is bounded by a 30-second timeout,
and an empty listing is reported as an error.

In the editor, the model field is a dropdown fed by the provider's known models
with a manual-entry input beside it. **Fetch models** merges the endpoint's
listing into the draft, selects the first model when none is chosen, and reports
failures inline without disabling manual entry or saving. Provider cards show
the active model and the number of known models.

## Alternatives considered

- **Keep the protocol-first navigation.** It is the reported problem: it makes
  the common self-hosted OpenAI-compatible relay require picking a vendor list
  the relay is not in, and it cannot express a vendor with several protocols.
- **Fetch models automatically when the form opens or on save.** Adds network
  latency and failures to actions that should be local and deterministic;
  discovery stays an explicit user action.
- **Store model lists in SQLite.** Provider configuration is desktop-local
  runtime configuration, not workspace data (ADR-0012); moving it into the
  database would drag it into packages and backups for no benefit.
- **One provider per model.** Duplicates the base URL and secret, and makes
  switching models an edit of the provider list rather than a selection.
- **Vendor plugins or OAuth sign-in.** Would let the catalogue do more, but
  CowBot has neither an account model nor per-vendor auth flows; the three wire
  protocols remain the supported surface.

## Consequences

- The editor matches the user's vocabulary: vendor first, protocol only when it
  matters. Changing protocol is no longer destructive.
- Existing `config.json` files stay readable, and keeping the active model in
  the list means no separate migration.
- A model listing can be stale or incomplete. Fetching refreshes it, manual
  entry always works, and saving is never blocked by a failed listing.
- Endpoints without a usable `/models` response degrade to manual entry with an
  inline error.
- The vendor catalogue is maintained manually. Adding a vendor or changing a
  default model is a frontend data change, not a schema change. Cloud entries are
  limited to endpoints that accept a bearer API key; Azure OpenAI's `api-key`
  header and Vertex AI's OAuth flow are not representable with the current three
  clients, so those vendors are not listed.
- No OAuth, notes, or header/body overrides are offered; adding them later
  requires new provider fields and client support.
