use cowbot_bus::EventBus;
use cowbot_core::agent::AgentId;
use cowbot_core::approval::ApprovalLevel;
use cowbot_core::assignment::{
    Assignment, AssignmentEventKind, AssignmentMode, AssignmentOrigin, AssignmentOutcome,
    AssignmentRequester, AssignmentStatus, ReplyMode,
};
use cowbot_core::identity::WorkspaceId;
use cowbot_core::task::{Task, TaskStatus};
use cowbot_service::{bootstrap_local, AssignmentService};
use cowbot_store::Store;

async fn test_store() -> Store {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn setup() -> (Store, AssignmentService, WorkspaceId, AgentId) {
    let store = test_store().await;
    let context = bootstrap_local(&store).await.unwrap();
    let bus = EventBus::default_bus();
    let service = AssignmentService::new(store.clone(), bus);
    let agent = AgentId::new("agent-1");
    (store, service, context.workspace.id, agent)
}

async fn make_task(
    store: &Store,
    workspace_id: &WorkspaceId,
    agent: &AgentId,
    description: &str,
) -> Task {
    let task = Task::new(description, "A verified result").assign_to(agent.clone());
    store.create_task(workspace_id, None, &task).await.unwrap();
    task
}

#[tokio::test]
async fn assignment_lifecycle_records_events() {
    let (store, service, workspace_id, agent) = setup().await;
    let task = make_task(&store, &workspace_id, &agent, "Implement login").await;
    let requester = AssignmentRequester::User("user-1".to_string());

    let assignment = service
        .enqueue_root(
            &task,
            AssignmentOrigin::User,
            requester,
            agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    assert_eq!(assignment.status, AssignmentStatus::Queued);
    assert_eq!(assignment.root_id, assignment.id);
    assert_eq!(assignment.depth, 0);

    let started = service.start(&assignment.id).await.unwrap();
    assert_eq!(started.status, AssignmentStatus::Running);
    assert_eq!(started.executions, 1);
    assert_eq!(
        store.get_task(&task.id).await.unwrap().unwrap().status,
        TaskStatus::InProgress
    );

    let waiting = service.mark_waiting(&assignment.id).await.unwrap();
    assert_eq!(waiting.status, AssignmentStatus::Waiting);

    let resumed = service.resume(&assignment.id).await.unwrap();
    assert_eq!(resumed.status, AssignmentStatus::Resume);

    let running = service.start(&assignment.id).await.unwrap();
    assert_eq!(running.executions, 2);

    let completed = service
        .complete(&assignment.id, "Login implemented")
        .await
        .unwrap();
    assert_eq!(completed.status, AssignmentStatus::Completed);
    match completed.outcome {
        Some(AssignmentOutcome::Completed { output }) => {
            assert_eq!(output, "Login implemented")
        }
        other => panic!("expected completed outcome, got {other:?}"),
    }
    assert_eq!(
        store.get_task(&task.id).await.unwrap().unwrap().status,
        TaskStatus::Completed {
            output: "Login implemented".to_string()
        }
    );

    let events = service.events(&assignment.id).await.unwrap();
    let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
    assert_eq!(
        kinds,
        vec![
            AssignmentEventKind::Enqueued,
            AssignmentEventKind::Started,
            AssignmentEventKind::Waiting,
            AssignmentEventKind::Resumed,
            AssignmentEventKind::Started,
            AssignmentEventKind::Completed,
        ]
    );
}

#[tokio::test]
async fn delegation_builds_tree_and_rejects_ancestor_work() {
    let (store, service, workspace_id, agent) = setup().await;
    let root_task = make_task(&store, &workspace_id, &agent, "Plan the work").await;
    let root = service
        .enqueue_root(
            &root_task,
            AssignmentOrigin::User,
            AssignmentRequester::User("user-1".to_string()),
            agent.clone(),
            ApprovalLevel::Trusted,
        )
        .await
        .unwrap();

    let child_agent = AgentId::new("agent-2");
    let child_task = make_task(&store, &workspace_id, &child_agent, "Implement the plan").await;
    let child = service
        .delegate(
            &root.id,
            &child_task,
            AssignmentRequester::User("user-1".to_string()),
            child_agent,
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    assert_eq!(child.depth, 1);
    assert_eq!(child.parent_id, Some(root.id.clone()));
    assert_eq!(child.root_id, root.id);
    assert_eq!(child.approval_level, ApprovalLevel::Trusted);

    let child_agent = AgentId::new("agent-3");
    let third_task = make_task(&store, &workspace_id, &child_agent, "Review the plan").await;
    let result = service
        .delegate(
            &child.id,
            &third_task,
            AssignmentRequester::Agent(agent.clone()),
            child_agent,
            ApprovalLevel::Ask,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delegation_respects_request_budget() {
    let (store, service, workspace_id, agent) = setup().await;
    let task = make_task(&store, &workspace_id, &agent, "Root").await;
    let mut root = Assignment::new_root(
        task.id.clone(),
        AssignmentOrigin::User,
        AssignmentRequester::User("user-1".to_string()),
        agent.clone(),
        AssignmentMode::Work,
        ReplyMode::Resume,
        ApprovalLevel::Ask,
    );
    root.budgets.requests = 1;
    store.create_assignment(&root).await.unwrap();

    let child_task = make_task(&store, &workspace_id, &agent, "Child").await;
    let result = service
        .delegate(
            &root.id,
            &child_task,
            AssignmentRequester::User("user-1".to_string()),
            AgentId::new("agent-2"),
            ApprovalLevel::Ask,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cancellation_cascades_to_descendants() {
    let (store, service, workspace_id, agent) = setup().await;
    let root_task = make_task(&store, &workspace_id, &agent, "Root").await;
    let root = service
        .enqueue_root(
            &root_task,
            AssignmentOrigin::User,
            AssignmentRequester::User("user-1".to_string()),
            agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();

    let child_agent = AgentId::new("agent-2");
    let child_task = make_task(&store, &workspace_id, &child_agent, "Child").await;
    let child = service
        .delegate(
            &root.id,
            &child_task,
            AssignmentRequester::User("user-1".to_string()),
            child_agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();

    let grandchild_task = make_task(&store, &workspace_id, &agent, "Grandchild").await;
    let grandchild = service
        .delegate(
            &child.id,
            &grandchild_task,
            AssignmentRequester::User("user-2".to_string()),
            agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();

    service.cancel(&root.id, "user cancelled").await.unwrap();

    for id in [&root.id, &child.id, &grandchild.id] {
        let assignment = service.get(id).await.unwrap().unwrap();
        assert_eq!(assignment.status, AssignmentStatus::Cancelled);
    }
    for task in [&root_task, &child_task, &grandchild_task] {
        let stored = store.get_task(&task.id).await.unwrap().unwrap();
        assert_eq!(stored.status, TaskStatus::Cancelled);
    }
    let child_events = service.events(&child.id).await.unwrap();
    assert!(child_events
        .iter()
        .any(|event| event.kind == AssignmentEventKind::Cancelled));
}

#[tokio::test]
async fn restart_reconciliation_reruns_once_then_fails() {
    let (store, service, workspace_id, agent) = setup().await;
    let task = make_task(&store, &workspace_id, &agent, "Long task").await;
    let assignment = service
        .enqueue_root(
            &task,
            AssignmentOrigin::User,
            AssignmentRequester::User("user-1".to_string()),
            agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    service.start(&assignment.id).await.unwrap();

    service.reconcile_on_startup().await.unwrap();
    let reconciled = service.get(&assignment.id).await.unwrap().unwrap();
    assert_eq!(reconciled.status, AssignmentStatus::Queued);
    assert_eq!(reconciled.restarts, 1);

    service.start(&assignment.id).await.unwrap();
    service.reconcile_on_startup().await.unwrap();
    let failed = service.get(&assignment.id).await.unwrap().unwrap();
    assert_eq!(failed.status, AssignmentStatus::Failed);
    match failed.outcome {
        Some(AssignmentOutcome::Failed { error }) => {
            assert!(error.contains("interrupted"))
        }
        other => panic!("expected failed outcome, got {other:?}"),
    }
}

#[tokio::test]
async fn restart_reconciliation_waits_for_started_children() {
    let (store, service, workspace_id, agent) = setup().await;
    let root_task = make_task(&store, &workspace_id, &agent, "Root").await;
    let root = service
        .enqueue_root(
            &root_task,
            AssignmentOrigin::User,
            AssignmentRequester::User("user-1".to_string()),
            agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    service.start(&root.id).await.unwrap();

    let child_agent = AgentId::new("agent-2");
    let child_task = make_task(&store, &workspace_id, &child_agent, "Child").await;
    let child = service
        .delegate(
            &root.id,
            &child_task,
            AssignmentRequester::User("user-1".to_string()),
            child_agent,
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    service.start(&child.id).await.unwrap();

    service.reconcile_on_startup().await.unwrap();
    let reconciled = service.get(&root.id).await.unwrap().unwrap();
    assert_eq!(reconciled.status, AssignmentStatus::Waiting);
}

#[tokio::test]
async fn stall_guard_fails_after_repeated_no_progress() {
    let (store, service, workspace_id, agent) = setup().await;
    let task = make_task(&store, &workspace_id, &agent, "Stalling task").await;
    let assignment = service
        .enqueue_root(
            &task,
            AssignmentOrigin::User,
            AssignmentRequester::User("user-1".to_string()),
            agent.clone(),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    service.start(&assignment.id).await.unwrap();

    let first = service
        .record_progress(&assignment.id, 0, Some("tool failed"))
        .await
        .unwrap();
    assert_eq!(first.status, AssignmentStatus::Running);
    assert_eq!(first.stall_strikes, 1);

    let second = service
        .record_progress(&assignment.id, 0, Some("tool failed"))
        .await
        .unwrap();
    assert_eq!(second.status, AssignmentStatus::Failed);
    let events = service.events(&assignment.id).await.unwrap();
    assert!(events
        .iter()
        .any(|event| event.kind == AssignmentEventKind::Stalled));
}

#[tokio::test]
async fn execution_budget_failure_is_persisted() {
    let (store, service, workspace_id, agent) = setup().await;
    let task = make_task(&store, &workspace_id, &agent, "Budgeted task").await;
    let mut assignment = Assignment::new_root(
        task.id.clone(),
        AssignmentOrigin::User,
        AssignmentRequester::User("user-1".to_string()),
        agent,
        AssignmentMode::Work,
        ReplyMode::Resume,
        ApprovalLevel::Ask,
    );
    assignment.budgets.executions = 1;
    store.create_assignment(&assignment).await.unwrap();

    service.start(&assignment.id).await.unwrap();
    service.mark_waiting(&assignment.id).await.unwrap();
    service.resume(&assignment.id).await.unwrap();
    let result = service.start(&assignment.id).await;
    assert!(result.is_err());

    let failed = service.get(&assignment.id).await.unwrap().unwrap();
    assert_eq!(failed.status, AssignmentStatus::Failed);
    assert_eq!(
        store.get_task(&task.id).await.unwrap().unwrap().status,
        TaskStatus::Failed {
            error: "execution budget exhausted".to_string()
        }
    );
    let events = service.events(&assignment.id).await.unwrap();
    assert!(events
        .iter()
        .any(|event| event.kind == AssignmentEventKind::BudgetExhausted));
}

#[test]
fn delegation_inherits_approval_level() {
    use cowbot_core::assignment::inherit_approval_level;

    assert_eq!(
        inherit_approval_level(ApprovalLevel::Trusted, ApprovalLevel::Ask),
        ApprovalLevel::Trusted
    );
    assert_eq!(
        inherit_approval_level(ApprovalLevel::Ask, ApprovalLevel::SafeAuto),
        ApprovalLevel::SafeAuto
    );
    assert_eq!(
        inherit_approval_level(ApprovalLevel::PlanOnly, ApprovalLevel::Trusted),
        ApprovalLevel::PlanOnly
    );
    assert_eq!(
        inherit_approval_level(ApprovalLevel::SafeAuto, ApprovalLevel::Ask),
        ApprovalLevel::SafeAuto
    );
}
