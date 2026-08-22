use tauri::{Manager, State};

use crate::services::bing_wallpaper::{
    BingWallpaperActionInput, BingWallpaperActionResult, BingWallpaperError, BingWallpaperPreview,
    BingWallpaperPreviewResourceInput, BingWallpaperSnapshot, BingWallpaperState,
};

mod window;

pub use window::{
    hide_paper_window, paper_window_options, paper_window_position, toggle_paper_window,
    PaperWindowAnchor, PaperWindowOptions, PAPER_WINDOW_LABEL,
};

#[tauri::command]
pub fn get_bing_wallpaper_snapshot(
    state: State<'_, BingWallpaperState>,
) -> Result<BingWallpaperSnapshot, BingWallpaperError> {
    Ok(state.snapshot())
}

#[tauri::command]
pub async fn refresh_bing_wallpapers(
    state: State<'_, BingWallpaperState>,
) -> Result<BingWallpaperSnapshot, BingWallpaperError> {
    Ok(state.refresh().await)
}

#[tauri::command]
pub async fn get_bing_wallpaper_preview(
    input: BingWallpaperActionInput,
    state: State<'_, BingWallpaperState>,
) -> Result<BingWallpaperPreview, BingWallpaperError> {
    state.preview(&input.wallpaper_id).await
}

#[tauri::command]
pub async fn read_bing_wallpaper_preview(
    input: BingWallpaperPreviewResourceInput,
    app: tauri::AppHandle,
) -> Result<tauri::ipc::Response, BingWallpaperError> {
    let read_app = app.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        read_app
            .state::<BingWallpaperState>()
            .read_preview_bytes(&input.token)
    })
    .await
    .map_err(|_| BingWallpaperError {
        code: "preview.worker".into(),
        message: "The wallpaper preview read worker stopped unexpectedly.".into(),
        retryable: true,
    })??;
    crate::services::performance::record_media_transfer(&app, "paper_preview_read", bytes.len());
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn release_bing_wallpaper_preview(
    input: BingWallpaperPreviewResourceInput,
    state: State<'_, BingWallpaperState>,
) {
    state.release_preview(&input.token);
}

#[tauri::command]
pub async fn save_bing_wallpaper_to_downloads(
    input: BingWallpaperActionInput,
    app: tauri::AppHandle,
    state: State<'_, BingWallpaperState>,
) -> Result<BingWallpaperActionResult, BingWallpaperError> {
    let downloads = app
        .path()
        .download_dir()
        .map_err(|path_error| BingWallpaperError {
            code: "downloads.unavailable".into(),
            message: format!("Failed to resolve Downloads directory: {path_error}"),
            retryable: true,
        })?;
    state
        .save_to_downloads(&input.wallpaper_id, &downloads)
        .await
}

#[tauri::command]
pub async fn apply_bing_wallpaper(
    input: BingWallpaperActionInput,
    state: State<'_, BingWallpaperState>,
) -> Result<BingWallpaperActionResult, BingWallpaperError> {
    state.apply_with_system_setter(&input.wallpaper_id).await
}
