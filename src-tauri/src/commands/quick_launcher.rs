use tauri::{Emitter, Manager, WebviewUrl};

use crate::services::quick_launcher::contracts::{
    QuickLauncherActivateInput, QuickLauncherActivationResult, QuickLauncherError,
    QuickLauncherIconBatchInput, QuickLauncherIconBatchResult, QuickLauncherIconInput,
    QuickLauncherIconResult, QuickLauncherIndexSnapshot, QuickLauncherRunningSnapshot,
    QuickLauncherSearchInput, QuickLauncherSearchResult,
};
use crate::services::quick_launcher::{system_language, QuickLauncherState};
use crate::services::surface_activity::{hide_surface, show_surface};
use crate::services::tool_windows::{hide_tool_window, prepare_tool_window, ToolWindowKind};

pub const LAUNCHER_WINDOW_LABEL: &str = ToolWindowKind::QuickLauncher.label();
pub const QUICK_LAUNCHER_RUNNING_UPDATED_EVENT: &str =
    "zero://quick-launcher/running-state-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLauncherToggleAction {
    Show,
    Hide,
}

pub fn quick_launcher_toggle_action(is_visible: bool) -> QuickLauncherToggleAction {
    if is_visible {
        QuickLauncherToggleAction::Hide
    } else {
        QuickLauncherToggleAction::Show
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuickLauncherWindowOptions {
    pub label: &'static str,
    pub width: f64,
    pub height: f64,
    pub min_width: f64,
    pub min_height: f64,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub skip_taskbar: bool,
}

pub fn quick_launcher_window_options() -> QuickLauncherWindowOptions {
    QuickLauncherWindowOptions {
        label: LAUNCHER_WINDOW_LABEL,
        width: 680.0,
        height: 420.0,
        min_width: 520.0,
        min_height: 320.0,
        resizable: false,
        decorations: false,
        transparent: true,
        always_on_top: true,
        skip_taskbar: true,
    }
}

#[tauri::command]
pub async fn get_quick_launcher_snapshot(
    app: tauri::AppHandle,
) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
    run_initialized(app, |state| state.snapshot()).await
}

#[tauri::command]
pub async fn refresh_quick_launcher_index(
    app: tauri::AppHandle,
) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
    run_initialized(app, |state| state.refresh(&system_language())).await
}

#[tauri::command]
pub async fn search_quick_launcher(
    input: QuickLauncherSearchInput,
    app: tauri::AppHandle,
) -> Result<QuickLauncherSearchResult, QuickLauncherError> {
    run_initialized(app, move |state| state.search(input)).await
}

#[tauri::command]
pub async fn get_quick_launcher_icon(
    input: QuickLauncherIconInput,
    app: tauri::AppHandle,
) -> Result<QuickLauncherIconResult, QuickLauncherError> {
    run_initialized(app, move |state| state.icon(input)).await
}

#[tauri::command]
pub async fn get_quick_launcher_icons(
    input: QuickLauncherIconBatchInput,
    app: tauri::AppHandle,
) -> Result<QuickLauncherIconBatchResult, QuickLauncherError> {
    run_initialized(app, move |state| state.icons(input)).await
}

#[tauri::command]
pub async fn refresh_quick_launcher_running_state(
    app: tauri::AppHandle,
) -> Result<QuickLauncherRunningSnapshot, QuickLauncherError> {
    let event_app = app.clone();
    let snapshot = run_initialized(app, |state| state.refresh_running_states()).await?;
    event_app
        .emit(QUICK_LAUNCHER_RUNNING_UPDATED_EVENT, &snapshot)
        .map_err(|error| {
            crate::services::quick_launcher::contracts::launcher_error(
                "launcher.running_state",
                "launcher.running_event_failed",
                format!("Could not publish running-state update: {error}"),
                true,
            )
        })?;
    Ok(snapshot)
}

#[tauri::command]
pub async fn activate_quick_launcher_item(
    input: QuickLauncherActivateInput,
    app: tauri::AppHandle,
) -> Result<QuickLauncherActivationResult, QuickLauncherError> {
    run_initialized(app, move |state| state.activate(input)).await
}

async fn run_initialized<T: Send + 'static>(
    app: tauri::AppHandle,
    operation: impl FnOnce(&QuickLauncherState) -> Result<T, QuickLauncherError> + Send + 'static,
) -> Result<T, QuickLauncherError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<QuickLauncherState>();
        state.initialize(app.clone())?;
        operation(&state)
    })
    .await
    .map_err(|error| {
        crate::services::quick_launcher::contracts::launcher_error(
            "launcher.initialize",
            "launcher.initialize_join_failed",
            format!("Launcher initialization task failed: {error}"),
            true,
        )
    })?
}

#[tauri::command]
pub fn show_quick_launcher_window(app: tauri::AppHandle) -> Result<(), String> {
    prepare_tool_window(&app, ToolWindowKind::QuickLauncher)?;
    let options = quick_launcher_window_options();
    let window = if let Some(window) = app.get_webview_window(options.label) {
        window
    } else {
        tauri::WebviewWindowBuilder::new(&app, options.label, WebviewUrl::App("".into()))
            .title("Zero Launch")
            .inner_size(options.width, options.height)
            .min_inner_size(options.min_width, options.min_height)
            .resizable(options.resizable)
            .decorations(options.decorations)
            .transparent(options.transparent)
            .always_on_top(options.always_on_top)
            .skip_taskbar(options.skip_taskbar)
            .focused(true)
            .visible(false)
            .build()
            .map_err(|error| format!("failed to create Zero Launch window: {error}"))?
    };
    window
        .center()
        .map_err(|error| format!("failed to center Zero Launch window: {error}"))?;
    show_surface(&window).map_err(|error| format!("failed to show Zero Launch window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to focus Zero Launch window: {error}"))?;
    let _ = window.emit("quick-launcher-shown", ());
    Ok(())
}

pub fn toggle_quick_launcher_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LAUNCHER_WINDOW_LABEL) {
        let is_visible = window
            .is_visible()
            .map_err(|error| format!("failed to read Zero Launch visibility: {error}"))?;
        if quick_launcher_toggle_action(is_visible) == QuickLauncherToggleAction::Hide {
            return hide_surface(&window)
                .map_err(|error| format!("failed to hide Zero Launch window: {error}"));
        }
    }

    show_quick_launcher_window(app.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_window_options_are_fixed_and_host_controlled() {
        let options = quick_launcher_window_options();
        assert_eq!(options.label, "launcher");
        assert_eq!((options.width, options.height), (680.0, 420.0));
        assert!(!options.resizable);
        assert!(!options.decorations);
        assert!(options.transparent);
        assert!(options.always_on_top);
        assert!(options.skip_taskbar);
    }

    #[test]
    fn status_bar_activation_toggles_launcher_visibility() {
        assert_eq!(
            quick_launcher_toggle_action(false),
            QuickLauncherToggleAction::Show
        );
        assert_eq!(
            quick_launcher_toggle_action(true),
            QuickLauncherToggleAction::Hide
        );
    }
}

#[tauri::command]
pub fn hide_quick_launcher_window(app: tauri::AppHandle) -> Result<(), String> {
    hide_tool_window(&app, ToolWindowKind::QuickLauncher)
}
