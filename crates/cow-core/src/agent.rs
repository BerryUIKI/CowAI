use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent definition capturing role, goal, backstory, and capabilities (inspired by CrewAI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Unique identifier for the agent
    pub id: AgentId,
    /// Display name of the agent (e.g. "Cow-Coder")
    pub name: String,
    /// Role description (e.g. "Senior Rust Systems Engineer")
    pub role: String,
    /// High-level objective the agent seeks to accomplish
    pub goal: String,
    /// Contextual background and persona for prompt generation
    pub backstory: String,
    /// Available tool names assigned to this agent
    pub tools: Vec<String>,
    /// Whether this agent is permitted to delegate sub-tasks to other agents
    pub allow_delegation: bool,
    /// Maximum internal reasoning / tool call iterations per task
    pub max_iterations: usize,
    /// Whether to emit verbose trace logs and intermediate thoughts
    pub verbose: bool,
}

impl AgentProfile {
    pub fn new(
        name: impl Into<String>,
        role: impl Into<String>,
        goal: impl Into<String>,
        backstory: impl Into<String>,
    ) -> Self {
        Self {
            id: AgentId::random(),
            name: name.into(),
            role: role.into(),
            goal: goal.into(),
            backstory: backstory.into(),
            tools: Vec::new(),
            allow_delegation: false,
            max_iterations: 15,
            verbose: true,
        }
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }

    pub fn with_delegation(mut self, allow: bool) -> Self {
        self.allow_delegation = allow;
        self
    }

    /// Formats the foundational system prompt derived from role, goal, and backstory.
    pub fn build_system_prompt(&self) -> String {
        format!(
            "You are {name}.\n\
             Your Role: {role}\n\
             Your Goal: {goal}\n\
             Your Background: {backstory}\n\n\
             Operate collaboratively with other agents and humans in the platform. \
             Provide structured outputs and use tools when appropriate.",
            name = self.name,
            role = self.role,
            goal = self.goal,
            backstory = self.backstory
        )
    }
}
