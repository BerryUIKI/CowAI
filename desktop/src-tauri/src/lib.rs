mod commands;

use std::sync::Arc;

use cow_client::{Client, LocalClient};
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
            let db_path = data_dir.join("cowai.db");
            let client = tauri::async_runtime::block_on(LocalClient::open(&db_path))?;

            let mut events = client.subscribe();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = events.recv().await {
                    let _ = handle.emit("cowai-event", &event);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CowAI");
}
