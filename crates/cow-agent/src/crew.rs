use crate::agent::AgentInstance;
use cow_bus::EventBus;
use cow_core::agent::AgentId;
use cow_core::error::{CowError, CowResult};
use cow_core::event::{Event, EventPayload};
use cow_core::task::{Task, TaskId};
use std::collections::HashMap;
use tracing::info;

/// Orchestration strategy for coordinating a crew of agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessType {
    /// Tasks are executed in order, passing prerequisite outputs downstream
    Sequential,
    /// Future extension: Hierarchical manager agent delegation
    Hierarchical,
}

/// A collaborative group of agents working on a set of tasks (inspired by CrewAI).
pub struct Crew {
    pub agents: HashMap<AgentId, AgentInstance>,
    pub tasks: Vec<Task>,
    pub process: ProcessType,
}

impl Crew {
    pub fn new(process: ProcessType) -> Self {
        Self {
            agents: HashMap::new(),
            tasks: Vec::new(),
            process,
        }
    }

    pub fn add_agent(&mut self, agent: AgentInstance) {
        self.agents.insert(agent.profile.id.clone(), agent);
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Kicks off crew execution and coordinates tasks across agents.
    pub async fn kickoff(&mut self, bus: &EventBus) -> CowResult<HashMap<TaskId, String>> {
        info!("Kicking off Crew execution with {} tasks", self.tasks.len());

        let mut completed_outputs: HashMap<TaskId, String> = HashMap::new();

        match self.process {
            ProcessType::Sequential => {
                for task in &self.tasks {
                    let task_id = task.id.clone();

                    let assigned_id = task.assigned_agent.as_ref().ok_or_else(|| {
                        CowError::Task(format!("Task '{}' has no assigned agent", task.description))
                    })?;

                    let agent = self.agents.get_mut(assigned_id).ok_or_else(|| {
                        CowError::Agent(format!(
                            "Assigned agent '{}' not found in crew",
                            assigned_id
                        ))
                    })?;

                    // Notify task creation and assignment on event bus
                    let _ = bus.publish(Event::new(EventPayload::TaskCreated(task.clone())));
                    let _ = bus.publish(Event::new(EventPayload::TaskAssigned {
                        task_id: task_id.clone(),
                        agent_id: assigned_id.clone(),
                    }));

                    // Collect prerequisite outputs
                    let mut context = Vec::new();
                    for dep_id in &task.context_tasks {
                        if let Some(out) = completed_outputs.get(dep_id) {
                            context.push(out.clone());
                        }
                    }

                    match agent.execute_task(task, &context, bus).await {
                        Ok(output) => {
                            let _ = bus.publish(Event::new(EventPayload::TaskCompleted {
                                task_id: task_id.clone(),
                                output: output.clone(),
                            }));
                            completed_outputs.insert(task_id, output);
                        }
                        Err(e) => {
                            let _ = bus.publish(Event::new(EventPayload::TaskFailed {
                                task_id: task_id.clone(),
                                error: e.to_string(),
                            }));
                            return Err(e);
                        }
                    }
                }
            }
            ProcessType::Hierarchical => {
                return Err(CowError::Task(
                    "Hierarchical process mode is scheduled for future release".to_string(),
                ));
            }
        }

        info!("Crew execution completed successfully");
        Ok(completed_outputs)
    }
}
