use std::collections::HashMap;
use std::sync::Arc;

use cow_agent::Crew;
use cow_bus::EventBus;
use cow_core::approval::ApprovalLevel;
use cow_core::assignment::{AssignmentOrigin, AssignmentRequester};
use cow_core::error::{CowError, CowResult};
use cow_core::identity::WorkspaceId;
use cow_core::task::TaskId;
use cow_store::Store;

use crate::approvals::{ApprovalService, ScopedApprovalGate};
use crate::assignments::AssignmentService;

/// Executes a sequential crew through the assignment runtime.
#[derive(Clone)]
pub struct CrewRunner {
    assignments: AssignmentService,
}

impl CrewRunner {
    pub fn new(store: Store, bus: EventBus) -> Self {
        Self {
            assignments: AssignmentService::new(store, bus),
        }
    }

    pub fn assignments(&self) -> &AssignmentService {
        &self.assignments
    }

    /// Run every task in a sequential crew, persisting an assignment per task.
    pub async fn run(
        &self,
        crew: &mut Crew,
        workspace_id: &WorkspaceId,
        requester: AssignmentRequester,
        origin: AssignmentOrigin,
        approval_level: ApprovalLevel,
    ) -> CowResult<HashMap<TaskId, String>> {
        let tasks = crew.tasks.clone();
        let mut completed: HashMap<TaskId, String> = HashMap::new();
        let approvals = ApprovalService::new(
            self.assignments.store().clone(),
            self.assignments.bus().clone(),
            workspace_id.clone(),
        );

        for task in tasks {
            let assignee = task.assigned_agent.clone().ok_or_else(|| {
                CowError::Task(format!("task '{}' has no assigned agent", task.id))
            })?;

            if self.assignments.store().get_task(&task.id).await?.is_none() {
                self.assignments
                    .store()
                    .create_task(workspace_id, None, &task)
                    .await?;
            }

            let agent = crew.agents.get_mut(&assignee).ok_or_else(|| {
                CowError::Agent(format!(
                    "assigned agent '{}' was not found in the crew",
                    assignee
                ))
            })?;

            let assignment = self
                .assignments
                .enqueue_root(&task, origin, requester.clone(), assignee, approval_level)
                .await?;
            self.assignments.start(&assignment.id).await?;

            let context: Vec<String> = task
                .context_tasks
                .iter()
                .filter_map(|dependency| completed.get(dependency).cloned())
                .collect();

            let gate = Arc::new(ScopedApprovalGate::new(
                approvals.clone(),
                assignment.id.clone(),
            ));
            agent.set_tool_gate(Some(gate));
            let execution = agent
                .execute_task(&task, &context, self.assignments.bus())
                .await;
            agent.clear_tool_gate();

            match execution {
                Ok(output) => {
                    self.assignments.complete(&assignment.id, &output).await?;
                    completed.insert(task.id.clone(), output);
                }
                Err(error) => {
                    self.assignments
                        .fail(&assignment.id, error.to_string())
                        .await?;
                    return Err(error);
                }
            }
        }

        Ok(completed)
    }
}
