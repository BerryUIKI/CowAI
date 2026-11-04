import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AgentProfile,
  Approval,
  ApprovalLevel,
  Assignment,
  AssignmentEvent,
  Channel,
  ChannelMessage,
  ClientSession,
  CowEvent,
  Run,
  Task,
  TemplateDocument,
  ToolCallRecord,
} from "./types";

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

export function listAgents(): Promise<AgentProfile[]> {
  return invoke<AgentProfile[]>("list_agents");
}

export function listTemplates(): Promise<TemplateDocument[]> {
  return invoke<TemplateDocument[]>("list_templates");
}

export function instantiateAgents(templateId: string): Promise<AgentProfile[]> {
  return invoke<AgentProfile[]>("instantiate_agents", { templateId });
}

export function createTask(input: {
  description: string;
  expectedOutput: string;
  assignee: string;
  channelId: string | null;
  approvalLevel: ApprovalLevel;
}): Promise<Task> {
  return invoke<Task>("create_task", {
    description: input.description,
    expectedOutput: input.expectedOutput,
    assignee: input.assignee,
    channelId: input.channelId,
    approvalLevel: input.approvalLevel,
  });
}

export function listTasks(): Promise<Task[]> {
  return invoke<Task[]>("list_tasks");
}

export function cancelTask(taskId: string, reason: string): Promise<void> {
  return invoke<void>("cancel_task", { taskId, reason });
}

export function listAssignments(taskId: string): Promise<Assignment[]> {
  return invoke<Assignment[]>("list_assignments", { taskId });
}

export function listAssignmentEvents(assignmentId: string): Promise<AssignmentEvent[]> {
  return invoke<AssignmentEvent[]>("list_assignment_events", { assignmentId });
}

export function listRuns(taskId: string): Promise<Run[]> {
  return invoke<Run[]>("list_runs", { taskId });
}

export function listToolCalls(runId: string): Promise<ToolCallRecord[]> {
  return invoke<ToolCallRecord[]>("list_tool_calls", { runId });
}

export function pendingApprovals(): Promise<Approval[]> {
  return invoke<Approval[]>("pending_approvals");
}

export function decideApproval(approvalId: string, approved: boolean): Promise<Approval> {
  return invoke<Approval>("decide_approval", { approvalId, approved });
}
