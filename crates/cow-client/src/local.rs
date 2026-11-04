use std::path::Path;

use async_trait::async_trait;
use cow_bus::EventBus;
use cow_core::agent::AgentProfile;
use cow_core::channel::{Channel, ChannelId, ChannelMessage, Sender};
use cow_core::error::{CowError, CowResult};
use cow_core::template::TemplateDocument;
use cow_service::{
    bootstrap_local, ApprovalService, AssignmentService, ChannelService, TemplateService,
};
use cow_store::Store;
use tokio::sync::{broadcast, mpsc};
use tracing::warn;

use crate::{Client, ClientSession, ClientUser, EventStream};

/// In-process client backed by `cow-store` and `cow-service`.
pub struct LocalClient {
    store: Store,
    bus: EventBus,
    session: ClientSession,
    channels: ChannelService,
    templates: TemplateService,
}

impl LocalClient {
    /// Open (or create) the local SQLite workspace at `path`.
    pub async fn open(path: impl AsRef<Path>) -> CowResult<Self> {
        let store = Store::connect(path).await?;
        Self::initialize(store, EventBus::default_bus()).await
    }

    /// Open an in-memory workspace, primarily for tests and embedded use.
    pub async fn open_in_memory() -> CowResult<Self> {
        let store = Store::connect_in_memory().await?;
        Self::initialize(store, EventBus::default_bus()).await
    }

    async fn initialize(store: Store, bus: EventBus) -> CowResult<Self> {
        store.migrate().await?;
        let context = bootstrap_local(&store).await?;

        AssignmentService::new(store.clone(), bus.clone())
            .reconcile_on_startup()
            .await?;
        ApprovalService::new(store.clone(), bus.clone(), context.workspace.id.clone())
            .reconcile_on_startup()
            .await?;

        let channels = ChannelService::new(store.clone(), bus.clone());
        let templates = TemplateService::new(store.clone());
        let session = ClientSession {
            user: ClientUser {
                id: context.user.id,
                name: context.user.name,
                email: context.user.email,
            },
            workspace: context.workspace,
        };

        Ok(Self {
            store,
            bus,
            session,
            channels,
            templates,
        })
    }

    fn user_sender(&self) -> Sender {
        Sender::User(self.session.user.id.0.clone())
    }
}

#[async_trait]
impl Client for LocalClient {
    async fn session(&self) -> CowResult<ClientSession> {
        Ok(self.session.clone())
    }

    fn subscribe(&self) -> EventStream {
        let mut broadcast_rx = self.bus.subscribe();
        let (tx, rx) = mpsc::channel(256);

        tokio::spawn(async move {
            loop {
                match broadcast_rx.recv().await {
                    Ok(event) => {
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "client event stream lagged; events were skipped");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        EventStream::new(rx)
    }

    async fn list_channels(&self) -> CowResult<Vec<Channel>> {
        self.channels
            .list_channels(&self.session.workspace.id)
            .await
    }

    async fn create_channel(&self, name: &str, topic: &str) -> CowResult<Channel> {
        self.channels
            .create_channel(&self.session.workspace.id, name, topic)
            .await
    }

    async fn post_message(
        &self,
        channel_id: &ChannelId,
        content: &str,
    ) -> CowResult<ChannelMessage> {
        self.channels
            .post_message(channel_id, self.user_sender(), content)
            .await
    }

    async fn post_thread_message(
        &self,
        channel_id: &ChannelId,
        content: &str,
        thread_id: &str,
    ) -> CowResult<ChannelMessage> {
        self.channels
            .post_thread_message(
                channel_id,
                self.user_sender(),
                content,
                Some(thread_id.to_string()),
            )
            .await
    }

    async fn list_messages(
        &self,
        channel_id: &ChannelId,
        limit: u32,
    ) -> CowResult<Vec<ChannelMessage>> {
        self.channels.list_messages(channel_id, limit).await
    }

    async fn list_templates(&self) -> CowResult<Vec<TemplateDocument>> {
        self.templates.list().await
    }

    async fn get_template(&self, id: &str) -> CowResult<Option<TemplateDocument>> {
        self.templates.get(id).await
    }

    async fn save_template(&self, document: &TemplateDocument) -> CowResult<()> {
        self.templates.save(document).await
    }

    async fn delete_template(&self, id: &str) -> CowResult<()> {
        self.templates.delete(id).await
    }

    async fn import_template(&self, json: &str) -> CowResult<TemplateDocument> {
        self.templates.import_json(json).await
    }

    async fn export_template(&self, id: &str) -> CowResult<String> {
        let document = self
            .templates
            .get(id)
            .await?
            .ok_or_else(|| CowError::Config(format!("template '{id}' was not found")))?;
        TemplateService::export_json(&document)
    }

    async fn instantiate_agents(&self, template_id: &str) -> CowResult<Vec<AgentProfile>> {
        self.templates
            .instantiate_agents(&self.session.workspace.id, template_id)
            .await
    }

    async fn list_agents(&self) -> CowResult<Vec<AgentProfile>> {
        self.store.list_agents(&self.session.workspace.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cow_core::approval::{Approval, ApprovalStatus};
    use cow_core::task::Task;

    #[tokio::test]
    async fn initialize_expires_orphaned_approvals() {
        let store = Store::connect_in_memory().await.unwrap();
        store.migrate().await.unwrap();
        let context = bootstrap_local(&store).await.unwrap();
        let task = Task::new("Pending approval", "Output");
        store
            .create_task(&context.workspace.id, None, &task)
            .await
            .unwrap();
        let approval = Approval::new(task.id.clone(), "call-1");
        store.create_approval(&approval).await.unwrap();

        let client = LocalClient::initialize(store.clone(), EventBus::default_bus())
            .await
            .unwrap();

        assert!(store
            .list_pending_approvals(&task.id)
            .await
            .unwrap()
            .is_empty());
        let stored = store.get_approval(&approval.id).await.unwrap().unwrap();
        assert_eq!(stored.status, ApprovalStatus::Expired);
        assert_eq!(client.session().await.unwrap().user.name, "Local Owner");
    }
}
