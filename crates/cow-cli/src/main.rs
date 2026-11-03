use clap::{Parser, Subcommand};
use cow_agent::{AgentInstance, Crew, ProcessType, StandardRoles};
use cow_bus::EventBus;
use cow_core::event::EventPayload;
use cow_core::task::Task;
use cow_llm::{LlmProvider, MockLlmProvider, OpenAiCompatibleProvider};
use cow_tools::{
    CodeExecutionTool, ReadFileTool, SandboxBackend, SubprocessSandbox, ToolRegistry, WriteFileTool,
};
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "cowai")]
#[command(author = "CowAI Contributors")]
#[command(version = "0.1.0")]
#[command(about = "CowAI: High-performance Multi-Agent Collaboration Platform in Rust", long_about = None)]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an autonomous multi-agent crew on an engineering task
    Run {
        /// The objective or task prompt to execute
        #[arg(short, long)]
        task: String,

        /// Model name to use (e.g. gpt-4o, deepseek-chat, llama3)
        #[arg(short, long, default_value = "gpt-4o")]
        model: String,

        /// Custom OpenAI-compatible base URL (e.g. http://localhost:11434/v1 for Ollama)
        #[arg(long, env = "OPENAI_BASE_URL")]
        base_url: Option<String>,

        /// API key for the LLM provider
        #[arg(long, env = "OPENAI_API_KEY")]
        api_key: Option<String>,

        /// Use mock LLM provider for local dry-run testing
        #[arg(long)]
        dry_run: bool,
    },

    /// Inspect registered tools and test sandbox code execution
    Tools,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    match cli.command {
        Commands::Run {
            task,
            model,
            base_url,
            api_key,
            dry_run,
        } => {
            run_crew(task, model, base_url, api_key, dry_run).await?;
        }
        Commands::Tools => {
            inspect_tools().await?;
        }
    }

    Ok(())
}

async fn run_crew(
    task_prompt: String,
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
    dry_run: bool,
) -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║       🐄 CowAI Multi-Agent Collaboration Platform        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    println!("Objective: {}\n", task_prompt);

    // Initialize Event Bus
    let bus = EventBus::default_bus();
    let mut rx = bus.subscribe();

    // Spawn event listener to print live platform activities
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event.payload {
                EventPayload::TaskCreated(t) => {
                    println!("[TASK CREATED] {}", t.description);
                }
                EventPayload::TaskAssigned {
                    task_id: _,
                    agent_id,
                } => {
                    println!("[ASSIGNED] -> Agent '{}'", agent_id);
                }
                EventPayload::AgentThought { agent_id, thought } => {
                    println!("\n💭 [{}] Thought:\n{}", agent_id, thought.trim());
                }
                EventPayload::ToolCallRequested { agent_id, call } => {
                    println!(
                        "\n🔧 [{}] Tool Call: {} (args: {})",
                        agent_id, call.name, call.arguments
                    );
                }
                EventPayload::ToolCallExecuted { agent_id, result } => {
                    let status = if result.success { "SUCCESS" } else { "FAILED" };
                    println!(
                        "⚡ [{}] Tool Result [{}]:\n{}",
                        agent_id,
                        status,
                        result.output.trim()
                    );
                }
                EventPayload::TaskCompleted { task_id: _, output } => {
                    println!("\n✅ [TASK COMPLETED]\n{}\n", output.trim());
                }
                EventPayload::TaskFailed { task_id: _, error } => {
                    println!("\n❌ [TASK FAILED]: {}", error);
                }
                _ => {}
            }
        }
    });

    // Initialize Sandbox & Tools
    let sandbox = Arc::new(SubprocessSandbox::default());
    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(CodeExecutionTool::new(sandbox)));
    tools.register(Arc::new(ReadFileTool::new(None)));
    tools.register(Arc::new(WriteFileTool::new(None)));

    // Initialize LLM Provider
    let llm: Arc<dyn LlmProvider> = if dry_run || (api_key.is_none() && base_url.is_none()) {
        println!("Note: Running in dry-run mode using MockLlmProvider.");
        println!("      Set OPENAI_API_KEY or --base-url to use a live LLM endpoint.\n");
        Arc::new(MockLlmProvider::echo())
    } else {
        let url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        Arc::new(OpenAiCompatibleProvider::new(url, api_key))
    };

    // Assemble Crew (Planner + Coder + Reviewer)
    let planner_profile = StandardRoles::planner();
    let planner_id = planner_profile.id.clone();
    let planner = AgentInstance::new(planner_profile, llm.clone(), tools.clone(), &model);

    let coder_profile = StandardRoles::coder();
    let coder_id = coder_profile.id.clone();
    let coder = AgentInstance::new(coder_profile, llm.clone(), tools.clone(), &model);

    let reviewer_profile = StandardRoles::reviewer();
    let reviewer_id = reviewer_profile.id.clone();
    let reviewer = AgentInstance::new(reviewer_profile, llm.clone(), tools.clone(), &model);

    let mut crew = Crew::new(ProcessType::Sequential);
    crew.add_agent(planner);
    crew.add_agent(coder);
    crew.add_agent(reviewer);

    // Build Task Pipeline
    let plan_task = Task::new(
        format!(
            "Analyze objective and formulate architecture plan: {}",
            task_prompt
        ),
        "A clear, step-by-step implementation and verification plan.",
    )
    .assign_to(planner_id);
    let plan_id = plan_task.id.clone();

    let code_task = Task::new(
        "Execute code implementation and verify functionality in the sandbox.",
        "Working source code and sandbox execution results.",
    )
    .assign_to(coder_id)
    .with_context_task(plan_id.clone());
    let code_id = code_task.id.clone();

    let review_task = Task::new(
        "Review implementation, tests, and diffs for correctness and edge cases.",
        "Comprehensive review findings and final approval status.",
    )
    .assign_to(reviewer_id)
    .with_context_task(code_id);

    crew.add_task(plan_task);
    crew.add_task(code_task);
    crew.add_task(review_task);

    // Execute!
    let results = crew.kickoff(&bus).await?;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Execution Summary                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    for (task_id, output) in results {
        println!("\nTask ID: {}\nDeliverable:\n{}", task_id, output);
    }

    Ok(())
}

async fn inspect_tools() -> anyhow::Result<()> {
    let sandbox = Arc::new(SubprocessSandbox::default());
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(CodeExecutionTool::new(sandbox.clone())));
    registry.register(Arc::new(ReadFileTool::new(None)));
    registry.register(Arc::new(WriteFileTool::new(None)));

    println!("\nAvailable CowAI Tools:\n");
    for def in registry.definitions() {
        println!("* {} - {}", def.name, def.description);
        println!("  Schema: {}\n", def.parameters);
    }

    println!("Testing Subprocess Sandbox execution...");
    let cmd = "echo Sandbox Active";
    let res = sandbox.execute(cmd, None, 5).await?;
    println!("Sandbox Test Exit Code: {:?}", res.exit_code);
    println!("Sandbox Test Output: {}", res.stdout.trim());

    Ok(())
}
