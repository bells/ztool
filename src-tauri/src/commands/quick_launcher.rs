use tauri::{Emitter, Manager, State, WebviewUrl};

use crate::services::quick_launcher::contracts::{
    QuickLauncherActivateInput, QuickLauncherActivationResult, QuickLauncherError,
    QuickLauncherIconInput, QuickLauncherIconResult, QuickLauncherIndexSnapshot,
    QuickLauncherSearchInput, QuickLauncherSearchResult,
};
use crate::services::quick_launcher::{system_language, QuickLauncherState};
use crate::services::tool_windows::{hide_tool_window, prepare_tool_window, ToolWindowKind};

pub const LAUNCHER_WINDOW_LABEL: &str = ToolWindowKind::QuickLauncher.label();

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
pub fn get_quick_launcher_snapshot(
    state: State<'_, QuickLauncherState>,
) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
    state.snapshot()
}

#[tauri::command]
pub async fn refresh_quick_launcher_index(
    app: tauri::AppHandle,
) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<QuickLauncherState>()
            .refresh(&system_language())
    })
    .await
    .map_err(|error| {
        crate::services::quick_launcher::contracts::launcher_error(
            "launcher.refresh",
            "launcher.refresh_join_failed",
            format!("Launcher refresh task failed: {error}"),
            true,
        )
    })?
}

#[tauri::command]
pub fn search_quick_launcher(
    input: QuickLauncherSearchInput,
    state: State<'_, QuickLauncherState>,
) -> Result<QuickLauncherSearchResult, QuickLauncherError> {
    state.search(input)
}

#[tauri::command]
pub fn get_quick_launcher_icon(
    input: QuickLauncherIconInput,
    state: State<'_, QuickLauncherState>,
) -> Result<QuickLauncherIconResult, QuickLauncherError> {
    state.icon(input)
}

#[tauri::command]
pub fn activate_quick_launcher_item(
    input: QuickLauncherActivateInput,
    state: State<'_, QuickLauncherState>,
) -> Result<QuickLauncherActivationResult, QuickLauncherError> {
    state.activate(input)
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
    window
        .show()
        .map_err(|error| format!("failed to show Zero Launch window: {error}"))?;
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
            return window
                .hide()
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
