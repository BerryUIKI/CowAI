pub mod agent;
pub mod crew;
pub mod roles;

pub use agent::AgentInstance;
pub use crew::{Crew, ProcessType};
pub use roles::StandardRoles;

#[cfg(test)]
mod tests {
    use super::*;
    use berry_bus::EventBus;
    use berry_core::task::Task;
    use berry_llm::MockLlmProvider;
    use berry_tools::{CodeExecutionTool, SubprocessSandbox, ToolRegistry};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_crew_sequential_execution() {
        let bus = EventBus::default_bus();
        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(CodeExecutionTool::new(Arc::new(
            SubprocessSandbox::default(),
        ))));

        let mock_llm = Arc::new(MockLlmProvider::echo());

        let planner_profile = StandardRoles::planner();
        let planner_id = planner_profile.id.clone();
        let planner = AgentInstance::new(
            planner_profile,
            mock_llm.clone(),
            tools.clone(),
            "mock-model",
        );

        let coder_profile = StandardRoles::coder();
        let coder_id = coder_profile.id.clone();
        let coder =
            AgentInstance::new(coder_profile, mock_llm.clone(), tools.clone(), "mock-model");

        let mut crew = Crew::new(ProcessType::Sequential);
        crew.add_agent(planner);
        crew.add_agent(coder);

        let task1 = Task::new("Create implementation plan", "List of steps").assign_to(planner_id);
        let task1_id = task1.id.clone();

        let task2 = Task::new("Implement the code", "Working source code")
            .assign_to(coder_id)
            .with_context_task(task1_id.clone());

        crew.add_task(task1);
        crew.add_task(task2);

        let results = crew.kickoff(&bus).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains_key(&task1_id));
    }
}
