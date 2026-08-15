use tauri::{Manager, State};

use crate::services::bing_wallpaper::{
    BingWallpaperActionInput, BingWallpaperActionResult, BingWallpaperError, BingWallpaperPreview,
    BingWallpaperSnapshot, BingWallpaperState,
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
