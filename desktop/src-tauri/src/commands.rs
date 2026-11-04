use cow_client::Client;
use cow_core::channel::{Channel, ChannelId, ChannelMessage};
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_session(state: State<'_, AppState>) -> Result<cow_client::ClientSession, String> {
    state
        .client
        .session()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_channels(state: State<'_, AppState>) -> Result<Vec<Channel>, String> {
    state
        .client
        .list_channels()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn create_channel(
    state: State<'_, AppState>,
    name: String,
    topic: String,
) -> Result<Channel, String> {
    state
        .client
        .create_channel(&name, &topic)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_messages(
    state: State<'_, AppState>,
    channel_id: String,
    limit: u32,
) -> Result<Vec<ChannelMessage>, String> {
    state
        .client
        .list_messages(&ChannelId::new(channel_id), limit)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn post_message(
    state: State<'_, AppState>,
    channel_id: String,
    content: String,
) -> Result<ChannelMessage, String> {
    state
        .client
        .post_message(&ChannelId::new(channel_id), &content)
        .await
        .map_err(|error| error.to_string())
}
