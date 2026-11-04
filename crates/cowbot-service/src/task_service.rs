use cowbot_bus::EventBus;
use cowbot_core::approval::ApprovalLevel;
use cowbot_core::assignment::{Assignment, AssignmentOrigin, AssignmentRequester};
use cowbot_core::channel::ChannelId;
use cowbot_core::error::{CowError, CowResult};
use cowbot_core::identity::WorkspaceId;
use cowbot_core::task::{Task, TaskId};
use cowbot_store::Store;

use crate::assignments::AssignmentService;

/// Application service for task creation, lookup, and cancellation.
#[derive(Clone)]
pub struct TaskService {
    store: Store,
    assignments: AssignmentService,
}

impl TaskService {
    pub fn new(store: Store, bus: EventBus) -> Self {
        Self {
            assignments: AssignmentService::new(store.clone(), bus),
            store,
        }
    }

    /// Persist a task and enqueue its root assignment.
    pub async fn create(
        &self,
        workspace_id: &WorkspaceId,
        channel_id: Option<&ChannelId>,
        task: &Task,
        requester: AssignmentRequester,
        approval_level: ApprovalLevel,
    ) -> CowResult<Assignment> {
        let assignee = task
            .assigned_agent
            .clone()
            .ok_or_else(|| CowError::Task(format!("task '{}' has no assigned agent", task.id)))?;
        self.store
            .create_task(workspace_id, channel_id, task)
            .await?;
        self.assignments
            .enqueue_root(
                task,
                AssignmentOrigin::User,
                requester,
                assignee,
                approval_level,
            )
            .await
    }

    pub async fn list(&self, workspace_id: &WorkspaceId) -> CowResult<Vec<Task>> {
        self.store.list_tasks(workspace_id).await
    }

    pub async fn get(&self, id: &TaskId) -> CowResult<Option<Task>> {
        self.store.get_task(id).await
    }

    /// Cancel the task's active root assignment; task status follows the
    /// assignment lifecycle.
    pub async fn cancel(&self, task_id: &TaskId, reason: impl Into<String>) -> CowResult<()> {
        let assignments = self.store.list_assignments_for_task(task_id).await?;
        let root = assignments
            .iter()
            .find(|assignment| assignment.parent_id.is_none() && !assignment.is_terminal())
            .ok_or_else(|| CowError::Task(format!("task '{task_id}' has no active assignment")))?;
        self.assignments.cancel(&root.id, reason).await
    }
}
