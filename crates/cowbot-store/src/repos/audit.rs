use cowbot_core::audit::AuditEvent;
use cowbot_core::error::CowResult;
use cowbot_core::identity::WorkspaceId;

use crate::error::{db, parse_datetime};
use crate::store::Store;

#[derive(sqlx::FromRow)]
struct AuditEventRow {
    id: String,
    workspace_id: String,
    actor: String,
    action: String,
    target: String,
    created_at: String,
}

impl AuditEventRow {
    fn into_domain(self) -> CowResult<AuditEvent> {
        Ok(AuditEvent {
            id: self.id,
            workspace_id: WorkspaceId(self.workspace_id),
            actor: self.actor,
            action: self.action,
            target: self.target,
            created_at: parse_datetime(&self.created_at)?,
        })
    }
}

impl Store {
    pub async fn append_audit_event(&self, event: &AuditEvent) -> CowResult<()> {
        sqlx::query(
            "INSERT INTO audit_events (id, workspace_id, actor, action, target, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(&event.workspace_id.0)
        .bind(&event.actor)
        .bind(&event.action)
        .bind(&event.target)
        .bind(event.created_at.to_rfc3339())
        .execute(self.pool())
        .await
        .map_err(db)?;
        Ok(())
    }

    pub async fn list_audit_events(
        &self,
        workspace_id: &WorkspaceId,
        limit: u32,
    ) -> CowResult<Vec<AuditEvent>> {
        let rows = sqlx::query_as::<_, AuditEventRow>(
            "SELECT id, workspace_id, actor, action, target, created_at
             FROM audit_events WHERE workspace_id = ?
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(&workspace_id.0)
        .bind(i64::from(limit))
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        rows.into_iter().map(AuditEventRow::into_domain).collect()
    }
}
