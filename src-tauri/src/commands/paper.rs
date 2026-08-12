use tauri::{Manager, PhysicalPosition, PhysicalRect, PhysicalSize, WebviewUrl, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

pub const PAPER_WINDOW_LABEL: &str = "paper";
const TRAY_WINDOW_LABEL: &str = "tray";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaperWindowAnchor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

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
    if anchor.width == 0
        || anchor.height == 0
        || window_size.width == 0
        || window_size.height == 0
        || work_area.size.width < window_size.width
        || work_area.size.height < window_size.height
    {
        return None;
    }

    let anchor_center_x = i64::from(anchor.x) + i64::from(anchor.width) / 2;
    let preferred_x = anchor_center_x - i64::from(window_size.width) / 2;
    let preferred_y = i64::from(anchor.y) + i64::from(anchor.height) + i64::from(PAPER_WINDOW_GAP);
    let min_x = i64::from(work_area.position.x);
    let min_y = i64::from(work_area.position.y);
    let max_x = min_x + i64::from(work_area.size.width - window_size.width);
    let max_y = min_y + i64::from(work_area.size.height - window_size.height);

    Some(PhysicalPosition::new(
        preferred_x.clamp(min_x, max_x) as i32,
        preferred_y.clamp(min_y, max_y) as i32,
    ))
}

pub fn hide_paper_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(PAPER_WINDOW_LABEL) {
        window
            .hide()
            .map_err(|error| format!("failed to hide Zero Paper window: {error}"))?;
    }
    Ok(())
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
            return window
                .hide()
                .map_err(|error| format!("failed to hide Zero Paper window: {error}"));
        }
    }

    hide_transient_window(app, TRAY_WINDOW_LABEL)?;
    crate::commands::quick_launcher::hide_quick_launcher_window(app.clone())?;
    let window = get_or_create_paper_window(app, options)?;
    position_paper_window(&window, anchor, options)?;
    window
        .show()
        .map_err(|error| format!("failed to show Zero Paper window: {error}"))?;
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
                    let _ = dismiss_window.hide();
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
    if let Some(anchor) = anchor {
        let monitor = window
            .monitor_from_point(
                f64::from(anchor.x) + f64::from(anchor.width) / 2.0,
                f64::from(anchor.y) + f64::from(anchor.height) / 2.0,
            )
            .map_err(|error| format!("failed to resolve Zero Paper monitor: {error}"))?;
        if let Some(monitor) = monitor {
            let scale_factor = monitor.scale_factor();
            let window_size = PhysicalSize::new(
                (options.width * scale_factor).round() as u32,
                (options.height * scale_factor).round() as u32,
            );
            if let Some(position) = paper_window_position(anchor, window_size, *monitor.work_area())
            {
                return window
                    .set_position(position)
                    .map_err(|error| format!("failed to position Zero Paper window: {error}"));
            }
        }
    }

    window
        .as_ref()
        .window()
        .move_window(Position::TrayCenter)
        .map_err(|error| format!("failed to place Zero Paper near the status bar: {error}"))
}

fn hide_transient_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        window
            .hide()
            .map_err(|error| format!("failed to hide {label} window: {error}"))?;
    }
    Ok(())
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
