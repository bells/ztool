use tauri::Manager;

use crate::plugins::registry::PluginRegistryState;
use crate::services::status_bar::{
    ensure_status_bar_settings, run_status_bar_item_action as run_status_bar_item_action_service,
    status_bar_items, update_status_bar_settings as update_status_bar_settings_service,
    RunStatusBarItemActionInput, StatusBarItemSnapshot, StatusBarSettings,
    UpdateStatusBarSettingsInput,
};

#[tauri::command]
pub async fn get_status_bar_settings(app: tauri::AppHandle) -> Result<StatusBarSettings, String> {
    run_status_bar_worker(app, |app| {
        let records = app
            .state::<PluginRegistryState>()
            .with_registry(|registry| Ok(registry.records().to_vec()))?;
        ensure_status_bar_settings(app, &records)
    })
    .await
}

#[tauri::command]
pub async fn update_status_bar_settings(
    app: tauri::AppHandle,
    input: UpdateStatusBarSettingsInput,
) -> Result<StatusBarSettings, String> {
    run_status_bar_worker(app, move |app| {
        update_status_bar_settings_service(app, input)
    })
    .await
}

#[tauri::command]
pub async fn get_status_bar_items(
    app: tauri::AppHandle,
) -> Result<Vec<StatusBarItemSnapshot>, String> {
    run_status_bar_worker(app, status_bar_items).await
}

#[tauri::command]
pub async fn run_status_bar_item_action(
    app: tauri::AppHandle,
    input: RunStatusBarItemActionInput,
) -> Result<(), String> {
    run_status_bar_worker(app, move |app| {
        run_status_bar_item_action_service(app, &input.item_id)
    })
    .await
}

async fn run_status_bar_worker<T: Send + 'static>(
    app: tauri::AppHandle,
    operation: impl FnOnce(&tauri::AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || operation(&app))
        .await
        .map_err(|_| "status bar worker stopped unexpectedly".to_string())?
}
