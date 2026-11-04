import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, KeyboardEvent as ReactKeyboardEvent } from "react";

import icon from "./assets/cowai-icon.svg";
import wordmark from "./assets/cowai-wordmark-dark.svg";
import {
  createChannel,
  getSession,
  listChannels,
  listMessages,
  onCowaiEvent,
  postMessage,
} from "./api";
import type { Channel, ChannelMessage, ClientSession } from "./types";

const messageTime = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
});

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong. Try again.";
}

function senderLabel(message: ChannelMessage, session: ClientSession | null): string {
  const sender = message.sender;
  if (sender.type === "System") return "System";
  if (sender.type === "User") {
    return session && sender.id === session.user.id ? "You" : "Teammate";
  }
  return `Agent ${sender.id.slice(0, 8)}`;
}

export default function App() {
  const [session, setSession] = useState<ClientSession | null>(null);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChannelMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [creating, setCreating] = useState(false);
  const [channelName, setChannelName] = useState("");
  const [channelTopic, setChannelTopic] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement | null>(null);
  const nameInputRef = useRef<HTMLInputElement | null>(null);

  const selectedChannel = useMemo(
    () => channels.find((channel) => channel.id === selectedId) ?? null,
    [channels, selectedId],
  );

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

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const [nextSession, nextChannels] = await Promise.all([getSession(), listChannels()]);
        if (cancelled) return;
        setSession(nextSession);
        setChannels(nextChannels);
        setSelectedId(nextChannels[0]?.id ?? null);
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
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    onCowaiEvent((event) => {
      if (event.payload.type === "ChannelCreated") {
        refreshChannels().catch((err) => setError(errorMessage(err)));
      }
      if (event.payload.type === "MessagePosted") {
        refreshMessages(selectedId).catch((err) => setError(errorMessage(err)));
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
  }, [refreshChannels, refreshMessages, selectedId]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [messages]);

  useEffect(() => {
    if (creating) nameInputRef.current?.focus();
  }, [creating]);

  async function handleCreateChannel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = channelName.trim();
    const topic = channelTopic.trim();
    if (!name) return;

    setError(null);
    try {
      const channel = await createChannel(name, topic);
      setCreating(false);
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

  return (
    <div className="app">
      <aside className="sidebar">
        <img className="wordmark" src={wordmark} alt="CowAI" />

        <div className="workspace">
          <span className="workspace-label">Workspace</span>
          <h1 className="workspace-name">{session?.workspace.name ?? "Local workspace"}</h1>
        </div>

        <nav className="channel-nav" aria-label="Channels">
          <div className="nav-heading">
            <span>Channels</span>
            <button type="button" className="text-button" onClick={() => setCreating(true)}>
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

          {creating ? (
            <form className="new-channel" onSubmit={handleCreateChannel}>
              <label className="field">
                <span>Name</span>
                <input
                  ref={nameInputRef}
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
                <button type="submit" className="primary-button" disabled={!channelName.trim()}>
                  Create channel
                </button>
                <button type="button" className="ghost-button" onClick={() => setCreating(false)}>
                  Cancel
                </button>
              </div>
            </form>
          ) : (
            <button type="button" className="new-channel-button" onClick={() => setCreating(true)}>
              <span aria-hidden="true">+</span> New channel
            </button>
          )}
        </nav>

        <div className="sidebar-footer">
          <span className="status-dot" aria-hidden="true" />
          Local workspace
        </div>
      </aside>

      <main className="main">
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
        ) : selectedChannel ? (
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
                          {messageTime.format(new Date(message.created_at))}
                        </time>
                      </div>
                      <p className="message-content">{message.content}</p>
                      {message.thread_id ? <span className="thread-note">In a thread</span> : null}
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
              <button type="submit" className="send-button" disabled={!draft.trim() || sending}>
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
            <button type="button" className="primary-button" onClick={() => setCreating(true)}>
              Create a channel
            </button>
          </div>
        )}
      </main>
    </div>
  );
}
