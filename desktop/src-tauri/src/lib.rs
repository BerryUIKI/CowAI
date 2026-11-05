mod commands;
mod config;
mod migration;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use cowbot_client::{Client, LocalClient};
use cowbot_llm::LlmProvider;
use cowbot_tools::{
    CodeExecutionTool, ReadFileTool, SubprocessSandbox, ToolRegistry, WriteFileTool,
};
use tauri::{Emitter, Manager};

/// Model configuration currently in effect.
pub struct ModelRuntime {
    pub llm: Arc<dyn LlmProvider>,
    pub model: String,
}

/// Shared state for Tauri commands.
pub struct AppState {
    pub client: Arc<LocalClient>,
    pub tools: ToolRegistry,
    /// Active model provider; replaced when the user switches providers.
    pub model: RwLock<ModelRuntime>,
    /// Path of the local `config.json` holding model providers.
    pub config_path: PathBuf,
    /// Task ids with a crew currently executing.
    pub running: Mutex<HashSet<String>>,
}

impl AppState {
    /// Snapshot the provider handle and model name for one crew run.
    pub fn runtime_model(&self) -> Result<(Arc<dyn LlmProvider>, String), String> {
        let runtime = self.model.read().map_err(|error| error.to_string())?;
        Ok((runtime.llm.clone(), runtime.model.clone()))
    }

    /// Rebuild the LLM client after the provider configuration changed.
    pub fn apply_config(&self, config: &config::AppConfig) -> Result<(), String> {
        let resolved = config::resolve_active(config);
        let llm = config::build_provider(&resolved);
        let mut runtime = self.model.write().map_err(|error| error.to_string())?;
        runtime.model = resolved.model;
        runtime.llm = llm;
        Ok(())
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

            let config_path = config::config_path(&data_dir);
            let config = match config::load(&config_path) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!(
                        "warning: failed to read {}: {error}; using built-in defaults",
                        config_path.display()
                    );
                    config::AppConfig::default()
                }
            };
            let resolved = config::resolve_active(&config);

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
                tools: AppState::build_tools(&data_dir),
                model: RwLock::new(ModelRuntime {
                    llm: config::build_provider(&resolved),
                    model: resolved.model,
                }),
                config_path,
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
            commands::get_model_settings,
            commands::save_model_provider,
            commands::delete_model_provider,
            commands::activate_model_provider,
            commands::test_model_provider,
            commands::get_onboarding_state,
            commands::complete_onboarding,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CowBot");
}
