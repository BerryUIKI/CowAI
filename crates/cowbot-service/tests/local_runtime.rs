use cowbot_bus::EventBus;
use cowbot_core::agent::AgentId;
use cowbot_core::channel::Sender;
use cowbot_core::event::EventPayload;
use cowbot_core::identity::Role;
use cowbot_service::{bootstrap_local, ChannelService};
use cowbot_store::Store;

async fn test_store() -> Store {
    let store = Store::connect_in_memory().await.unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn local_bootstrap_is_idempotent() {
    let store = test_store().await;

    let first = bootstrap_local(&store).await.unwrap();
    let second = bootstrap_local(&store).await.unwrap();

    assert_eq!(first.user.id, second.user.id);
    assert_eq!(first.workspace.id, second.workspace.id);
    assert_eq!(store.list_users().await.unwrap().len(), 1);
    assert_eq!(store.list_workspaces().await.unwrap().len(), 1);

    let membership = store
        .get_membership(&first.user.id, &first.workspace.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(membership.role, Role::Owner);
}

#[tokio::test]
async fn channels_and_messages_persist_and_publish_events() {
    let store = test_store().await;
    let bus = EventBus::default_bus();
    let mut receiver = bus.subscribe();
    let service = ChannelService::new(store.clone(), bus.clone());

    let context = bootstrap_local(&store).await.unwrap();
    let channel = service
        .create_channel(&context.workspace.id, "engineering", "Engineering work")
        .await
        .unwrap();

    match receiver.recv().await.unwrap().payload {
        EventPayload::ChannelCreated(created) => assert_eq!(created.id, channel.id),
        other => panic!("expected ChannelCreated, got {other:?}"),
    }

    let question = service
        .post_message(
            &channel.id,
            Sender::User(context.user.id.0.clone()),
            "Implement the login flow",
        )
        .await
        .unwrap();

    match receiver.recv().await.unwrap().payload {
        EventPayload::MessagePosted(message) => assert_eq!(message.id, question.id),
        other => panic!("expected MessagePosted, got {other:?}"),
    }

    let reply = service
        .post_thread_message(
            &channel.id,
            Sender::Agent(AgentId::new("agent-coder")),
            "Working on it",
            Some(question.id.clone()),
        )
        .await
        .unwrap();

    match receiver.recv().await.unwrap().payload {
        EventPayload::MessagePosted(message) => assert_eq!(message.id, reply.id),
        other => panic!("expected MessagePosted, got {other:?}"),
    }

    let messages = service.list_messages(&channel.id, 10).await.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Implement the login flow");
    assert_eq!(messages[1].thread_id, Some(question.id.clone()));
    assert!(matches!(messages[0].sender, Sender::User(_)));
    assert!(matches!(messages[1].sender, Sender::Agent(_)));

    let channels = service.list_channels(&context.workspace.id).await.unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].name, "engineering");
}
