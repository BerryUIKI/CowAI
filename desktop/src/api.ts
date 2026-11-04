import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Channel, ChannelMessage, ClientSession, CowEvent } from "./types";

export function getSession(): Promise<ClientSession> {
  return invoke<ClientSession>("get_session");
}

export function listChannels(): Promise<Channel[]> {
  return invoke<Channel[]>("list_channels");
}

export function createChannel(name: string, topic: string): Promise<Channel> {
  return invoke<Channel>("create_channel", { name, topic });
}

export function listMessages(channelId: string, limit = 200): Promise<ChannelMessage[]> {
  return invoke<ChannelMessage[]>("list_messages", { channelId, limit });
}

export function postMessage(channelId: string, content: string): Promise<ChannelMessage> {
  return invoke<ChannelMessage>("post_message", { channelId, content });
}

export function onCowaiEvent(handler: (event: CowEvent) => void): Promise<UnlistenFn> {
  return listen<CowEvent>("cowai-event", (event) => handler(event.payload));
}
