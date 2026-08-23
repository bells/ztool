use tauri::{Manager, WebviewUrl, WindowEvent};

use crate::services::surface_activity::{hide_surface, show_surface};
use crate::services::tool_windows::{
    hide_tool_window, position_anchored_tool_window, prepare_tool_window, ToolWindowAnchor,
    ToolWindowKind, ToolWindowLogicalSize,
};

pub const SNAP_MENU_WINDOW_LABEL: &str = ToolWindowKind::SnapMenu.label();
const SNAP_MENU_WINDOW_GAP: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapMenuWindowOptions {
    pub label: &'static str,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub skip_taskbar: bool,
    pub focused: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapMenuToggleAction {
    Show,
    Hide,
}

fn snap_menu_toggle_action(is_visible: bool) -> SnapMenuToggleAction {
    if is_visible {
        SnapMenuToggleAction::Hide
    } else {
        SnapMenuToggleAction::Show
    }
}

pub fn snap_menu_window_options() -> SnapMenuWindowOptions {
    SnapMenuWindowOptions {
        label: SNAP_MENU_WINDOW_LABEL,
        width: 252.0,
        height: 92.0,
        resizable: false,
        decorations: false,
        transparent: true,
        always_on_top: true,
        skip_taskbar: true,
        focused: false,
        visible: false,
    }
}

pub fn hide_snap_menu_window(app: &tauri::AppHandle) -> Result<(), String> {
    hide_tool_window(app, ToolWindowKind::SnapMenu)
}

pub fn restore_snap_menu_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(SNAP_MENU_WINDOW_LABEL)
        .ok_or_else(|| "Zero Snap menu window is unavailable".to_string())?;
    show_surface(&window).map_err(|error| format!("failed to restore Zero Snap menu: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to refocus Zero Snap menu: {error}"))
}

pub fn toggle_snap_menu_window(
    app: &tauri::AppHandle,
    anchor: Option<ToolWindowAnchor>,
) -> Result<(), String> {
    let options = snap_menu_window_options();
    if let Some(window) = app.get_webview_window(options.label) {
        let is_visible = window
            .is_visible()
            .map_err(|error| format!("failed to read Zero Snap menu visibility: {error}"))?;
        if snap_menu_toggle_action(is_visible) == SnapMenuToggleAction::Hide {
            return hide_surface(&window)
                .map_err(|error| format!("failed to hide Zero Snap menu: {error}"));
        }
    }

    prepare_tool_window(app, ToolWindowKind::SnapMenu)?;
    let window = get_or_create_snap_menu_window(app, options)?;
    position_anchored_tool_window(
        &window,
        anchor,
        ToolWindowLogicalSize {
            width: options.width,
            height: options.height,
        },
        SNAP_MENU_WINDOW_GAP,
        "Zero Snap menu",
    )?;
    show_surface(&window).map_err(|error| format!("failed to show Zero Snap menu: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to focus Zero Snap menu: {error}"))
}

fn get_or_create_snap_menu_window(
    app: &tauri::AppHandle,
    options: SnapMenuWindowOptions,
) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(options.label) {
        return Ok(window);
    }

    let window = tauri::WebviewWindowBuilder::new(app, options.label, WebviewUrl::App("".into()))
        .title("Zero Snap")
        .inner_size(options.width, options.height)
        .resizable(options.resizable)
        .decorations(options.decorations)
        .transparent(options.transparent)
        .always_on_top(options.always_on_top)
        .skip_taskbar(options.skip_taskbar)
        .focused(options.focused)
        .visible(options.visible)
        .build()
        .map_err(|error| format!("failed to create Zero Snap menu: {error}"))?;

    let dismiss_window = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            let dismiss_window = dismiss_window.clone();
            tauri::async_runtime::spawn_blocking(move || {
                std::thread::sleep(std::time::Duration::from_millis(120));
                if !dismiss_window.is_focused().unwrap_or(false) {
                    let _ = hide_surface(&dismiss_window);
                }
            });
        }
    });
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_menu_window_is_compact_and_host_controlled() {
        let options = snap_menu_window_options();
        assert_eq!(options.label, "snap-menu");
        assert_eq!((options.width, options.height), (252.0, 92.0));
        assert!(!options.resizable);
        assert!(!options.decorations);
        assert!(options.transparent);
        assert!(options.always_on_top);
        assert!(options.skip_taskbar);
        assert!(!options.focused);
        assert!(!options.visible);
    }

    #[test]
    fn repeated_visible_snap_activation_hides_instead_of_reopening() {
        assert_eq!(snap_menu_toggle_action(false), SnapMenuToggleAction::Show);
        assert_eq!(snap_menu_toggle_action(true), SnapMenuToggleAction::Hide);
    }
}
