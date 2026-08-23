use tauri::{Manager, PhysicalPosition, PhysicalRect, PhysicalSize, WebviewUrl, WindowEvent};

use crate::services::surface_activity::{hide_surface, show_surface};
use crate::services::tool_windows::{
    anchored_tool_window_position, hide_tool_window, position_anchored_tool_window,
    prepare_tool_window, ToolWindowAnchor, ToolWindowKind, ToolWindowLogicalSize,
};

pub const PAPER_WINDOW_LABEL: &str = ToolWindowKind::Paper.label();
const PAPER_WINDOW_GAP: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperWindowOptions {
    pub label: &'static str,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub skip_taskbar: bool,
}

pub type PaperWindowAnchor = ToolWindowAnchor;

pub fn paper_window_options() -> PaperWindowOptions {
    PaperWindowOptions {
        label: PAPER_WINDOW_LABEL,
        width: 400.0,
        height: 300.0,
        resizable: false,
        decorations: false,
        transparent: true,
        always_on_top: true,
        skip_taskbar: true,
    }
}

pub fn paper_window_position(
    anchor: PaperWindowAnchor,
    window_size: PhysicalSize<u32>,
    work_area: PhysicalRect<i32, u32>,
) -> Option<PhysicalPosition<i32>> {
    anchored_tool_window_position(anchor, window_size, work_area, PAPER_WINDOW_GAP)
}

pub fn hide_paper_window(app: &tauri::AppHandle) -> Result<(), String> {
    hide_tool_window(app, ToolWindowKind::Paper)
}

pub fn toggle_paper_window(
    app: &tauri::AppHandle,
    anchor: Option<PaperWindowAnchor>,
) -> Result<(), String> {
    let options = paper_window_options();
    if let Some(window) = app.get_webview_window(options.label) {
        if window
            .is_visible()
            .map_err(|error| format!("failed to read Zero Paper visibility: {error}"))?
        {
            return hide_surface(&window)
                .map_err(|error| format!("failed to hide Zero Paper window: {error}"));
        }
    }

    prepare_tool_window(app, ToolWindowKind::Paper)?;
    let window = get_or_create_paper_window(app, options)?;
    position_paper_window(&window, anchor, options)?;
    show_surface(&window).map_err(|error| format!("failed to show Zero Paper window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to focus Zero Paper window: {error}"))?;
    Ok(())
}

fn get_or_create_paper_window(
    app: &tauri::AppHandle,
    options: PaperWindowOptions,
) -> Result<tauri::WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(options.label) {
        return Ok(window);
    }

    let window = tauri::WebviewWindowBuilder::new(app, options.label, WebviewUrl::App("".into()))
        .title("Zero Paper")
        .inner_size(options.width, options.height)
        .resizable(options.resizable)
        .decorations(options.decorations)
        .transparent(options.transparent)
        .always_on_top(options.always_on_top)
        .skip_taskbar(options.skip_taskbar)
        .focused(true)
        .visible(false)
        .build()
        .map_err(|error| format!("failed to create Zero Paper window: {error}"))?;

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

fn position_paper_window(
    window: &tauri::WebviewWindow,
    anchor: Option<PaperWindowAnchor>,
    options: PaperWindowOptions,
) -> Result<(), String> {
    position_anchored_tool_window(
        window,
        anchor,
        ToolWindowLogicalSize {
            width: options.width,
            height: options.height,
        },
        PAPER_WINDOW_GAP,
        "Zero Paper",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_window_options_are_compact_and_host_controlled() {
        let options = paper_window_options();
        assert_eq!(options.label, "paper");
        assert_eq!((options.width, options.height), (400.0, 300.0));
        assert!(!options.resizable);
        assert!(!options.decorations);
        assert!(options.transparent);
        assert!(options.always_on_top);
        assert!(options.skip_taskbar);
    }

    #[test]
    fn paper_window_centers_under_anchor_and_clamps_to_work_area() {
        let size = PhysicalSize::new(400, 300);
        let work_area = PhysicalRect {
            position: PhysicalPosition::new(0, 24),
            size: PhysicalSize::new(1440, 876),
        };
        assert_eq!(
            paper_window_position(
                PaperWindowAnchor {
                    x: 700,
                    y: 0,
                    width: 22,
                    height: 22,
                },
                size,
                work_area,
            ),
            Some(PhysicalPosition::new(511, 28))
        );
        assert_eq!(
            paper_window_position(
                PaperWindowAnchor {
                    x: 4,
                    y: 0,
                    width: 22,
                    height: 22,
                },
                size,
                work_area,
            ),
            Some(PhysicalPosition::new(0, 28))
        );
        assert_eq!(
            paper_window_position(
                PaperWindowAnchor {
                    x: 1420,
                    y: 0,
                    width: 22,
                    height: 22,
                },
                size,
                work_area,
            ),
            Some(PhysicalPosition::new(1040, 28))
        );
    }

    #[test]
    fn paper_window_rejects_invalid_or_oversized_geometry() {
        assert_eq!(
            paper_window_position(
                PaperWindowAnchor {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 22,
                },
                PhysicalSize::new(400, 300),
                PhysicalRect {
                    position: PhysicalPosition::new(0, 0),
                    size: PhysicalSize::new(320, 240),
                },
            ),
            None
        );
    }
}
