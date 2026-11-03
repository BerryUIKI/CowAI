use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Kind of reusable template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateKind {
    Agent,
    Team,
}

/// A stored agent or team template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub kind: TemplateKind,
    pub name: String,
    pub description: String,
    pub version: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

impl Template {
    pub fn new(kind: TemplateKind, name: impl Into<String>, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            name: name.into(),
            description: String::new(),
            version: "1.0.0".to_string(),
            payload,
            created_at: Utc::now(),
        }
    }
}
