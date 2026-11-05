import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";

import ModelProviderSettings from "./ModelProviderSettings";
import {
  createWorkspace,
  exportWorkspace,
  importWorkspace,
  listWorkspaces,
  switchWorkspace,
} from "./api";
import type {
  AgentProfile,
  ClientSession,
  TemplateDocument,
  WorkspaceSummary,
} from "./types";

interface SettingsViewProps {
  session: ClientSession | null;
  agents: AgentProfile[];
  templates: TemplateDocument[];
  templateId: string;
  busy: string | null;
  onSelectTemplate: (templateId: string) => void;
  onAddAgents: () => void;
}

type SettingsSection = "model" | "team" | "workspace";

const sections: { id: SettingsSection; label: string }[] = [
  { id: "model", label: "Model" },
  { id: "team", label: "Team" },
  { id: "workspace", label: "Workspace" },
];

function agentInitials(name: string): string {
  return name
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("");
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong. Try again.";
}

function packageFileName(name: string | undefined): string {
  const base =
    (name ?? "workspace").trim().replace(/[\\/:*?"<>|]+/g, "-") || "workspace";
  return base.toLowerCase().endsWith(".cowpkg") ? base : `${base}.cowpkg`;
}

export default function SettingsView({
  session,
  agents,
  templates,
  templateId,
  busy,
  onSelectTemplate,
  onAddAgents,
}: SettingsViewProps) {
  const [section, setSection] = useState<SettingsSection>("model");
  const [workspaces, setWorkspaces] = useState<WorkspaceSummary[]>([]);
  const [workspaceBusy, setWorkspaceBusy] = useState<string | null>(null);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const [workspaceNotice, setWorkspaceNotice] = useState<string | null>(null);
  const [newWorkspaceName, setNewWorkspaceName] = useState("");

  useEffect(() => {
    let cancelled = false;

    listWorkspaces()
      .then((next) => {
        if (!cancelled) setWorkspaces(next);
      })
      .catch((error) => {
        if (!cancelled) setWorkspaceError(errorMessage(error));
      });

    return () => {
      cancelled = true;
    };
  }, []);

  async function handleSwitchWorkspace(database: string) {
    setWorkspaceBusy(`switch-${database}`);
    setWorkspaceError(null);
    setWorkspaceNotice(null);
    try {
      await switchWorkspace(database);
      // The backend restarts the app into the selected database.
    } catch (error) {
      setWorkspaceError(errorMessage(error));
      setWorkspaceBusy(null);
    }
  }

  async function handleExportWorkspace() {
    setWorkspaceError(null);
    setWorkspaceNotice(null);

    let target: string | null = null;
    try {
      target = await save({
        title: "Export workspace",
        defaultPath: packageFileName(session?.workspace.name),
        filters: [{ name: "CowBot package", extensions: ["cowpkg"] }],
      });
    } catch (error) {
      setWorkspaceError(errorMessage(error));
      return;
    }
    if (!target) return;

    setWorkspaceBusy("export");
    try {
      const written = await exportWorkspace(target);
      setWorkspaceNotice(`Exported to ${written}`);
    } catch (error) {
      setWorkspaceError(errorMessage(error));
    } finally {
      setWorkspaceBusy(null);
    }
  }

  async function handleCreateWorkspace() {
    const name = newWorkspaceName.trim();
    if (!name) return;

    setWorkspaceBusy("create");
    setWorkspaceError(null);
    setWorkspaceNotice(null);
    try {
      await createWorkspace(name);
      // The backend registers the workspace and restarts the app.
    } catch (error) {
      setWorkspaceError(errorMessage(error));
      setWorkspaceBusy(null);
    }
  }

  async function handleImportWorkspace() {
    setWorkspaceError(null);
    setWorkspaceNotice(null);

    let selected: string | null = null;
    try {
      selected = await open({
        title: "Import workspace package",
        multiple: false,
        directory: false,
        filters: [{ name: "CowBot package", extensions: ["cowpkg"] }],
      });
    } catch (error) {
      setWorkspaceError(errorMessage(error));
      return;
    }
    if (typeof selected !== "string") return;

    setWorkspaceBusy("import");
    try {
      await importWorkspace(selected);
      // The backend registers the workspace and restarts the app.
    } catch (error) {
      setWorkspaceError(errorMessage(error));
      setWorkspaceBusy(null);
    }
  }

  return (
    <div className="settings-view">
      <header className="view-header">
        <h2 className="view-title">Settings</h2>
        <p className="view-subtitle">
          Model providers, team, and workspace details for this workspace.
        </p>
      </header>

      <div className="settings-layout">
        <nav className="settings-nav" aria-label="Settings sections">
          {sections.map((item) => (
            <button
              key={item.id}
              type="button"
              className="settings-nav-item"
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => setSection(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>

        <div className="settings-content">
          {section === "model" ? (
            <section className="settings-section" aria-labelledby="settings-model">
              <div className="settings-section-head">
                <h3 id="settings-model">Model providers</h3>
                <p className="settings-hint">
                  Connect an OpenAI-compatible, Anthropic-compatible, or Responses-compatible
                  endpoint. Without an active provider, CowBot uses the offline mock model.
                </p>
              </div>
              <ModelProviderSettings />
            </section>
          ) : null}

          {section === "team" ? (
            <>
              <section className="settings-section" aria-labelledby="settings-templates">
                <div className="settings-section-head">
                  <h3 id="settings-templates">Agent templates</h3>
                  <p className="settings-hint">
                    Instantiating a template adds its agents to this workspace.
                  </p>
                </div>
                {templates.length === 0 ? (
                  <p className="settings-empty">No templates available.</p>
                ) : (
                  <ul className="settings-list">
                    {templates.map((template) => (
                      <li key={template.id} className="settings-card">
                        <div className="settings-card-head">
                          <span className="settings-card-title">{template.name}</span>
                          <span className="settings-badge">{template.kind}</span>
                        </div>
                        {template.description ? (
                          <p className="settings-card-body">{template.description}</p>
                        ) : null}
                        <div className="settings-card-foot">
                          <span className="settings-meta-inline">
                            {template.agents.length} agent
                            {template.agents.length === 1 ? "" : "s"}
                          </span>
                          <button
                            type="button"
                            className="ghost-button"
                            onClick={() => {
                              onSelectTemplate(template.id);
                              onAddAgents();
                            }}
                            disabled={busy === "add-agents"}
                          >
                            {busy === "add-agents" && templateId === template.id
                              ? "Adding…"
                              : "Add agents"}
                          </button>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              <section className="settings-section" aria-labelledby="settings-agents">
                <div className="settings-section-head">
                  <h3 id="settings-agents">Agents</h3>
                  <p className="settings-hint">
                    {agents.length === 0
                      ? "No agents yet. Add a template above."
                      : `${agents.length} agent${agents.length === 1 ? "" : "s"} in this workspace.`}
                  </p>
                </div>
                {agents.length === 0 ? (
                  <p className="settings-empty">No agents yet.</p>
                ) : (
                  <ul className="settings-list">
                    {agents.map((agent) => (
                      <li key={agent.id} className="settings-card">
                        <div className="settings-card-head">
                          <span className="agent-avatar" aria-hidden="true">
                            {agentInitials(agent.name)}
                          </span>
                          <div>
                            <span className="settings-card-title">{agent.name}</span>
                            <p className="settings-card-subtitle">{agent.role}</p>
                          </div>
                        </div>
                        {agent.goal ? <p className="settings-card-body">{agent.goal}</p> : null}
                        <div className="settings-card-foot">
                          <span className="settings-meta-inline">
                            {agent.model ?? "default model"}
                          </span>
                          <span className="settings-meta-inline">
                            {agent.tools.length === 0
                              ? "no tools"
                              : agent.tools.join(", ")}
                          </span>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </>
          ) : null}

          {section === "workspace" ? (
            <>
              <section className="settings-section" aria-labelledby="settings-workspace">
                <div className="settings-section-head">
                  <h3 id="settings-workspace">Workspace</h3>
                  <p className="settings-hint">
                    Local workspace identity. Data stays on this machine.
                  </p>
                </div>
                <dl className="settings-meta">
                  <div>
                    <dt>Workspace</dt>
                    <dd>{session?.workspace.name ?? "—"}</dd>
                  </div>
                  <div>
                    <dt>Owner</dt>
                    <dd>{session?.user.name ?? "—"}</dd>
                  </div>
                  <div>
                    <dt>Email</dt>
                    <dd>{session?.user.email ?? "—"}</dd>
                  </div>
                </dl>
              </section>

              <section
                className="settings-section"
                aria-labelledby="settings-local-workspaces"
              >
                <div className="settings-section-head">
                  <h3 id="settings-local-workspaces">Local workspaces</h3>
                  <p className="settings-hint">
                    Each workspace lives in its own local database. Creating,
                    importing, and switching restart CowBot; finish running tasks
                    first.
                  </p>
                </div>

                {workspaceError ? (
                  <p className="settings-status is-error" role="alert">
                    {workspaceError}
                  </p>
                ) : null}
                {workspaceNotice ? (
                  <p className="settings-status is-ok" role="status">
                    {workspaceNotice}
                  </p>
                ) : null}

                <div className="workspace-create">
                  <label className="field">
                    <span>New workspace</span>
                    <input
                      value={newWorkspaceName}
                      onChange={(event) => setNewWorkspaceName(event.target.value)}
                      placeholder="Team workspace"
                      autoComplete="off"
                      disabled={workspaceBusy !== null}
                    />
                  </label>
                  <button
                    type="button"
                    className="primary-button"
                    onClick={handleCreateWorkspace}
                    disabled={workspaceBusy !== null || !newWorkspaceName.trim()}
                  >
                    {workspaceBusy === "create" ? "Creating…" : "Create workspace"}
                  </button>
                </div>

                {workspaces.length === 0 ? (
                  <p className="settings-empty">No workspaces registered.</p>
                ) : (
                  <ul className="settings-list">
                    {workspaces.map((workspace) => (
                      <li key={workspace.database} className="settings-card">
                        <div className="settings-card-head">
                          <span className="settings-card-title">{workspace.name}</span>
                          {workspace.active ? (
                            <span className="settings-badge settings-badge-active">
                              Active
                            </span>
                          ) : null}
                          {!workspace.available ? (
                            <span className="settings-badge">Missing</span>
                          ) : null}
                        </div>
                        <p className="settings-card-body">{workspace.database}</p>
                        <div className="settings-card-foot">
                          <span className="settings-meta-inline">
                            {workspace.active
                              ? "Open now"
                              : workspace.available
                                ? "Available"
                                : "Database file not found"}
                          </span>
                          <div className="settings-card-actions">
                            {!workspace.active && workspace.available ? (
                              <button
                                type="button"
                                className="ghost-button"
                                onClick={() => handleSwitchWorkspace(workspace.database)}
                                disabled={workspaceBusy !== null}
                              >
                                {workspaceBusy === `switch-${workspace.database}`
                                  ? "Switching…"
                                  : "Switch"}
                              </button>
                            ) : null}
                          </div>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}

                <div className="form-actions workspace-actions">
                  <button
                    type="button"
                    className="primary-button"
                    onClick={handleExportWorkspace}
                    disabled={workspaceBusy !== null}
                  >
                    {workspaceBusy === "export" ? "Exporting…" : "Export workspace"}
                  </button>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={handleImportWorkspace}
                    disabled={workspaceBusy !== null}
                  >
                    {workspaceBusy === "import" ? "Importing…" : "Import package"}
                  </button>
                </div>
              </section>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}
