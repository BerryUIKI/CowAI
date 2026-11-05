use cowbot_core::agent::AgentProfile;
use cowbot_core::channel::{Channel, ChannelMessage, Sender};
use cowbot_core::identity::{Membership, Role, User, Workspace};
use cowbot_core::task::{Task, TaskStatus};
use cowbot_package::{
    export_workspace, import_workspace, inspect, PackageError, PackageKind, PACKAGE_FORMAT,
};
use cowbot_store::Store;

async fn test_store() -> Store {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed(store: &Store) -> Workspace {
    let user = User::new("Alice", "alice@example.com", "argon2-hash");
    let workspace = Workspace::new("Research Lab");
    store.create_user(&user).await.unwrap();
    store.create_workspace(&workspace).await.unwrap();
    store
        .add_member(&Membership::new(
            user.id.clone(),
            workspace.id.clone(),
            Role::Owner,
        ))
        .await
        .unwrap();

    let channel = Channel::new("general", "Team chatter");
    store.create_channel(&workspace.id, &channel).await.unwrap();
    store
        .append_message(&ChannelMessage::new(
            channel.id.clone(),
            Sender::User(user.id.0.clone()),
            "hello from the export test",
        ))
        .await
        .unwrap();

    let agent = AgentProfile::new("Cow-Coder", "Engineer", "Ship code", "Terse.");
    store.upsert_agent(&workspace.id, &agent).await.unwrap();

    let task = Task::new("Do the thing", "A working thing");
    store.create_task(&workspace.id, None, &task).await.unwrap();

    workspace
}

#[tokio::test]
async fn export_then_import_round_trip() {
    let source = test_store().await;
    let workspace = seed(&source).await;

    let bytes = export_workspace(&source, &workspace.id).await.unwrap();
    assert!(!bytes.is_empty());

    let manifest = inspect(&bytes).unwrap();
    assert_eq!(manifest.format, PACKAGE_FORMAT);
    assert_eq!(manifest.kind, PackageKind::Workspace);
    assert_eq!(manifest.exported_by.workspace_name, "Research Lab");
    assert!(!manifest.checksums.is_empty());

    let target = test_store().await;
    let summary = import_workspace(&target, &bytes).await.unwrap();
    assert_eq!(summary.channels, 1);
    assert_eq!(summary.messages, 1);
    assert_eq!(summary.agents, 1);
    assert_eq!(summary.tasks, 1);
    assert!(summary.workspace_name.ends_with("(imported)"));

    let channels = target.list_channels(&summary.workspace_id).await.unwrap();
    assert_eq!(channels.len(), 1);
    let messages = target.list_messages(&channels[0].id, 10).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "hello from the export test");

    let agents = target.list_agents(&summary.workspace_id).await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "Cow-Coder");

    let tasks = target.list_tasks(&summary.workspace_id).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(matches!(tasks[0].status, TaskStatus::Pending));
}

#[tokio::test]
async fn import_creates_a_new_workspace_and_leaves_existing_ones_alone() {
    let source = test_store().await;
    let workspace = seed(&source).await;
    let bytes = export_workspace(&source, &workspace.id).await.unwrap();

    let target = test_store().await;
    seed(&target).await;
    let before = target.list_workspaces().await.unwrap().len();

    let summary = import_workspace(&target, &bytes).await.unwrap();
    let after = target.list_workspaces().await.unwrap();

    assert_eq!(after.len(), before + 1);
    assert_ne!(summary.workspace_id, workspace.id);
    assert!(after.iter().any(|ws| ws.id == summary.workspace_id));
}

#[tokio::test]
async fn tampered_payload_is_rejected() {
    let source = test_store().await;
    let workspace = seed(&source).await;
    let mut bytes = export_workspace(&source, &workspace.id).await.unwrap();

    // Flip a byte in the compressed body; the checksum guard must catch it.
    let midpoint = bytes.len() / 2;
    bytes[midpoint] ^= 0xff;

    let target = test_store().await;
    let result = import_workspace(&target, &bytes).await;
    assert!(result.is_err(), "tampered package must not import");
    assert!(target.list_workspaces().await.unwrap().is_empty());
}

#[tokio::test]
async fn non_zip_input_is_rejected() {
    let target = test_store().await;
    let result = import_workspace(&target, b"not a zip archive").await;
    assert!(matches!(result, Err(PackageError::Zip(_))));
}

#[tokio::test]
async fn manifest_checksums_cover_every_content_file() {
    let source = test_store().await;
    let workspace = seed(&source).await;
    let bytes = export_workspace(&source, &workspace.id).await.unwrap();
    let manifest = inspect(&bytes).unwrap();

    for path in [
        "workspace.json",
        "agents.json",
        "templates.json",
        "channels.jsonl",
        "messages.jsonl",
        "tasks.json",
        "runs.jsonl",
        "audit.jsonl",
    ] {
        assert!(
            manifest.checksums.contains_key(path),
            "manifest is missing a checksum for {path}"
        );
    }
}
