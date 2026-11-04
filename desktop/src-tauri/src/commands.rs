use cow_client::{Client, NewTask};
use cow_core::agent::{AgentId, AgentProfile};
use cow_core::approval::{Approval, ApprovalId, ApprovalLevel};
use cow_core::assignment::{Assignment, AssignmentEvent, AssignmentId};
use cow_core::channel::{Channel, ChannelId, ChannelMessage};
use cow_core::run::{Run, RunId, ToolCallRecord};
use cow_core::task::{Task, TaskId};
use cow_core::template::TemplateDocument;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_session(state: State<'_, AppState>) -> Result<cow_client::ClientSession, String> {
    state
        .client
        .session()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_channels(state: State<'_, AppState>) -> Result<Vec<Channel>, String> {
    state
        .client
        .list_channels()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_channel(
    state: State<'_, AppState>,
    name: String,
    topic: String,
) -> Result<Channel, String> {
    state
        .client
        .create_channel(&name, &topic)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_messages(
    state: State<'_, AppState>,
    channel_id: String,
    limit: u32,
) -> Result<Vec<ChannelMessage>, String> {
    state
        .client
        .list_messages(&ChannelId::new(channel_id), limit)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn post_message(
    state: State<'_, AppState>,
    channel_id: String,
    content: String,
) -> Result<ChannelMessage, String> {
    state
        .client
        .post_message(&ChannelId::new(channel_id), &content)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_agents(state: State<'_, AppState>) -> Result<Vec<AgentProfile>, String> {
    state
        .client
        .list_agents()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_templates(state: State<'_, AppState>) -> Result<Vec<TemplateDocument>, String> {
    state
        .client
        .list_templates()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn instantiate_agents(
    state: State<'_, AppState>,
    template_id: String,
) -> Result<Vec<AgentProfile>, String> {
    state
        .client
        .instantiate_agents(&template_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_task(
    state: State<'_, AppState>,
    description: String,
    expected_output: String,
    assignee: String,
    channel_id: Option<String>,
    approval_level: String,
) -> Result<Task, String> {
    let approval_level = parse_approval_level(&approval_level)?;
    let mut task = NewTask::new(description, expected_output, AgentId::new(assignee));
    if let Some(channel_id) = channel_id.filter(|id| !id.trim().is_empty()) {
        task = task.in_channel(ChannelId::new(channel_id));
    }
    task = task.with_approval_level(approval_level);

    state
        .client
        .create_task(&task)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    state
        .client
        .list_tasks()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cancel_task(
    state: State<'_, AppState>,
    task_id: String,
    reason: String,
) -> Result<(), String> {
    state
        .client
        .cancel_task(&TaskId::new(task_id), &reason)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_assignments(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Vec<Assignment>, String> {
    state
        .client
        .list_assignments(&TaskId::new(task_id))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_assignment_events(
    state: State<'_, AppState>,
    assignment_id: String,
) -> Result<Vec<AssignmentEvent>, String> {
    state
        .client
        .list_assignment_events(&AssignmentId::new(assignment_id))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_runs(state: State<'_, AppState>, task_id: String) -> Result<Vec<Run>, String> {
    state
        .client
        .list_runs(&TaskId::new(task_id))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_tool_calls(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<ToolCallRecord>, String> {
    state
        .client
        .list_tool_calls(&RunId::new(run_id))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn pending_approvals(state: State<'_, AppState>) -> Result<Vec<Approval>, String> {
    state
        .client
        .pending_approvals()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn decide_approval(
    state: State<'_, AppState>,
    approval_id: String,
    approved: bool,
) -> Result<Approval, String> {
    state
        .client
        .decide_approval(&ApprovalId::new(approval_id), approved)
        .await
        .map_err(|error| error.to_string())
}

fn parse_approval_level(value: &str) -> Result<ApprovalLevel, String> {
    match value {
        "ask" => Ok(ApprovalLevel::Ask),
        "safe_auto" => Ok(ApprovalLevel::SafeAuto),
        "plan_only" => Ok(ApprovalLevel::PlanOnly),
        "trusted" => Ok(ApprovalLevel::Trusted),
        other => Err(format!("unknown approval level '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_levels_parse() {
        assert_eq!(parse_approval_level("ask"), Ok(ApprovalLevel::Ask));
        assert_eq!(
            parse_approval_level("safe_auto"),
            Ok(ApprovalLevel::SafeAuto)
        );
        assert_eq!(
            parse_approval_level("plan_only"),
            Ok(ApprovalLevel::PlanOnly)
        );
        assert_eq!(parse_approval_level("trusted"), Ok(ApprovalLevel::Trusted));
    }

    #[test]
    fn unknown_approval_level_is_rejected() {
        assert!(parse_approval_level("sometimes").is_err());
    }
}
