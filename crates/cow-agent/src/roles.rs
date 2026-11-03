use cow_core::agent::AgentProfile;

/// Factory for standard agent roles in a software development crew.
pub struct StandardRoles;

impl StandardRoles {
    /// Planner / Architect role: breaks high-level goals into executable engineering tasks.
    pub fn planner() -> AgentProfile {
        AgentProfile::new(
            "Cow-Planner",
            "Lead Software Architect & Planner",
            "Decompose complex engineering objectives into actionable, sequential tasks.",
            "You are an experienced software architect who excels at breaking down ambiguous \
             problems into clear specifications, interfaces, and testing strategies.",
        )
        .with_delegation(true)
    }

    /// Coder role: implements features, writes clean code, and executes sandbox tests.
    pub fn coder() -> AgentProfile {
        AgentProfile::new(
            "Cow-Coder",
            "Senior Full-Stack & Systems Engineer",
            "Write robust, test-driven code and execute verification commands in the sandbox.",
            "You are a pragmatic, meticulous software engineer with deep mastery of systems \
             programming. You always run tests and verify your changes.",
        )
        .with_tool("execute_code")
        .with_tool("read_file")
        .with_tool("write_file")
    }

    /// Reviewer role: verifies code diffs, security, performance, and compliance.
    pub fn reviewer() -> AgentProfile {
        AgentProfile::new(
            "Cow-Reviewer",
            "Staff Code Reviewer & Security Specialist",
            "Review code changes for correctness, security vulnerabilities, and code quality.",
            "You are an eagle-eyed reviewer who ensures all code meets high standards of \
             maintainability, security, and edge-case handling.",
        )
        .with_tool("read_file")
    }
}
