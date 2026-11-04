use std::sync::Arc;

use cow_agent::{AgentInstance, Crew, ProcessType, StandardRoles};
use cow_bus::EventBus;
use cow_core::approval::ApprovalLevel;
use cow_core::assignment::{
    AssignmentEventKind, AssignmentOrigin, AssignmentRequester, AssignmentStatus,
};
use cow_core::run::RunStatus;
use cow_core::task::Task;
use cow_llm::{ChatCompletionResponse, LlmToolCall, MockLlmProvider};
use cow_service::{bootstrap_local, CrewRunner};
use cow_store::Store;
use cow_tools::ToolRegistry;
use serde_json::json;

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
