use std::sync::Arc;

use cow_agent::{AgentInstance, Crew, ProcessType, StandardRoles};
use cow_bus::EventBus;
use cow_core::approval::ApprovalLevel;
use cow_core::assignment::{
    AssignmentEventKind, AssignmentOrigin, AssignmentRequester, AssignmentStatus,
};
use cow_core::task::Task;
use cow_llm::MockLlmProvider;
use cow_service::{bootstrap_local, CrewRunner};
use cow_store::Store;
use cow_tools::ToolRegistry;

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
    }
}
