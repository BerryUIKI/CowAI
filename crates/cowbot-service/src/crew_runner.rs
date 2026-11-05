use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use cowbot_agent::Crew;
use cowbot_bus::EventBus;
use cowbot_core::approval::ApprovalLevel;
use cowbot_core::assignment::{AssignmentOrigin, AssignmentRequester};
use cowbot_core::error::{CowError, CowResult};
use cowbot_core::identity::WorkspaceId;
use cowbot_core::run::{Run, RunId, RunStatus, ToolCallRecord};
use cowbot_core::task::TaskId;
use cowbot_core::tool::{ToolCall, ToolCallRecorder, ToolResult};
use cowbot_store::Store;

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
                .enqueue_root(
                    &task,
                    origin,
                    requester.clone(),
                    assignee.clone(),
                    approval_level,
                )
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

            let run = Run::new(task.id.clone(), assignee.clone());
            self.assignments.store().create_run(&run).await?;
            agent.set_tool_recorder(Some(Arc::new(RunToolCallRecorder {
                store: self.assignments.store().clone(),
                run_id: run.id.clone(),
            })));

            let execution = agent
                .execute_task(&task, &context, self.assignments.bus())
                .await;
            agent.clear_tool_recorder();
            agent.clear_tool_gate();

            match execution {
                Ok(output) => {
                    self.assignments
                        .store()
                        .update_run_status(&run.id, RunStatus::Completed, Some(Utc::now()))
                        .await?;
                    self.assignments.complete(&assignment.id, &output).await?;
                    completed.insert(task.id.clone(), output);
                }
                Err(error) => {
                    self.assignments
                        .store()
                        .update_run_status(&run.id, RunStatus::Failed, Some(Utc::now()))
                        .await?;
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

/// Persists tool calls under the run that produced them.
struct RunToolCallRecorder {
    store: Store,
    run_id: RunId,
}

#[async_trait]
impl ToolCallRecorder for RunToolCallRecorder {
    async fn record_tool_call(
        &self,
        call: &ToolCall,
        result: &ToolResult,
        approved_by: Option<&str>,
    ) -> CowResult<()> {
        let mut record = ToolCallRecord::new(
            self.run_id.clone(),
            call.name.clone(),
            call.arguments.clone(),
        );
        record.id = call.id.clone();
        record.result = Some(serde_json::to_value(result).map_err(CowError::from)?);
        record.approved_by = approved_by.map(str::to_string);
        self.store.record_tool_call(&record).await
    }
}
