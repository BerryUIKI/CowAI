use cow_client::{Client, LocalClient};
use cow_core::channel::Sender;
use cow_core::event::EventPayload;

#[tokio::test]
async fn local_client_bootstraps_a_session() {
    let client = LocalClient::open_in_memory().await.unwrap();

    let session = client.session().await.unwrap();
    assert_eq!(session.user.name, "Local Owner");
    assert_eq!(session.user.email, "local@cowai.local");
    assert_eq!(session.workspace.name, "Default Workspace");
    assert!(client.list_channels().await.unwrap().is_empty());
}

#[tokio::test]
async fn local_client_manages_channels_and_messages() {
    let client = LocalClient::open_in_memory().await.unwrap();
    let session = client.session().await.unwrap();

    let channel = client
        .create_channel("engineering", "Engineering work")
        .await
        .unwrap();

    let channels = client.list_channels().await.unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].id, channel.id);

    let question = client
        .post_message(&channel.id, "Implement the login flow")
        .await
        .unwrap();
    match &question.sender {
        Sender::User(user_id) => assert_eq!(user_id, &session.user.id.0),
        other => panic!("expected the local user sender, got {other:?}"),
    }

    let reply = client
        .post_thread_message(&channel.id, "Working on it", &question.id)
        .await
        .unwrap();
    assert_eq!(reply.thread_id.as_deref(), Some(question.id.as_str()));

    let messages = client.list_messages(&channel.id, 10).await.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Implement the login flow");
    assert_eq!(messages[1].thread_id.as_deref(), Some(question.id.as_str()));
}

#[tokio::test]
async fn local_client_streams_events() {
    let client = LocalClient::open_in_memory().await.unwrap();
    let mut events = client.subscribe();

    let channel = client
        .create_channel("alerts", "Deployment alerts")
        .await
        .unwrap();
    match events.recv().await.unwrap().payload {
        EventPayload::ChannelCreated(created) => assert_eq!(created.id, channel.id),
        other => panic!("expected ChannelCreated, got {other:?}"),
    }

    let message = client.post_message(&channel.id, "hello").await.unwrap();
    match events.recv().await.unwrap().payload {
        EventPayload::MessagePosted(posted) => assert_eq!(posted.id, message.id),
        other => panic!("expected MessagePosted, got {other:?}"),
    }
}
