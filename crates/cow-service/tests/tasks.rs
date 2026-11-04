use cow_bus::EventBus;
use cow_core::agent::AgentId;
use cow_core::approval::ApprovalLevel;
use cow_core::assignment::{AssignmentRequester, AssignmentStatus};
use cow_core::event::EventPayload;
use cow_core::task::{Task, TaskStatus};
use cow_service::{bootstrap_local, TaskService};
use cow_store::Store;

#[tokio::test]
async fn task_service_creates_lists_and_cancels_tasks() {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    let context = bootstrap_local(&store).await.unwrap();
    let bus = EventBus::default_bus();
    let mut events = bus.subscribe();
    let tasks = TaskService::new(store.clone(), bus);

    let agent = AgentId::new("agent-coder");
    let task = Task::new("Implement login", "Working code").assign_to(agent);
    let assignment = tasks
        .create(
            &context.workspace.id,
            None,
            &task,
            AssignmentRequester::User(context.user.id.0.clone()),
            ApprovalLevel::Ask,
        )
        .await
        .unwrap();
    assert_eq!(assignment.status, AssignmentStatus::Queued);

    match events.recv().await.unwrap().payload {
        EventPayload::TaskCreated(created) => assert_eq!(created.id, task.id),
        other => panic!("expected TaskCreated, got {other:?}"),
    }

    let listed = tasks.list(&context.workspace.id).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, task.id);
    assert_eq!(listed[0].status, TaskStatus::Pending);

    tasks.cancel(&task.id, "no longer needed").await.unwrap();

    let stored = tasks.get(&task.id).await.unwrap().unwrap();
    assert_eq!(stored.status, TaskStatus::Cancelled);
    let stored_assignment = store.get_assignment(&assignment.id).await.unwrap().unwrap();
    assert_eq!(stored_assignment.status, AssignmentStatus::Cancelled);
}
