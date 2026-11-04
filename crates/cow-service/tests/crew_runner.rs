use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use cow_agent::{AgentInstance, Crew, ProcessType, StandardRoles};
use cow_bus::EventBus;
use cow_core::approval::{ApprovalLevel, ApprovalStatus};
use cow_core::assignment::{
    AssignmentEventKind, AssignmentOrigin, AssignmentRequester, AssignmentStatus,
};
use cow_core::error::CowError;
use cow_core::run::RunStatus;
use cow_core::task::Task;
use cow_core::tool::{Tool, ToolCall, ToolResult};
use cow_llm::{ChatCompletionResponse, LlmToolCall, MockLlmProvider};
use cow_service::{bootstrap_local, ApprovalService, CrewRunner};
use cow_store::Store;
use cow_tools::ToolRegistry;
use serde_json::json;

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo_tool"
    }

    fn description(&self) -> &str {
        "Echoes a value"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({ "type": "object" })
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, CowError> {
        Ok(ToolResult::success(call.id.clone(), "echoed"))
    }
}

#[tokio::test]
async fn crew_runner_executes_tasks_through_assignments() {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let context = bootstrap_local(&store).await.unwrap();
    let bus = EventBus::default_bus();
    let runner = CrewRunner::new(store.clone(), bus);

    let llm = Arc::new(MockLlmProvider::echo());
    let tools = ToolRegistry::new();

    let planner_profile = StandardRoles::planner();
    let planner_id = planner_profile.id.clone();
    let coder_profile = StandardRoles::coder();
    let coder_id = coder_profile.id.clone();

    let mut crew = Crew::new(ProcessType::Sequential);
    crew.add_agent(AgentInstance::new(
        planner_profile,
        llm.clone(),
        tools.clone(),
        "mock-model",
    ));
    crew.add_agent(AgentInstance::new(coder_profile, llm, tools, "mock-model"));

    let plan_task = Task::new("Plan the change", "An implementation plan").assign_to(planner_id);
    let plan_id = plan_task.id.clone();
    let code_task = Task::new("Implement the change", "Working code")
        .assign_to(coder_id)
        .with_context_task(plan_id.clone());
    let code_id = code_task.id.clone();

    crew.add_task(plan_task);
    crew.add_task(code_task);

    let results = runner
        .run(
            &mut crew,
            &context.workspace.id,
            AssignmentRequester::User(context.user.id.0.clone()),
            AssignmentOrigin::User,
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.contains_key(&plan_id));
    assert!(results.contains_key(&code_id));

    for task_id in [&plan_id, &code_id] {
        let assignments = store.list_assignments_for_task(task_id).await.unwrap();
        assert_eq!(assignments.len(), 1);
        let assignment = &assignments[0];
        assert_eq!(assignment.status, AssignmentStatus::Completed);
        assert_eq!(assignment.executions, 1);

        let events = store.list_assignment_events(&assignment.id).await.unwrap();
        let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                AssignmentEventKind::Enqueued,
                AssignmentEventKind::Started,
                AssignmentEventKind::Completed,
            ]
        );

        let runs = store.list_runs(task_id).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, RunStatus::Completed);
        assert!(runs[0].finished_at.is_some());

        let expected_agent = crew
            .tasks
            .iter()
            .find(|task| task.id == *task_id)
            .and_then(|task| task.assigned_agent.clone())
            .unwrap();
        assert_eq!(runs[0].agent_id, expected_agent);
    }
}

#[tokio::test]
async fn failed_task_leaves_a_failed_run() {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let context = bootstrap_local(&store).await.unwrap();
    let bus = EventBus::default_bus();
    let runner = CrewRunner::new(store.clone(), bus);

    let mut profile = StandardRoles::coder();
    profile.max_iterations = 1;
    let agent_id = profile.id.clone();
    let llm = Arc::new(MockLlmProvider::new(vec![ChatCompletionResponse {
        content: Some("calling a tool".to_string()),
        tool_calls: vec![LlmToolCall {
            id: "call-1".to_string(),
            name: "execute_code".to_string(),
            arguments: json!({ "command": "echo hi" }),
        }],
        finish_reason: Some("tool_calls".to_string()),
    }]));

    let mut crew = Crew::new(ProcessType::Sequential);
    crew.add_agent(AgentInstance::new(
        profile,
        llm,
        ToolRegistry::new(),
        "mock-model",
    ));
    let task = Task::new("Exhaust the iteration limit", "A failure").assign_to(agent_id);
    let task_id = task.id.clone();
    crew.add_task(task);

    let result = runner
        .run(
            &mut crew,
            &context.workspace.id,
            AssignmentRequester::User(context.user.id.0.clone()),
            AssignmentOrigin::User,
            ApprovalLevel::Trusted,
        )
        .await;
    assert!(result.is_err());

    let runs = store.list_runs(&task_id).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, RunStatus::Failed);
    assert!(runs[0].finished_at.is_some());

    let assignments = store.list_assignments_for_task(&task_id).await.unwrap();
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].status, AssignmentStatus::Failed);
}

#[tokio::test]
async fn crew_runner_records_tool_calls() {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let context = bootstrap_local(&store).await.unwrap();
    let bus = EventBus::default_bus();
    let runner = CrewRunner::new(store.clone(), bus);

    let responses = vec![
        ChatCompletionResponse {
            content: Some("calling the tool".to_string()),
            tool_calls: vec![LlmToolCall {
                id: "call-1".to_string(),
                name: "echo_tool".to_string(),
                arguments: json!({ "value": "hi" }),
            }],
            finish_reason: Some("tool_calls".to_string()),
        },
        ChatCompletionResponse {
            content: Some("finished".to_string()),
            tool_calls: Vec::new(),
            finish_reason: Some("stop".to_string()),
        },
    ];
    let llm = Arc::new(MockLlmProvider::new(responses));
    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(EchoTool));

    let profile = StandardRoles::coder();
    let agent_id = profile.id.clone();
    let mut crew = Crew::new(ProcessType::Sequential);
    crew.add_agent(AgentInstance::new(profile, llm, tools, "mock-model"));

    let task = Task::new("Call the echo tool", "Echoed output").assign_to(agent_id);
    let task_id = task.id.clone();
    crew.add_task(task);

    let results = runner
        .run(
            &mut crew,
            &context.workspace.id,
            AssignmentRequester::User(context.user.id.0.clone()),
            AssignmentOrigin::User,
            ApprovalLevel::Trusted,
        )
        .await
        .unwrap();
    assert_eq!(results.len(), 1);

    let runs = store.list_runs(&task_id).await.unwrap();
    assert_eq!(runs.len(), 1);
    let calls = store.list_tool_calls(&runs[0].id).await.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call-1");
    assert_eq!(calls[0].name, "echo_tool");
    assert_eq!(calls[0].arguments, json!({ "value": "hi" }));
    assert_eq!(calls[0].approved_by, None);

    let result = calls[0].result.as_ref().unwrap();
    assert_eq!(result["success"], json!(true));
    assert_eq!(result["output"], json!("echoed"));
}

#[tokio::test]
async fn crew_runner_records_the_human_approver() {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let context = bootstrap_local(&store).await.unwrap();
    let bus = EventBus::default_bus();
    let runner = CrewRunner::new(store.clone(), bus.clone());

    let responses = vec![
        ChatCompletionResponse {
            content: Some("calling the tool".to_string()),
            tool_calls: vec![LlmToolCall {
                id: "call-approved".to_string(),
                name: "echo_tool".to_string(),
                arguments: json!({ "value": "hi" }),
            }],
            finish_reason: Some("tool_calls".to_string()),
        },
        ChatCompletionResponse {
            content: Some("finished".to_string()),
            tool_calls: Vec::new(),
            finish_reason: Some("stop".to_string()),
        },
    ];
    let llm = Arc::new(MockLlmProvider::new(responses));
    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(EchoTool));

    let profile = StandardRoles::coder();
    let agent_id = profile.id.clone();
    let mut crew = Crew::new(ProcessType::Sequential);
    crew.add_agent(AgentInstance::new(profile, llm, tools, "mock-model"));

    let task = Task::new("Approve the echo tool", "Echoed output").assign_to(agent_id);
    let task_id = task.id.clone();
    crew.add_task(task);

    let decider = ApprovalService::new(store.clone(), bus, context.workspace.id.clone());
    let decide_store = store.clone();
    let decide_task_id = task_id.clone();
    tokio::spawn(async move {
        for _ in 0..100 {
            let pending = decide_store
                .list_pending_approvals(&decide_task_id)
                .await
                .unwrap();
            if let Some(approval) = pending.first() {
                decider
                    .decide(&approval.id, ApprovalStatus::Approved, "user-1")
                    .await
                    .unwrap();
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("no pending approval appeared");
    });

    let results = tokio::time::timeout(
        Duration::from_secs(5),
        runner.run(
            &mut crew,
            &context.workspace.id,
            AssignmentRequester::User(context.user.id.0.clone()),
            AssignmentOrigin::User,
            ApprovalLevel::Ask,
        ),
    )
    .await
    .expect("crew runner timed out")
    .unwrap();
    assert_eq!(results.len(), 1);

    let runs = store.list_runs(&task_id).await.unwrap();
    assert_eq!(runs.len(), 1);
    let calls = store.list_tool_calls(&runs[0].id).await.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call-approved");
    assert_eq!(calls[0].approved_by.as_deref(), Some("user-1"));
    assert_eq!(calls[0].result.as_ref().unwrap()["success"], json!(true));
}
