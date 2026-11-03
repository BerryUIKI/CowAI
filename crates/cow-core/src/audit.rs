use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::identity::WorkspaceId;

/// An auditable action performed in a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub workspace_id: WorkspaceId,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub created_at: DateTime<Utc>,
}

impl AuditEvent {
    pub fn new(
        workspace_id: WorkspaceId,
        actor: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workspace_id,
            actor: actor.into(),
            action: action.into(),
            target: target.into(),
            created_at: Utc::now(),
        }
    }
}
