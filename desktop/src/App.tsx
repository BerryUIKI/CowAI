import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, KeyboardEvent as ReactKeyboardEvent } from "react";

import ApprovalCards from "./ApprovalCards";
import Onboarding from "./Onboarding";
import SettingsView from "./SettingsView";
import TaskDetail from "./TaskDetail";
import type { AssignmentWithEvents, RunWithCalls } from "./TaskDetail";
import UpdateBanner from "./UpdateBanner";
import {
  cancelTask,
  completeOnboarding,
  createChannel,
  createTask,
  decideApproval,
  getOnboardingState,
  getSession,
  instantiateAgents,
  listAgents,
  listAssignmentEvents,
  listAssignments,
  listChannels,
  listMessages,
  listRuns,
  listTasks,
  listTemplates,
  listToolCalls,
  onCowbotEvent,
  onRunFinished,
  pendingApprovals,
  postMessage,
  runTask,
} from "./api";
import icon from "./assets/cowbot-icon.svg";
import wordmark from "./assets/cowbot-wordmark-dark.svg";
import { errorMessage } from "./errors";
import { formatTime } from "./format";
import type {
  AgentProfile,
  Approval,
  ApprovalLevel,
  Channel,
  ChannelMessage,
  ClientSession,
  OnboardingState,
  Task,
  TemplateDocument,
} from "./types";
import { approvalLevelLabel, taskStatusKind, taskStatusLabel } from "./types";
import { useUpdater } from "./updater";

type View = "channels" | "tasks" | "approvals" | "settings";

const approvalLevels: ApprovalLevel[] = ["ask", "safe_auto", "plan_only", "trusted"];

function senderLabel(message: ChannelMessage, session: ClientSession | null): string {
  const sender = message.sender;
  if (sender.type === "System") return "System";
  if (sender.type === "User") {
    return session && sender.id === session.user.id ? "You" : "Teammate";
  }
  return `Agent ${sender.id.slice(0, 8)}`;
}

function agentName(agents: AgentProfile[], id: string | null): string {
  if (!id) return "Unassigned";
  return agents.find((agent) => agent.id === id)?.name ?? `Agent ${id.slice(0, 8)}`;
}

export default function App() {
  const [view, setView] = useState<View>("channels");
  const [session, setSession] = useState<ClientSession | null>(null);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChannelMessage[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [taskAssignments, setTaskAssignments] = useState<AssignmentWithEvents[]>([]);
  const [taskRuns, setTaskRuns] = useState<RunWithCalls[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [templates, setTemplates] = useState<TemplateDocument[]>([]);
  const [onboarding, setOnboarding] = useState<OnboardingState | null>(null);
  const [approvals, setApprovals] = useState<Approval[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [creatingChannel, setCreatingChannel] = useState(false);
  const [channelName, setChannelName] = useState("");
  const [channelTopic, setChannelTopic] = useState("");
  const [creatingTask, setCreatingTask] = useState(false);
  const [taskDescription, setTaskDescription] = useState("");
  const [taskOutput, setTaskOutput] = useState("");
  const [taskAssignee, setTaskAssignee] = useState("");
  const [taskChannel, setTaskChannel] = useState("");
  const [taskApproval, setTaskApproval] = useState<ApprovalLevel>("ask");
  const [templateId, setTemplateId] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [runningTasks, setRunningTasks] = useState<Set<string>>(() => new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement | null>(null);
  const channelNameRef = useRef<HTMLInputElement | null>(null);
  const taskDescriptionRef = useRef<HTMLTextAreaElement | null>(null);
  const updater = useUpdater();

  const selectedChannel = useMemo(
    () => channels.find((channel) => channel.id === selectedId) ?? null,
    [channels, selectedId],
  );

  const selectedTask = useMemo(
    () => tasks.find((task) => task.id === selectedTaskId) ?? null,
    [tasks, selectedTaskId],
  );

  const onboardingOpen = onboarding !== null && !onboarding.completed;

  const refreshChannels = useCallback(async (preferredId?: string) => {
    const next = await listChannels();
    setChannels(next);
    setSelectedId((current) => {
      if (preferredId && next.some((channel) => channel.id === preferredId)) {
        return preferredId;
      }
      if (current && next.some((channel) => channel.id === current)) {
        return current;
      }
      return next[0]?.id ?? null;
    });
  }, []);

  const refreshMessages = useCallback(async (channelId: string | null) => {
    if (!channelId) {
      setMessages([]);
      return;
    }
    setMessages(await listMessages(channelId));
  }, []);

  const refreshTasks = useCallback(async (preferredId?: string) => {
    const next = await listTasks();
    setTasks(next);
    setSelectedTaskId((current) => {
      if (preferredId && next.some((task) => task.id === preferredId)) {
        return preferredId;
      }
      if (current && next.some((task) => task.id === current)) {
        return current;
      }
      return next[0]?.id ?? null;
    });
  }, []);

  const refreshTaskDetail = useCallback(async (taskId: string | null) => {
    if (!taskId) {
      setTaskAssignments([]);
      setTaskRuns([]);
      return;
    }

    const [assignments, runs] = await Promise.all([
      listAssignments(taskId),
      listRuns(taskId),
    ]);
    const [assignmentsWithEvents, runsWithCalls] = await Promise.all([
      Promise.all(
        assignments.map(async (assignment) => ({
          assignment,
          events: await listAssignmentEvents(assignment.id),
        })),
      ),
      Promise.all(
        runs.map(async (run) => ({
          run,
          calls: await listToolCalls(run.id),
        })),
      ),
    ]);
    setTaskAssignments(assignmentsWithEvents);
    setTaskRuns(runsWithCalls);
  }, []);

  const refreshApprovals = useCallback(async () => {
    setApprovals(await pendingApprovals());
  }, []);

  const refreshAgents = useCallback(async () => {
    setAgents(await listAgents());
  }, []);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const [
          nextSession,
          nextChannels,
          nextTasks,
          nextApprovals,
          nextAgents,
          nextTemplates,
          nextOnboarding,
        ] = await Promise.all([
          getSession(),
          listChannels(),
          listTasks(),
          pendingApprovals(),
          listAgents(),
          listTemplates(),
          getOnboardingState(),
        ]);
        if (cancelled) return;
        setSession(nextSession);
        setChannels(nextChannels);
        setSelectedId(nextChannels[0]?.id ?? null);
        setTasks(nextTasks);
        setSelectedTaskId(nextTasks[0]?.id ?? null);
        setApprovals(nextApprovals);
        setAgents(nextAgents);
        setTemplates(nextTemplates);
        setOnboarding(nextOnboarding);
      } catch (err) {
        if (!cancelled) setError(errorMessage(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    refreshMessages(selectedId).catch((err) => setError(errorMessage(err)));
  }, [refreshMessages, selectedId]);

  useEffect(() => {
    refreshTaskDetail(selectedTaskId).catch((err) => setError(errorMessage(err)));
  }, [refreshTaskDetail, selectedTaskId]);

  useEffect(() => {
    if (!taskAssignee && agents.length > 0) setTaskAssignee(agents[0].id);
  }, [agents, taskAssignee]);

  useEffect(() => {
    if (!templateId && templates.length > 0) setTemplateId(templates[0].id);
  }, [templates, templateId]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    onCowbotEvent((event) => {
      const type = event.payload.type;
      if (type === "ChannelCreated") {
        refreshChannels().catch((err) => setError(errorMessage(err)));
      }
      if (type === "MessagePosted") {
        refreshMessages(selectedId).catch((err) => setError(errorMessage(err)));
      }
      if (
        type === "TaskCreated" ||
        type === "TaskAssigned" ||
        type === "TaskCompleted" ||
        type === "TaskFailed"
      ) {
        refreshTasks().catch((err) => setError(errorMessage(err)));
        if (selectedTaskId) {
          refreshTaskDetail(selectedTaskId).catch((err) => setError(errorMessage(err)));
        }
      }
      if (type === "ApprovalRequested" || type === "ApprovalDecided") {
        refreshApprovals().catch((err) => setError(errorMessage(err)));
      }
    })
      .then((stop) => {
        if (cancelled) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch((err) => setError(errorMessage(err)));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [
    refreshApprovals,
    refreshChannels,
    refreshMessages,
    refreshTaskDetail,
    refreshTasks,
    selectedId,
    selectedTaskId,
  ]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    onRunFinished((event) => {
      setRunningTasks((current) => {
        const next = new Set(current);
        next.delete(event.task_id);
        return next;
      });
      if (!event.ok && event.error) {
        setError(event.error);
      }
      refreshTasks().catch((err) => setError(errorMessage(err)));
      if (selectedTaskId) {
        refreshTaskDetail(selectedTaskId).catch((err) => setError(errorMessage(err)));
      }
    })
      .then((stop) => {
        if (cancelled) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch((err) => setError(errorMessage(err)));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refreshTaskDetail, refreshTasks, selectedTaskId]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [messages]);

  useEffect(() => {
    if (creatingChannel) channelNameRef.current?.focus();
  }, [creatingChannel]);

  useEffect(() => {
    if (creatingTask) taskDescriptionRef.current?.focus();
  }, [creatingTask]);

  async function handleCreateChannel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = channelName.trim();
    const topic = channelTopic.trim();
    if (!name) return;

    setError(null);
    try {
      const channel = await createChannel(name, topic);
      setCreatingChannel(false);
      setChannelName("");
      setChannelTopic("");
      await refreshChannels(channel.id);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function sendMessage() {
    const content = draft.trim();
    if (!content || !selectedId || sending) return;

    setSending(true);
    setError(null);
    try {
      await postMessage(selectedId, content);
      setDraft("");
      await refreshMessages(selectedId);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSending(false);
    }
  }

  function handleComposerKeyDown(event: ReactKeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void sendMessage();
    }
  }

  function startCreatingTask() {
    setView("tasks");
    setCreatingTask(true);
    setError(null);
  }

  async function handleCreateTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const description = taskDescription.trim();
    const expectedOutput = taskOutput.trim();
    if (!description || !expectedOutput || !taskAssignee) return;

    setBusy("create-task");
    setError(null);
    try {
      const task = await createTask({
        description,
        expectedOutput,
        assignee: taskAssignee,
        channelId: taskChannel || null,
        approvalLevel: taskApproval,
      });
      setCreatingTask(false);
      setTaskDescription("");
      setTaskOutput("");
      setTaskChannel("");
      await refreshTasks(task.id);
      setSelectedTaskId(task.id);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  async function instantiateTemplate(templateId: string) {
    const added = await instantiateAgents(templateId);
    await refreshAgents();
    if (added[0]) setTaskAssignee(added[0].id);
    return added;
  }

  async function handleAddAgents() {
    if (!templateId) return;
    setBusy("add-agents");
    setError(null);
    try {
      await instantiateTemplate(templateId);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleCompleteOnboarding() {
    const next = await completeOnboarding();
    setOnboarding(next);
  }

  async function handleCancelTask() {
    if (!selectedTaskId) return;
    if (!window.confirm("Cancel this task and its active assignment?")) return;

    setBusy("cancel-task");
    setError(null);
    try {
      await cancelTask(selectedTaskId, "Cancelled in desktop");
      await refreshTasks();
      await refreshTaskDetail(selectedTaskId);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleRunTask(taskId: string) {
    setError(null);
    setRunningTasks((current) => new Set(current).add(taskId));
    try {
      await runTask(taskId, taskApproval);
      await refreshTasks();
    } catch (err) {
      setRunningTasks((current) => {
        const next = new Set(current);
        next.delete(taskId);
        return next;
      });
      setError(errorMessage(err));
    }
  }

  async function handleDecideApproval(approval: Approval, approved: boolean) {
    setBusy(approval.id);
    setError(null);
    try {
      await decideApproval(approval.id, approved);
      await refreshApprovals();
      await refreshTasks();
      if (selectedTaskId) await refreshTaskDetail(selectedTaskId);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="app">
      <aside className="sidebar" inert={onboardingOpen}>
        <img className="wordmark" src={wordmark} alt="CowBot" />

        <div className="workspace">
          <span className="workspace-label">Workspace</span>
          <h1 className="workspace-name">{session?.workspace.name ?? "Local workspace"}</h1>
        </div>

        <nav className="primary-nav" aria-label="Views">
          <button
            type="button"
            className="primary-nav-item"
            aria-current={view === "channels" ? "page" : undefined}
            onClick={() => setView("channels")}
          >
            Channels
          </button>
          <button
            type="button"
            className="primary-nav-item"
            aria-current={view === "tasks" ? "page" : undefined}
            onClick={() => setView("tasks")}
          >
            Tasks
          </button>
          <button
            type="button"
            className="primary-nav-item"
            aria-current={view === "approvals" ? "page" : undefined}
            onClick={() => setView("approvals")}
          >
            Approvals
            {approvals.length > 0 ? <span className="nav-count">{approvals.length}</span> : null}
          </button>
        </nav>

        {view === "channels" ? (
          <nav className="channel-nav" aria-label="Channels">
            <div className="nav-heading">
              <span>Channels</span>
              <button
                type="button"
                className="text-button"
                onClick={() => setCreatingChannel(true)}
              >
                New
              </button>
            </div>

            <ul className="channel-list">
              {channels.map((channel) => {
                const active = channel.id === selectedId;
                return (
                  <li key={channel.id}>
                    <button
                      type="button"
                      className="channel-button"
                      aria-current={active ? "page" : undefined}
                      onClick={() => setSelectedId(channel.id)}
                    >
                      <span className="channel-hash" aria-hidden="true">
                        #
                      </span>
                      <span className="channel-name">{channel.name}</span>
                    </button>
                  </li>
                );
              })}
            </ul>

            {creatingChannel ? (
              <form className="new-channel" onSubmit={handleCreateChannel}>
                <label className="field">
                  <span>Name</span>
                  <input
                    ref={channelNameRef}
                    value={channelName}
                    onChange={(event) => setChannelName(event.target.value)}
                    placeholder="engineering"
                    autoComplete="off"
                  />
                </label>
                <label className="field">
                  <span>Topic</span>
                  <input
                    value={channelTopic}
                    onChange={(event) => setChannelTopic(event.target.value)}
                    placeholder="What is this channel for?"
                    autoComplete="off"
                  />
                </label>
                <div className="form-actions">
                  <button
                    type="submit"
                    className="primary-button"
                    disabled={!channelName.trim()}
                  >
                    Create channel
                  </button>
                  <button
                    type="button"
                    className="ghost-button"
                    onClick={() => setCreatingChannel(false)}
                  >
                    Cancel
                  </button>
                </div>
              </form>
            ) : (
              <button
                type="button"
                className="new-channel-button"
                onClick={() => setCreatingChannel(true)}
              >
                <span aria-hidden="true">+</span> New channel
              </button>
            )}
          </nav>
        ) : null}

        {view === "tasks" ? (
          <div className="channel-nav" aria-label="Tasks">
            <div className="nav-heading">
              <span>Tasks</span>
              <button type="button" className="text-button" onClick={startCreatingTask}>
                New
              </button>
            </div>
            <ul className="task-list">
              {tasks.map((task) => (
                <li key={task.id}>
                  <button
                    type="button"
                    className="task-button"
                    aria-current={
                      task.id === selectedTaskId && !creatingTask ? "page" : undefined
                    }
                    onClick={() => {
                      setSelectedTaskId(task.id);
                      setCreatingTask(false);
                    }}
                  >
                    <span className={`status-dot status-${taskStatusKind(task.status)}`} />
                    <span className="task-button-text">
                      <span className="task-button-title">{task.description}</span>
                      <span className="task-button-meta">
                        {taskStatusLabel(task.status)} · {agentName(agents, task.assigned_agent)}
                      </span>
                    </span>
                  </button>
                </li>
              ))}
              {tasks.length === 0 ? <li className="nav-empty">No tasks yet.</li> : null}
            </ul>
            <button type="button" className="new-channel-button" onClick={startCreatingTask}>
              <span aria-hidden="true">+</span> New task
            </button>
          </div>
        ) : null}

        {view === "approvals" ? (
          <div className="channel-nav" aria-label="Approvals">
            <div className="nav-heading">
              <span>Approvals</span>
            </div>
            <p className="nav-note">
              Pending tool calls appear in the main pane. Agents request approval while a task
              runs.
            </p>
          </div>
        ) : null}

        <div className="sidebar-footer">
          <button
            type="button"
            className="sidebar-settings"
            aria-current={view === "settings" ? "page" : undefined}
            onClick={() => setView("settings")}
          >
            <svg
              className="sidebar-settings-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
            Settings
          </button>
          <div className="sidebar-status">
            <span className="status-dot status-completed" aria-hidden="true" />
            Local workspace
          </div>
        </div>
      </aside>

      <main className="main" inert={onboardingOpen}>
        <UpdateBanner updater={updater} tasksRunning={runningTasks.size > 0} />

        {error ? (
          <div className="error-banner" role="alert">
            <span>{error}</span>
            <button type="button" className="text-button" onClick={() => setError(null)}>
              Dismiss
            </button>
          </div>
        ) : null}

        {loading ? (
          <div className="loading-state">
            <p>Opening your workspace…</p>
          </div>
        ) : view === "channels" ? (
          selectedChannel ? (
            <>
              <header className="channel-header">
                <div>
                  <h2 className="channel-title">
                    <span className="channel-hash" aria-hidden="true">
                      #
                    </span>
                    {selectedChannel.name}
                  </h2>
                  <p className="channel-topic">
                    {selectedChannel.topic.trim() || "No topic yet."}
                  </p>
                </div>
              </header>

              <section
                className="timeline"
                aria-label={`Messages in ${selectedChannel.name}`}
                key={selectedChannel.id}
              >
                {messages.length === 0 ? (
                  <div className="empty-timeline">
                    <p className="empty-title">No messages yet.</p>
                    <p>Write the first message to #{selectedChannel.name}.</p>
                  </div>
                ) : (
                  <ol className="messages">
                    {messages.map((message) => (
                      <li key={message.id} className="message">
                        <span className="message-dot" aria-hidden="true" />
                        <div className="message-head">
                          <span className={`message-sender ${message.sender.type.toLowerCase()}`}>
                            {senderLabel(message, session)}
                          </span>
                          <time className="message-time" dateTime={message.created_at}>
                            {formatTime(message.created_at)}
                          </time>
                        </div>
                        <p className="message-content">{message.content}</p>
                        {message.thread_id ? (
                          <span className="thread-note">In a thread</span>
                        ) : null}
                      </li>
                    ))}
                  </ol>
                )}
                <div ref={endRef} />
              </section>

              <form
                className="composer"
                onSubmit={(event) => {
                  event.preventDefault();
                  void sendMessage();
                }}
              >
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  onKeyDown={handleComposerKeyDown}
                  placeholder={`Write to #${selectedChannel.name}`}
                  aria-label={`Message #${selectedChannel.name}`}
                  rows={1}
                />
                <button
                  type="submit"
                  className="send-button"
                  disabled={!draft.trim() || sending}
                >
                  {sending ? "Sending…" : "Send"}
                </button>
              </form>
            </>
          ) : (
            <div className="empty-workspace">
              <span className="empty-icon-tile">
                <img className="empty-icon" src={icon} alt="" />
              </span>
              <p className="empty-title">A quiet room, for now.</p>
              <p>Create a channel to start working with your agents.</p>
              <button
                type="button"
                className="primary-button"
                onClick={() => setCreatingChannel(true)}
              >
                Create a channel
              </button>
            </div>
          )
        ) : view === "tasks" ? (
          creatingTask ? (
            <div className="task-form-view">
              <header className="view-header">
                <h2 className="view-title">New task</h2>
                <p className="view-subtitle">
                  Give an agent a clear outcome and an approval policy.
                </p>
              </header>

              {agents.length === 0 ? (
                <div className="agent-bootstrap">
                  <p className="empty-title">No agents yet.</p>
                  <p>Add a team from a built-in template to assign this task.</p>
                  <div className="inline-form">
                    <select
                      value={templateId}
                      onChange={(event) => setTemplateId(event.target.value)}
                      aria-label="Template"
                    >
                      {templates.map((template) => (
                        <option key={template.id} value={template.id}>
                          {template.name}
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      className="primary-button"
                      onClick={handleAddAgents}
                      disabled={busy === "add-agents" || !templateId}
                    >
                      {busy === "add-agents" ? "Adding…" : "Add agents"}
                    </button>
                  </div>
                </div>
              ) : (
                <form className="task-form" onSubmit={handleCreateTask}>
                  <label className="field">
                    <span>Description</span>
                    <textarea
                      ref={taskDescriptionRef}
                      value={taskDescription}
                      onChange={(event) => setTaskDescription(event.target.value)}
                      placeholder="Implement the login flow and verify it in the sandbox."
                      rows={3}
                    />
                  </label>
                  <label className="field">
                    <span>Expected output</span>
                    <textarea
                      value={taskOutput}
                      onChange={(event) => setTaskOutput(event.target.value)}
                      placeholder="Working code, tests, and a short summary of changes."
                      rows={2}
                    />
                  </label>
                  <div className="form-row">
                    <label className="field">
                      <span>Assignee</span>
                      <select
                        value={taskAssignee}
                        onChange={(event) => setTaskAssignee(event.target.value)}
                      >
                        {agents.map((agent) => (
                          <option key={agent.id} value={agent.id}>
                            {agent.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="field">
                      <span>Channel</span>
                      <select
                        value={taskChannel}
                        onChange={(event) => setTaskChannel(event.target.value)}
                      >
                        <option value="">No channel</option>
                        {channels.map((channel) => (
                          <option key={channel.id} value={channel.id}>
                            #{channel.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="field">
                      <span>Approval</span>
                      <select
                        value={taskApproval}
                        onChange={(event) =>
                          setTaskApproval(event.target.value as ApprovalLevel)
                        }
                      >
                        {approvalLevels.map((level) => (
                          <option key={level} value={level}>
                            {approvalLevelLabel(level)}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                  <div className="form-actions">
                    <button
                      type="submit"
                      className="primary-button"
                      disabled={
                        busy === "create-task" ||
                        !taskDescription.trim() ||
                        !taskOutput.trim() ||
                        !taskAssignee
                      }
                    >
                      {busy === "create-task" ? "Creating…" : "Create task"}
                    </button>
                    <button
                      type="button"
                      className="ghost-button"
                      onClick={() => setCreatingTask(false)}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              )}
            </div>
          ) : selectedTask ? (
            <TaskDetail
              task={selectedTask}
              assignments={taskAssignments}
              runs={taskRuns}
              cancelling={busy === "cancel-task"}
              onCancel={handleCancelTask}
              running={runningTasks.has(selectedTask.id)}
              onRun={() => handleRunTask(selectedTask.id)}
            />
          ) : (
            <div className="empty-workspace">
              <p className="empty-title">No task selected.</p>
              <p>Create a task and choose an agent to own it.</p>
              <button type="button" className="primary-button" onClick={startCreatingTask}>
                New task
              </button>
            </div>
          )
        ) : view === "approvals" ? (
          <ApprovalCards
            approvals={approvals}
            tasks={tasks}
            decidingId={busy}
            onDecide={handleDecideApproval}
          />
        ) : (
          <SettingsView
            session={session}
            agents={agents}
            templates={templates}
            templateId={templateId}
            busy={busy}
            updater={updater}
            tasksRunning={runningTasks.size > 0}
            onSelectTemplate={setTemplateId}
            onAddAgents={handleAddAgents}
          />
        )}
      </main>

      {onboardingOpen ? (
        <Onboarding
          templates={templates}
          agents={agents}
          onAddAgents={instantiateTemplate}
          onComplete={handleCompleteOnboarding}
        />
      ) : null}
    </div>
  );
}
