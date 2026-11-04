import type { AgentProfile, ClientSession, TemplateDocument } from "./types";

interface SettingsViewProps {
  session: ClientSession | null;
  agents: AgentProfile[];
  templates: TemplateDocument[];
  templateId: string;
  busy: string | null;
  onSelectTemplate: (templateId: string) => void;
  onAddAgents: () => void;
}

function agentInitials(name: string): string {
  return name
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("");
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
  return (
    <div className="settings-view">
      <header className="view-header">
        <h2 className="view-title">Settings</h2>
        <p className="view-subtitle">
          Agents and templates available to this workspace.
        </p>
      </header>

      <section className="settings-section">
        <div className="settings-section-head">
          <h3>Workspace</h3>
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

      <section className="settings-section">
        <div className="settings-section-head">
          <h3>Agent templates</h3>
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

      <section className="settings-section">
        <div className="settings-section-head">
          <h3>Agents</h3>
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
    </div>
  );
}
