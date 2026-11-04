//! Transport-neutral client boundary for CowAI.
//!
//! Local mode calls the core in-process. A future remote implementation will
//! speak to `cow-server` over HTTP and WebSocket behind the same [`Client`]
//! trait, so the desktop UI and CLI never depend on storage or service types
//! directly (ADR-0001, ADR-0007).

use async_trait::async_trait;
use cow_core::agent::{AgentId, AgentProfile};
use cow_core::approval::{Approval, ApprovalId, ApprovalLevel};
use cow_core::assignment::Assignment;
use cow_core::channel::{Channel, ChannelId, ChannelMessage};
use cow_core::error::CowResult;
use cow_core::event::Event;
use cow_core::identity::{UserId, Workspace};
use cow_core::task::{Task, TaskId};
use cow_core::template::TemplateDocument;
use tokio::sync::mpsc;

mod local;

pub use local::LocalClient;

/// User identity as seen by the client.
///
/// Credentials never cross this boundary; a remote implementation receives a
/// session token instead of a password hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientUser {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

/// The authenticated user and workspace a client operates in.
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub user: ClientUser,
    pub workspace: Workspace,
}

/// Parameters for creating a task and its root assignment.
#[derive(Debug, Clone)]
pub struct NewTask {
    pub description: String,
    pub expected_output: String,
    pub assignee: AgentId,
    pub channel_id: Option<ChannelId>,
    pub approval_level: ApprovalLevel,
}

impl NewTask {
    pub fn new(
        description: impl Into<String>,
        expected_output: impl Into<String>,
        assignee: AgentId,
    ) -> Self {
        Self {
            description: description.into(),
            expected_output: expected_output.into(),
            assignee,
            channel_id: None,
            approval_level: ApprovalLevel::Ask,
        }
    }

    pub fn in_channel(mut self, channel_id: ChannelId) -> Self {
        self.channel_id = Some(channel_id);
        self
    }

    pub fn with_approval_level(mut self, approval_level: ApprovalLevel) -> Self {
        self.approval_level = approval_level;
        self
    }
}

/// A live stream of platform events for the UI.
pub struct EventStream {
    receiver: mpsc::Receiver<Event>,
}

impl EventStream {
    pub(crate) fn new(receiver: mpsc::Receiver<Event>) -> Self {
        Self { receiver }
    }

    /// Wait for the next event. Returns `None` once the stream is closed.
    pub async fn recv(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}

/// Unified client interface for local and future remote modes.
#[async_trait]
pub trait Client: Send + Sync {
    /// The local user and workspace this client is bound to.
    async fn session(&self) -> CowResult<ClientSession>;

    /// Subscribe to live platform events.
    fn subscribe(&self) -> EventStream;

    async fn list_channels(&self) -> CowResult<Vec<Channel>>;
    async fn create_channel(&self, name: &str, topic: &str) -> CowResult<Channel>;
    async fn post_message(
        &self,
        channel_id: &ChannelId,
        content: &str,
    ) -> CowResult<ChannelMessage>;
    async fn post_thread_message(
        &self,
        channel_id: &ChannelId,
        content: &str,
        thread_id: &str,
    ) -> CowResult<ChannelMessage>;
    async fn list_messages(
        &self,
        channel_id: &ChannelId,
        limit: u32,
    ) -> CowResult<Vec<ChannelMessage>>;

    async fn create_task(&self, task: &NewTask) -> CowResult<Task>;
    async fn list_tasks(&self) -> CowResult<Vec<Task>>;
    async fn get_task(&self, task_id: &TaskId) -> CowResult<Option<Task>>;
    async fn cancel_task(&self, task_id: &TaskId, reason: &str) -> CowResult<()>;
    async fn list_assignments(&self, task_id: &TaskId) -> CowResult<Vec<Assignment>>;

    async fn pending_approvals(&self) -> CowResult<Vec<Approval>>;
    async fn decide_approval(
        &self,
        approval_id: &ApprovalId,
        approved: bool,
    ) -> CowResult<Approval>;

    async fn list_templates(&self) -> CowResult<Vec<TemplateDocument>>;
    async fn get_template(&self, id: &str) -> CowResult<Option<TemplateDocument>>;
    async fn save_template(&self, document: &TemplateDocument) -> CowResult<()>;
    async fn delete_template(&self, id: &str) -> CowResult<()>;
    async fn import_template(&self, json: &str) -> CowResult<TemplateDocument>;
    async fn export_template(&self, id: &str) -> CowResult<String>;
    async fn instantiate_agents(&self, template_id: &str) -> CowResult<Vec<AgentProfile>>;
    async fn list_agents(&self) -> CowResult<Vec<AgentProfile>>;
}
