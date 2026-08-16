use crate::services::global_shortcuts::{
    global_shortcut_snapshots as global_shortcut_snapshots_service, GlobalShortcutSnapshot,
};

#[tauri::command]
pub fn get_global_shortcut_snapshots(
    app: tauri::AppHandle,
) -> Result<Vec<GlobalShortcutSnapshot>, String> {
    global_shortcut_snapshots_service(&app)
}
