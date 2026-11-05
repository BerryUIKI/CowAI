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
  ModelSettings,
  OnboardingState,
  ProviderProtocol,
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

export function onCowbotEvent(handler: (event: CowEvent) => void): Promise<UnlistenFn> {
  return listen<CowEvent>("cowbot-event", (event) => handler(event.payload));
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

export function runTask(taskId: string, approvalLevel: ApprovalLevel): Promise<void> {
  return invoke<void>("run_task", { taskId, approvalLevel });
}

export interface RunFinished {
  task_id: string;
  ok: boolean;
  error: string | null;
}

export function onRunFinished(handler: (event: RunFinished) => void): Promise<UnlistenFn> {
  return listen<RunFinished>("cowbot-run-finished", (event) => handler(event.payload));
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

export function getModelSettings(): Promise<ModelSettings> {
  return invoke<ModelSettings>("get_model_settings");
}

export function saveModelProvider(input: {
  providerId: string | null;
  protocol: ProviderProtocol;
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  clearApiKey: boolean;
}): Promise<ModelSettings> {
  return invoke<ModelSettings>("save_model_provider", {
    draft: {
      providerId: input.providerId,
      protocol: input.protocol,
      name: input.name,
      baseUrl: input.baseUrl,
      model: input.model,
      apiKey: input.apiKey.trim() ? input.apiKey : null,
      clearApiKey: input.clearApiKey,
    },
  });
}

export function deleteModelProvider(providerId: string): Promise<ModelSettings> {
  return invoke<ModelSettings>("delete_model_provider", { providerId });
}

export function activateModelProvider(providerId: string | null): Promise<ModelSettings> {
  return invoke<ModelSettings>("activate_model_provider", { providerId });
}

export function testModelProvider(input: {
  providerId: string | null;
  protocol: ProviderProtocol;
  baseUrl: string;
  model: string;
  apiKey: string;
}): Promise<string> {
  return invoke<string>("test_model_provider", {
    providerId: input.providerId,
    protocol: input.protocol,
    baseUrl: input.baseUrl,
    model: input.model,
    apiKey: input.apiKey.trim() ? input.apiKey : null,
  });
}

export function getOnboardingState(): Promise<OnboardingState> {
  return invoke<OnboardingState>("get_onboarding_state");
}

export function completeOnboarding(): Promise<OnboardingState> {
  return invoke<OnboardingState>("complete_onboarding");
}
