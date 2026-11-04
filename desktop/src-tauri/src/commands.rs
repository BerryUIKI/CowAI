use cowbot_agent::{AgentInstance, Crew, ProcessType, StandardRoles};
use cowbot_client::{Client, NewTask};
use cowbot_core::agent::{AgentId, AgentProfile};
use cowbot_core::approval::{Approval, ApprovalId, ApprovalLevel};
use cowbot_core::assignment::{Assignment, AssignmentEvent, AssignmentId};
use cowbot_core::channel::{Channel, ChannelId, ChannelMessage};
use cowbot_core::run::{Run, RunId, ToolCallRecord};
use cowbot_core::task::{Task, TaskId};
use cowbot_core::template::TemplateDocument;
use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::AppState;

/// Payload emitted on `cowbot-run-finished` when a crew run settles.
#[derive(Debug, Clone, Serialize)]
pub struct RunFinished {
    pub task_id: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn get_session(
    state: State<'_, AppState>,
) -> Result<cowbot_client::ClientSession, String> {
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
pub async fn run_task(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    approval_level: String,
) -> Result<(), String> {
    let approval_level = parse_approval_level(&approval_level)?;
    let task = state
        .client
        .get_task(&TaskId::new(task_id.clone()))
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("task '{task_id}' was not found"))?;

    {
        let mut running = state.running.lock().map_err(|error| error.to_string())?;
        if !running.insert(task_id.clone()) {
            return Err(format!("task '{task_id}' is already running"));
        }
    }

    let client = state.client.clone();
    let llm = state.llm.clone();
    let tools = state.tools.clone();
    let model = state.model.clone();
    let run_id = task_id.clone();

    tauri::async_runtime::spawn(async move {
        let outcome = async {
            let mut crew = build_standard_crew(&task.description, &llm, &tools, &model);
            client.run_crew(&mut crew, approval_level).await
        }
        .await;

        if let Some(state) = app.try_state::<AppState>() {
            if let Ok(mut running) = state.running.lock() {
                running.remove(&run_id);
            }
        }

        let payload = match outcome {
            Ok(_) => RunFinished {
                task_id: run_id,
                ok: true,
                error: None,
            },
            Err(error) => RunFinished {
                task_id: run_id,
                ok: false,
                error: Some(error.to_string()),
            },
        };
        let _ = app.emit("cowbot-run-finished", payload);
    });

    Ok(())
}

/// Build the Planner/Coder/Reviewer pipeline used by the CLI, with the
/// submitted task's description as the objective.
fn build_standard_crew(
    objective: &str,
    llm: &std::sync::Arc<dyn cowbot_llm::LlmProvider>,
    tools: &cowbot_tools::ToolRegistry,
    model: &str,
) -> Crew {
    let planner_profile = StandardRoles::planner();
    let planner_id = planner_profile.id.clone();
    let planner = AgentInstance::new(planner_profile, llm.clone(), tools.clone(), model);

    let coder_profile = StandardRoles::coder();
    let coder_id = coder_profile.id.clone();
    let coder = AgentInstance::new(coder_profile, llm.clone(), tools.clone(), model);

    let reviewer_profile = StandardRoles::reviewer();
    let reviewer_id = reviewer_profile.id.clone();
    let reviewer = AgentInstance::new(reviewer_profile, llm.clone(), tools.clone(), model);

    let mut crew = Crew::new(ProcessType::Sequential);
    crew.add_agent(planner);
    crew.add_agent(coder);
    crew.add_agent(reviewer);

    let plan_task = Task::new(
        format!("Analyze objective and formulate architecture plan: {objective}"),
        "A clear, step-by-step implementation and verification plan.",
    )
    .assign_to(planner_id);
    let plan_id = plan_task.id.clone();

    let code_task = Task::new(
        "Execute code implementation and verify functionality in the sandbox.",
        "Working source code and sandbox execution results.",
    )
    .assign_to(coder_id)
    .with_context_task(plan_id);

    let review_task = Task::new(
        "Review implementation, tests, and diffs for correctness and edge cases.",
        "Comprehensive review findings and final approval status.",
    )
    .assign_to(reviewer_id)
    .with_context_task(code_task.id.clone());

    crew.add_task(plan_task);
    crew.add_task(code_task);
    crew.add_task(review_task);
    crew
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
