use std::time::Duration;

use async_trait::async_trait;
use cow_bus::EventBus;
use cow_core::agent::AgentId;
use cow_core::approval::{Approval, ApprovalId, ApprovalLevel, ApprovalStatus};
use cow_core::assignment::AssignmentId;
use cow_core::audit::AuditEvent;
use cow_core::error::{CowError, CowResult};
use cow_core::event::{Event, EventPayload};
use cow_core::identity::WorkspaceId;
use cow_core::risk::RiskLevel;
use cow_core::task::TaskId;
use cow_core::tool::{ToolAuthorization, ToolCall, ToolGate};
use cow_store::Store;
use cow_tools::{RiskAssessment, StaticRiskClassifier};
use tokio::time::{sleep, Instant};

/// Policy outcome before a tool call executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    AutoApprove,
    RequireApproval,
    Deny,
}

/// Apply the workspace/assignment approval level to a risk level.
pub fn decision_for(level: ApprovalLevel, risk: RiskLevel) -> ApprovalDecision {
    match level {
        ApprovalLevel::Ask => match risk {
            RiskLevel::ReadOnly => ApprovalDecision::AutoApprove,
            RiskLevel::Low | RiskLevel::Medium | RiskLevel::High => {
                ApprovalDecision::RequireApproval
            }
            RiskLevel::Critical => ApprovalDecision::Deny,
        },
        ApprovalLevel::SafeAuto => match risk {
            RiskLevel::ReadOnly | RiskLevel::Low => ApprovalDecision::AutoApprove,
            RiskLevel::Medium | RiskLevel::High => ApprovalDecision::RequireApproval,
            RiskLevel::Critical => ApprovalDecision::Deny,
        },
        ApprovalLevel::PlanOnly => match risk {
            RiskLevel::ReadOnly => ApprovalDecision::AutoApprove,
            _ => ApprovalDecision::Deny,
        },
        ApprovalLevel::Trusted => match risk {
            RiskLevel::Critical => ApprovalDecision::Deny,
            _ => ApprovalDecision::AutoApprove,
        },
    }
}

/// Approval service used by the local runtime and the agent tool gate.
#[derive(Clone)]
pub struct ApprovalService {
    store: Store,
    bus: EventBus,
    workspace_id: WorkspaceId,
    timeout: Duration,
}

impl ApprovalService {
    pub fn new(store: Store, bus: EventBus, workspace_id: WorkspaceId) -> Self {
        Self {
            store,
            bus,
            workspace_id,
            timeout: Duration::from_secs(300),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn authorize(
        &self,
        assignment_id: &AssignmentId,
        agent_id: &AgentId,
        call: &ToolCall,
    ) -> CowResult<ToolAuthorization> {
        let assignment = self
            .store
            .get_assignment(assignment_id)
            .await?
            .ok_or_else(|| CowError::Task(format!("assignment '{assignment_id}' was not found")))?;
        let assessment = StaticRiskClassifier.classify(call);

        match decision_for(assignment.approval_level, assessment.level) {
            ApprovalDecision::AutoApprove => {
                self.record_audit(&agent_id.0, "tool.auto_approved", call, &assessment)
                    .await?;
                Ok(ToolAuthorization::Approved { approved_by: None })
            }
            ApprovalDecision::Deny => {
                self.record_audit(&agent_id.0, "tool.denied", call, &assessment)
                    .await?;
                Ok(ToolAuthorization::Denied {
                    reason: format!(
                        "approval policy denied {} risk operation: {}",
                        assessment.level.as_str(),
                        assessment.reason
                    ),
                })
            }
            ApprovalDecision::RequireApproval => {
                let approval = Approval::new(assignment.task_id.clone(), call.id.clone());
                self.store.create_approval(&approval).await?;
                let _ = self
                    .bus
                    .publish(Event::new(EventPayload::ApprovalRequested {
                        approval: approval.clone(),
                        risk: assessment.level,
                        reason: assessment.reason.clone(),
                    }));
                self.record_audit(&agent_id.0, "tool.approval_requested", call, &assessment)
                    .await?;

                let decided = self.wait_for_decision(&approval.id).await?;
                let decided = if decided.status == ApprovalStatus::Pending {
                    self.expire_or_load_decision(&approval.id).await?
                } else {
                    decided
                };
                match decided.status {
                    ApprovalStatus::Approved => {
                        let approved_by = decided.decided_by.clone();
                        self.record_audit(
                            approved_by.as_deref().unwrap_or("unknown"),
                            "tool.approval_approved",
                            call,
                            &assessment,
                        )
                        .await?;
                        Ok(ToolAuthorization::Approved { approved_by })
                    }
                    ApprovalStatus::Rejected => {
                        let decided_by =
                            decided.decided_by.unwrap_or_else(|| "unknown".to_string());
                        self.record_audit(&decided_by, "tool.approval_rejected", call, &assessment)
                            .await?;
                        Ok(ToolAuthorization::Denied {
                            reason: format!("approval rejected by {decided_by}"),
                        })
                    }
                    ApprovalStatus::Expired => {
                        self.record_audit("system", "tool.approval_expired", call, &assessment)
                            .await?;
                        Ok(ToolAuthorization::Denied {
                            reason: "approval expired without a decision".to_string(),
                        })
                    }
                    ApprovalStatus::Pending => Err(CowError::Task(format!(
                        "approval '{}' remained pending after its timeout",
                        approval.id
                    ))),
                }
            }
        }
    }

    pub async fn decide(
        &self,
        approval_id: &ApprovalId,
        status: ApprovalStatus,
        decided_by: &str,
    ) -> CowResult<Approval> {
        if status != ApprovalStatus::Approved && status != ApprovalStatus::Rejected {
            return Err(CowError::Config(
                "an approval can only be approved or rejected".to_string(),
            ));
        }
        self.store
            .decide_approval(approval_id, status, decided_by)
            .await?;
        let approval = self
            .store
            .get_approval(approval_id)
            .await?
            .ok_or_else(|| CowError::Task(format!("approval '{approval_id}' was not found")))?;
        let _ = self.bus.publish(Event::new(EventPayload::ApprovalDecided {
            approval: approval.clone(),
        }));
        let action = if status == ApprovalStatus::Approved {
            "tool.approval_approved"
        } else {
            "tool.approval_rejected"
        };
        self.store
            .append_audit_event(&AuditEvent::new(
                self.workspace_id.clone(),
                decided_by,
                action,
                format!(
                    "approval {} for tool call {}",
                    approval.id, approval.tool_call_id
                ),
            ))
            .await?;
        Ok(approval)
    }

    pub async fn pending_for_task(&self, task_id: &TaskId) -> CowResult<Vec<Approval>> {
        self.store.list_pending_approvals(task_id).await
    }

    pub async fn pending_for_workspace(&self) -> CowResult<Vec<Approval>> {
        self.store
            .list_pending_approvals_for_workspace(&self.workspace_id)
            .await
    }

    /// Expire approvals left pending by a previous process.
    ///
    /// The local runtime should call this once during startup: a pending row
    /// whose waiting agent died would otherwise stay actionable forever. Only
    /// rows still pending are expired, so a decision that lands concurrently is
    /// preserved. Returns the number of approvals expired.
    pub async fn reconcile_on_startup(&self) -> CowResult<usize> {
        let pending = self
            .store
            .list_pending_approvals_for_workspace(&self.workspace_id)
            .await?;
        let mut expired = 0;
        for approval in pending {
            let resolved = self.expire_or_load_decision(&approval.id).await?;
            if resolved.status != ApprovalStatus::Expired {
                continue;
            }
            expired += 1;
            self.store
                .append_audit_event(&AuditEvent::new(
                    self.workspace_id.clone(),
                    "system",
                    "tool.approval_expired",
                    format!(
                        "approval {} for tool call {} expired on startup",
                        approval.id, approval.tool_call_id
                    ),
                ))
                .await?;
        }
        Ok(expired)
    }

    async fn wait_for_decision(&self, approval_id: &ApprovalId) -> CowResult<Approval> {
        let deadline = Instant::now() + self.timeout;
        loop {
            let approval =
                self.store.get_approval(approval_id).await?.ok_or_else(|| {
                    CowError::Task(format!("approval '{approval_id}' was not found"))
                })?;
            if approval.status != ApprovalStatus::Pending || Instant::now() >= deadline {
                return Ok(approval);
            }
            sleep(Duration::from_millis(200)).await;
        }
    }

    /// Expire a timed-out approval unless another decision landed first.
    ///
    /// `decide_approval` only updates pending rows, so a human decision that
    /// races the timeout makes the expiry update fail. In that case the stored
    /// decision is returned and honoured instead of failing the tool call.
    async fn expire_or_load_decision(&self, approval_id: &ApprovalId) -> CowResult<Approval> {
        match self
            .store
            .decide_approval(approval_id, ApprovalStatus::Expired, "system")
            .await
        {
            Ok(()) => {
                let expired = self.store.get_approval(approval_id).await?.ok_or_else(|| {
                    CowError::Task(format!("approval '{approval_id}' was not found"))
                })?;
                let _ = self.bus.publish(Event::new(EventPayload::ApprovalDecided {
                    approval: expired.clone(),
                }));
                Ok(expired)
            }
            Err(error) => match self.store.get_approval(approval_id).await? {
                Some(stored) if stored.status != ApprovalStatus::Pending => Ok(stored),
                _ => Err(error),
            },
        }
    }

    async fn record_audit(
        &self,
        actor: &str,
        action: &str,
        call: &ToolCall,
        assessment: &RiskAssessment,
    ) -> CowResult<()> {
        self.store
            .append_audit_event(&AuditEvent::new(
                self.workspace_id.clone(),
                actor,
                action,
                format!(
                    "{} ({}) risk={} rule={} arguments={}",
                    call.name,
                    call.id,
                    assessment.level.as_str(),
                    assessment.rule_id,
                    call.arguments
                ),
            ))
            .await
    }
}

/// A tool gate scoped to one assignment.
#[derive(Clone)]
pub struct ScopedApprovalGate {
    service: ApprovalService,
    assignment_id: AssignmentId,
}

impl ScopedApprovalGate {
    pub fn new(service: ApprovalService, assignment_id: AssignmentId) -> Self {
        Self {
            service,
            assignment_id,
        }
    }
}

#[async_trait]
impl ToolGate for ScopedApprovalGate {
    async fn authorize(&self, agent_id: &AgentId, call: &ToolCall) -> CowResult<ToolAuthorization> {
        self.service
            .authorize(&self.assignment_id, agent_id, call)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cow_core::task::Task;

    #[tokio::test]
    async fn expiry_keeps_a_decision_that_raced_the_timeout() {
        let store = Store::connect_in_memory().await.unwrap();
        store.migrate().await.unwrap();
        let context = crate::bootstrap_local(&store).await.unwrap();
        let task = Task::new("Race task", "Output");
        store
            .create_task(&context.workspace.id, None, &task)
            .await
            .unwrap();

        let service = ApprovalService::new(
            store.clone(),
            EventBus::default_bus(),
            context.workspace.id.clone(),
        );
        let approval = Approval::new(task.id.clone(), "call-1");
        store.create_approval(&approval).await.unwrap();
        service
            .decide(&approval.id, ApprovalStatus::Approved, "user-1")
            .await
            .unwrap();

        let resolved = service.expire_or_load_decision(&approval.id).await.unwrap();
        assert_eq!(resolved.status, ApprovalStatus::Approved);
        assert_eq!(resolved.decided_by.as_deref(), Some("user-1"));
    }
}
