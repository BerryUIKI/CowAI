mod commands;
mod migration;

use std::sync::Arc;

use cowbot_client::{Client, LocalClient};
use tauri::{Emitter, Manager};

/// Shared state for Tauri commands.
pub struct AppState {
    pub client: Arc<LocalClient>,
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
