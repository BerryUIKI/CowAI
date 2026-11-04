mod commands;
mod migration;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use cowbot_client::{Client, LocalClient};
use cowbot_llm::{LlmProvider, MockLlmProvider, OpenAiCompatibleProvider};
use cowbot_tools::{
    CodeExecutionTool, ReadFileTool, SubprocessSandbox, ToolRegistry, WriteFileTool,
};
use tauri::{Emitter, Manager};

/// Shared state for Tauri commands.
pub struct AppState {
    pub client: Arc<LocalClient>,
    pub llm: Arc<dyn LlmProvider>,
    pub tools: ToolRegistry,
    pub model: String,
    /// Task ids with a crew currently executing.
    pub running: Mutex<HashSet<String>>,
}

impl AppState {
    /// Build the default LLM provider from the environment.
    ///
    /// Mirrors the CLI: a base URL or API key selects the OpenAI-compatible
    /// provider; otherwise the mock provider keeps the app usable offline.
    fn build_llm() -> Arc<dyn LlmProvider> {
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        let base_url = std::env::var("OPENAI_BASE_URL").ok();
        if api_key.is_none() && base_url.is_none() {
            return Arc::new(MockLlmProvider::echo());
        }
        let url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        Arc::new(OpenAiCompatibleProvider::new(url, api_key))
    }

    fn build_tools(workspace_dir: &std::path::Path) -> ToolRegistry {
        let sandbox = Arc::new(SubprocessSandbox::default());
        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(CodeExecutionTool::new(sandbox)));
        tools.register(Arc::new(ReadFileTool::new(Some(
            workspace_dir.to_path_buf(),
        ))));
        tools.register(Arc::new(WriteFileTool::new(Some(
            workspace_dir.to_path_buf(),
        ))));
        tools
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            migration::migrate_legacy_database(&data_dir);
            let db_path = data_dir.join("cowbot.db");
            let client = tauri::async_runtime::block_on(LocalClient::open(&db_path))?;
            // Local mode spawns the forwarding task, so subscribe must run
            // inside the runtime context.
            let mut events = tauri::async_runtime::block_on(async { client.subscribe() });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = events.recv().await {
                    let _ = handle.emit("cowbot-event", &event);
                }
            });

            app.manage(AppState {
                client: Arc::new(client),
                llm: AppState::build_llm(),
                tools: AppState::build_tools(&data_dir),
                model: "gpt-4o".to_string(),
                running: Mutex::new(HashSet::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_session,
            commands::list_channels,
            commands::create_channel,
            commands::list_messages,
            commands::post_message,
            commands::list_agents,
            commands::list_templates,
            commands::instantiate_agents,
            commands::create_task,
            commands::run_task,
            commands::list_tasks,
            commands::cancel_task,
            commands::list_assignments,
            commands::list_assignment_events,
            commands::list_runs,
            commands::list_tool_calls,
            commands::pending_approvals,
            commands::decide_approval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CowBot");
}
