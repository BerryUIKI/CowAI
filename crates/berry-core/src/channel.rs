use crate::agent::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a collaboration channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message sender identity (Agent, Human user, or System).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum Sender {
    Agent(AgentId),
    User(String),
    System,
}

/// A collaboration channel where agents and users interact (inspired by Mattermost).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    pub topic: String,
    pub created_at: DateTime<Utc>,
}

impl Channel {
    pub fn new(name: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            id: ChannelId::random(),
            name: name.into(),
            topic: topic.into(),
            created_at: Utc::now(),
        }
    }
}

/// A structured message posted to a channel or thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: String,
    pub channel_id: ChannelId,
    pub sender: Sender,
    pub content: String,
    pub thread_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ChannelMessage {
    pub fn new(channel_id: ChannelId, sender: Sender, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            channel_id,
            sender,
            content: content.into(),
            thread_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_thread(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }
}
