pub mod agent;
pub mod crew;
pub mod roles;

pub use agent::AgentInstance;
pub use crew::{Crew, ProcessType};
pub use roles::StandardRoles;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use cow_bus::EventBus;
    use cow_core::agent::AgentId;
    use cow_core::error::CowError;
    use cow_core::task::Task;
    use cow_core::tool::{ToolAuthorization, ToolCall, ToolGate};
    use cow_llm::{ChatCompletionResponse, LlmToolCall, MockLlmProvider};
    use cow_tools::{CodeExecutionTool, SubprocessSandbox, ToolRegistry};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct DenyGate {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ToolGate for DenyGate {
        async fn authorize(
            &self,
            _agent_id: &AgentId,
            _call: &ToolCall,
        ) -> Result<ToolAuthorization, CowError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolAuthorization::Denied {
                reason: "denied in test".to_string(),
            })
        }
    }

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

    #[tokio::test]
    async fn tool_gate_can_deny_tool_calls() {
        let bus = EventBus::default_bus();
        let responses = vec![
            ChatCompletionResponse {
                content: Some("trying".to_string()),
                tool_calls: vec![LlmToolCall {
                    id: "call-1".to_string(),
                    name: "execute_code".to_string(),
                    arguments: json!({ "command": "echo hi" }),
                }],
                finish_reason: Some("tool_calls".to_string()),
            },
            ChatCompletionResponse {
                content: Some("finished".to_string()),
                tool_calls: Vec::new(),
                finish_reason: Some("stop".to_string()),
            },
        ];
        let llm = Arc::new(MockLlmProvider::new(responses));
        let mut agent = AgentInstance::new(
            StandardRoles::coder(),
            llm,
            ToolRegistry::new(),
            "mock-model",
        );

        let calls = Arc::new(AtomicUsize::new(0));
        agent.set_tool_gate(Some(Arc::new(DenyGate {
            calls: calls.clone(),
        })));

        let task = Task::new("Run a command", "Output");
        let output = agent.execute_task(&task, &[], &bus).await.unwrap();
        assert_eq!(output, "finished");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
