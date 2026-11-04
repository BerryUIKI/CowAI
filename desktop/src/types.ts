export interface ClientUser {
  id: string;
  name: string;
  email: string;
}

export interface Workspace {
  id: string;
  name: string;
  created_at: string;
}

export interface ClientSession {
  user: ClientUser;
  workspace: Workspace;
}

export interface Channel {
  id: string;
  name: string;
  topic: string;
  created_at: string;
}

export type Sender =
  | { type: "Agent"; id: string }
  | { type: "User"; id: string }
  | { type: "System" };

export interface ChannelMessage {
  id: string;
  channel_id: string;
  sender: Sender;
  content: string;
  thread_id: string | null;
  created_at: string;
}

export interface CowEvent {
  id: string;
  timestamp: string;
  payload: {
    type: string;
    payload?: unknown;
  };
}
