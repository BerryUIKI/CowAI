import { useEffect, useState } from "react";
import type { FormEvent } from "react";

import {
  activateModelProvider,
  deleteModelProvider,
  getModelSettings,
  saveModelProvider,
  testModelProvider,
} from "./api";
import type { ModelProvider, ModelSettings, ProviderProtocol } from "./types";

interface ProtocolOption {
  id: ProviderProtocol;
  label: string;
  description: string;
}

const protocolOptions: ProtocolOption[] = [
  {
    id: "openai",
    label: "OpenAI Chat",
    description: "Chat Completions API. OpenAI, DeepSeek, Ollama, and most relays.",
  },
  {
    id: "anthropic",
    label: "Anthropic",
    description: "Messages API. Claude and Claude Code-compatible relays.",
  },
  {
    id: "openai-responses",
    label: "Responses",
    description: "Responses API. OpenAI and Codex-style endpoints.",
  },
];

interface ProviderPreset {
  id: string;
  label: string;
  protocol: ProviderProtocol;
  baseUrl: string;
  model: string;
}

function customPreset(protocol: ProviderProtocol): ProviderPreset {
  return {
    id: "custom",
    label: "Custom",
    protocol,
    baseUrl: "",
    model: "",
  };
}

const presetsByProtocol: Record<ProviderProtocol, ProviderPreset[]> = {
  openai: [
    {
      id: "openai",
      label: "OpenAI",
      protocol: "openai",
      baseUrl: "https://api.openai.com/v1",
      model: "gpt-4o",
    },
    {
      id: "deepseek",
      label: "DeepSeek",
      protocol: "openai",
      baseUrl: "https://api.deepseek.com",
      model: "deepseek-chat",
    },
    {
      id: "ollama",
      label: "Ollama",
      protocol: "openai",
      baseUrl: "http://localhost:11434/v1",
      model: "llama3.2",
    },
    customPreset("openai"),
  ],
  anthropic: [
    {
      id: "claude",
      label: "Claude",
      protocol: "anthropic",
      baseUrl: "https://api.anthropic.com",
      model: "claude-sonnet-5-5",
    },
    customPreset("anthropic"),
  ],
  "openai-responses": [
    {
      id: "openai-responses",
      label: "OpenAI Responses",
      protocol: "openai-responses",
      baseUrl: "https://api.openai.com/v1",
      model: "gpt-4o",
    },
    customPreset("openai-responses"),
  ],
};

function protocolLabel(protocol: ProviderProtocol): string {
  return (
    protocolOptions.find((option) => option.id === protocol)?.label ?? protocol
  );
}

function presetFor(
  protocol: ProviderProtocol,
  presetId: string,
): ProviderPreset {
  const presets = presetsByProtocol[protocol];
  return presets.find((preset) => preset.id === presetId) ?? presets[0];
}

interface ProviderDraft {
  providerId: string | null;
  protocol: ProviderProtocol;
  presetId: string;
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  clearApiKey: boolean;
}

interface ModelProviderSettingsProps {
  /// Hide the configuration file path when embedded in a guided flow.
  showConfigPath?: boolean;
}

function draftForPreset(preset: ProviderPreset): ProviderDraft {
  return {
    providerId: null,
    protocol: preset.protocol,
    presetId: preset.id,
    name: preset.id === "custom" ? "" : preset.label,
    baseUrl: preset.baseUrl,
    model: preset.model,
    apiKey: "",
    clearApiKey: false,
  };
}

function draftForProvider(provider: ModelProvider): ProviderDraft {
  return {
    providerId: provider.id,
    protocol: provider.protocol,
    presetId: "custom",
    name: provider.name,
    baseUrl: provider.base_url,
    model: provider.model,
    apiKey: "",
    clearApiKey: false,
  };
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong. Try again.";
}

export default function ModelProviderSettings({
  showConfigPath = true,
}: ModelProviderSettingsProps) {
  const [modelSettings, setModelSettings] = useState<ModelSettings | null>(null);
  const [draft, setDraft] = useState<ProviderDraft | null>(null);
  const [modelBusy, setModelBusy] = useState<string | null>(null);
  const [modelError, setModelError] = useState<string | null>(null);
  const [modelNotice, setModelNotice] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getModelSettings()
      .then((settings) => {
        if (!cancelled) setModelSettings(settings);
      })
      .catch((error) => {
        if (!cancelled) setModelError(errorMessage(error));
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const activeProvider =
    modelSettings?.providers.find((provider) => provider.active) ?? null;
  const hasStoredKey = draft?.providerId
    ? (modelSettings?.providers.find((provider) => provider.id === draft.providerId)
        ?.has_api_key ?? false)
    : false;

  function updateDraft(patch: Partial<ProviderDraft>) {
    setDraft((current) => (current ? { ...current, ...patch } : current));
  }

  function startAddProvider() {
    setDraft(draftForPreset(presetsByProtocol.openai[0]));
    setModelError(null);
    setModelNotice(null);
  }

  function startEditProvider(provider: ModelProvider) {
    setDraft(draftForProvider(provider));
    setModelError(null);
    setModelNotice(null);
  }

  function changeProtocol(protocol: ProviderProtocol) {
    if (!draft) return;
    const preset = presetsByProtocol[protocol][0];
    updateDraft({
      protocol,
      presetId: preset.id,
      name: draft.providerId ? draft.name : preset.label,
      baseUrl: preset.baseUrl,
      model: preset.model,
    });
  }

  function changePreset(presetId: string) {
    if (!draft) return;
    const preset = presetFor(draft.protocol, presetId);
    updateDraft({
      presetId,
      name: preset.id === "custom" ? draft.name : preset.label,
      baseUrl: preset.baseUrl,
      model: preset.model,
    });
  }

  async function handleSaveProvider(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft) return;

    setModelBusy("save-provider");
    setModelError(null);
    setModelNotice(null);
    try {
      const next = await saveModelProvider({
        providerId: draft.providerId,
        protocol: draft.protocol,
        name: draft.name,
        baseUrl: draft.baseUrl,
        model: draft.model,
        apiKey: draft.apiKey,
        clearApiKey: draft.clearApiKey,
      });
      setModelSettings(next);
      setDraft(null);
      setModelNotice(draft.providerId ? "Provider updated." : "Provider added.");
    } catch (error) {
      setModelError(errorMessage(error));
    } finally {
      setModelBusy(null);
    }
  }

  async function handleTestProvider() {
    if (!draft) return;

    setModelBusy("test-provider");
    setModelError(null);
    setModelNotice(null);
    try {
      const message = await testModelProvider({
        providerId: draft.providerId,
        protocol: draft.protocol,
        baseUrl: draft.baseUrl,
        model: draft.model,
        apiKey: draft.apiKey,
      });
      setModelNotice(message);
    } catch (error) {
      setModelError(errorMessage(error));
    } finally {
      setModelBusy(null);
    }
  }

  async function handleTestSavedProvider(provider: ModelProvider) {
    setModelBusy(`test-${provider.id}`);
    setModelError(null);
    setModelNotice(null);
    try {
      const message = await testModelProvider({
        providerId: provider.id,
        protocol: provider.protocol,
        baseUrl: provider.base_url,
        model: provider.model,
        apiKey: "",
      });
      setModelNotice(message);
    } catch (error) {
      setModelError(errorMessage(error));
    } finally {
      setModelBusy(null);
    }
  }

  async function handleActivateProvider(providerId: string | null) {
    setModelBusy(providerId ? `activate-${providerId}` : "activate-mock");
    setModelError(null);
    setModelNotice(null);
    try {
      const next = await activateModelProvider(providerId);
      setModelSettings(next);
      setModelNotice(
        providerId ? "Provider activated." : "Switched to the offline mock model.",
      );
    } catch (error) {
      setModelError(errorMessage(error));
    } finally {
      setModelBusy(null);
    }
  }

  async function handleDeleteProvider(provider: ModelProvider) {
    if (!window.confirm(`Delete provider "${provider.name}"?`)) return;

    setModelBusy(`delete-${provider.id}`);
    setModelError(null);
    setModelNotice(null);
    try {
      const next = await deleteModelProvider(provider.id);
      setModelSettings(next);
      if (draft?.providerId === provider.id) setDraft(null);
      setModelNotice("Provider deleted.");
    } catch (error) {
      setModelError(errorMessage(error));
    } finally {
      setModelBusy(null);
    }
  }

  const status =
    modelError || modelNotice ? (
      <>
        {modelError ? (
          <p className="settings-status is-error" role="alert">
            {modelError}
          </p>
        ) : null}
        {modelNotice ? (
          <p className="settings-status is-ok" role="status">
            {modelNotice}
          </p>
        ) : null}
      </>
    ) : null;

  if (!modelSettings) {
    return <p className="settings-empty">Loading model settings…</p>;
  }

  return (
    <>
      <div className="settings-active">
        <span className="settings-badge settings-badge-active">Active</span>
        <div className="settings-active-body">
          <p className="settings-card-title">{modelSettings.active_label}</p>
          <p className="settings-card-subtitle">
            {activeProvider
              ? `${protocolLabel(activeProvider.protocol)} · ${activeProvider.model} · ${activeProvider.base_url}`
              : "No provider selected. Runs use the offline mock model."}
          </p>
        </div>
        {activeProvider ? (
          <button
            type="button"
            className="ghost-button"
            onClick={() => handleActivateProvider(null)}
            disabled={modelBusy !== null}
          >
            Use offline mock
          </button>
        ) : null}
      </div>

      {modelSettings.providers.length === 0 ? (
        <p className="settings-empty">
          No providers yet. Add one to connect a real model.
        </p>
      ) : (
        <ul className="settings-list">
          {modelSettings.providers.map((provider) => (
            <li key={provider.id} className="settings-card">
              <div className="settings-card-head">
                <span className="settings-card-title">{provider.name}</span>
                <span className="settings-badge">
                  {protocolLabel(provider.protocol)}
                </span>
                {provider.active ? (
                  <span className="settings-badge settings-badge-active">Active</span>
                ) : null}
              </div>
              <p className="settings-card-body">
                {provider.model} · {provider.base_url}
              </p>
              <div className="settings-card-foot">
                <span className="settings-meta-inline">
                  {provider.has_api_key ? "API key saved" : "No API key"}
                </span>
                <div className="settings-card-actions">
                  {!provider.active ? (
                    <button
                      type="button"
                      className="ghost-button"
                      onClick={() => handleActivateProvider(provider.id)}
                      disabled={modelBusy !== null}
                    >
                      Use
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() => handleTestSavedProvider(provider)}
                    disabled={modelBusy !== null}
                  >
                    {modelBusy === `test-${provider.id}` ? "Testing…" : "Test"}
                  </button>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() => startEditProvider(provider)}
                    disabled={modelBusy !== null}
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() => handleDeleteProvider(provider)}
                    disabled={modelBusy !== null}
                  >
                    Delete
                  </button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      {draft ? (
        <form className="settings-form provider-editor" onSubmit={handleSaveProvider}>
          <div className="protocol-choice" role="group" aria-label="API protocol">
            {protocolOptions.map((option) => (
              <button
                key={option.id}
                type="button"
                className={`protocol-option${
                  draft.protocol === option.id ? " is-selected" : ""
                }`}
                aria-pressed={draft.protocol === option.id}
                onClick={() => changeProtocol(option.id)}
                disabled={modelBusy !== null}
              >
                <span className="protocol-option-label">{option.label}</span>
                <span className="protocol-option-hint">{option.description}</span>
              </button>
            ))}
          </div>

          <div className="preset-row" role="group" aria-label="Provider presets">
            {presetsByProtocol[draft.protocol].map((preset) => (
              <button
                key={preset.id}
                type="button"
                className={`preset-chip${
                  draft.presetId === preset.id ? " is-selected" : ""
                }`}
                aria-pressed={draft.presetId === preset.id}
                onClick={() => changePreset(preset.id)}
                disabled={modelBusy !== null}
              >
                {preset.label}
              </button>
            ))}
          </div>

          <div className="settings-form-row">
            <label className="field">
              <span>Name</span>
              <input
                value={draft.name}
                onChange={(event) => updateDraft({ name: event.target.value })}
                placeholder="OpenAI"
                autoComplete="off"
              />
            </label>
            <label className="field">
              <span>Model</span>
              <input
                value={draft.model}
                onChange={(event) => updateDraft({ model: event.target.value })}
                placeholder="gpt-4o"
                autoComplete="off"
              />
            </label>
          </div>

          <label className="field">
            <span>Base URL</span>
            <input
              value={draft.baseUrl}
              onChange={(event) => updateDraft({ baseUrl: event.target.value })}
              placeholder={
                draft.protocol === "anthropic"
                  ? "https://api.anthropic.com"
                  : "https://api.openai.com/v1"
              }
              autoComplete="off"
            />
          </label>

          <label className="field">
            <span>API key</span>
            <input
              type="password"
              value={draft.apiKey}
              onChange={(event) =>
                updateDraft({
                  apiKey: event.target.value,
                  clearApiKey: false,
                })
              }
              placeholder={
                hasStoredKey
                  ? "Saved — leave blank to keep"
                  : "Optional for local endpoints"
              }
              autoComplete="off"
            />
          </label>

          {hasStoredKey ? (
            <label className="settings-checkbox">
              <input
                type="checkbox"
                checked={draft.clearApiKey}
                onChange={(event) =>
                  updateDraft({
                    clearApiKey: event.target.checked,
                    apiKey: event.target.checked ? "" : draft.apiKey,
                  })
                }
              />
              Remove the saved API key
            </label>
          ) : null}

          <div className="form-actions">
            <button
              type="submit"
              className="primary-button"
              disabled={
                modelBusy !== null ||
                !draft.name.trim() ||
                !draft.baseUrl.trim() ||
                !draft.model.trim()
              }
            >
              {modelBusy === "save-provider" ? "Saving…" : "Save provider"}
            </button>
            <button
              type="button"
              className="ghost-button"
              onClick={handleTestProvider}
              disabled={
                modelBusy !== null ||
                !draft.baseUrl.trim() ||
                !draft.model.trim()
              }
            >
              {modelBusy === "test-provider" ? "Testing…" : "Test connection"}
            </button>
            <button
              type="button"
              className="ghost-button"
              onClick={() => setDraft(null)}
              disabled={modelBusy !== null}
            >
              Cancel
            </button>
          </div>

          {status}

          <p className="settings-hint">
            Keys are stored in plain text in the local configuration file;
            protect it as sensitive local data.
          </p>
        </form>
      ) : (
        <>
          <button type="button" className="settings-add-button" onClick={startAddProvider}>
            <span aria-hidden="true">+</span> Add provider
          </button>
          {status}
        </>
      )}

      {showConfigPath ? (
        <p className="settings-note">Configuration: {modelSettings.config_path}</p>
      ) : null}
    </>
  );
}
