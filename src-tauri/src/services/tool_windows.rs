use tauri::{Manager, PhysicalPosition, PhysicalRect, PhysicalSize};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::services::surface_activity::hide_surface;

const TRAY_WINDOW_LABEL: &str = "tray";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolWindowKind {
    Paper,
    QuickLauncher,
    SnapMenu,
}

impl ToolWindowKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::QuickLauncher => "launcher",
            Self::SnapMenu => "snap-menu",
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::Paper => "Zero Paper",
            Self::QuickLauncher => "Zero Launch",
            Self::SnapMenu => "Zero Snap",
        }
    }
}

const TOOL_WINDOWS: [ToolWindowKind; 3] = [
    ToolWindowKind::Paper,
    ToolWindowKind::QuickLauncher,
    ToolWindowKind::SnapMenu,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolWindowAnchor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolWindowLogicalSize {
    pub width: f64,
    pub height: f64,
}

pub fn anchored_tool_window_position(
    anchor: ToolWindowAnchor,
    window_size: PhysicalSize<u32>,
    work_area: PhysicalRect<i32, u32>,
    gap: i32,
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
    let preferred_y = i64::from(anchor.y) + i64::from(anchor.height) + i64::from(gap);
    let min_x = i64::from(work_area.position.x);
    let min_y = i64::from(work_area.position.y);
    let max_x = min_x + i64::from(work_area.size.width - window_size.width);
    let max_y = min_y + i64::from(work_area.size.height - window_size.height);

    Some(PhysicalPosition::new(
        preferred_x.clamp(min_x, max_x) as i32,
        preferred_y.clamp(min_y, max_y) as i32,
    ))
}

pub fn position_anchored_tool_window(
    window: &tauri::WebviewWindow,
    anchor: Option<ToolWindowAnchor>,
    logical_size: ToolWindowLogicalSize,
    gap: i32,
    display_name: &str,
) -> Result<(), String> {
    if let Some(anchor) = anchor {
        let monitor = window.monitor_from_point(
            f64::from(anchor.x) + f64::from(anchor.width) / 2.0,
            f64::from(anchor.y) + f64::from(anchor.height) / 2.0,
        );
        if let Ok(Some(monitor)) = monitor {
            let scale_factor = monitor.scale_factor();
            let window_size = PhysicalSize::new(
                (logical_size.width * scale_factor).round() as u32,
                (logical_size.height * scale_factor).round() as u32,
            );
            if let Some(position) =
                anchored_tool_window_position(anchor, window_size, *monitor.work_area(), gap)
            {
                return window
                    .set_position(position)
                    .map_err(|error| format!("failed to position {display_name} window: {error}"));
            }
        }
    }

    window
        .as_ref()
        .window()
        .move_window(Position::TrayCenter)
        .map_err(|error| format!("failed to place {display_name} near the status bar: {error}"))
}

pub fn prepare_tool_window(app: &tauri::AppHandle, opening: ToolWindowKind) -> Result<(), String> {
    hide_window(app, TRAY_WINDOW_LABEL, "Zero tray")?;
    for tool in peer_tool_windows(opening) {
        hide_tool_window(app, tool)?;
    }
    Ok(())
}

pub fn hide_tool_window(app: &tauri::AppHandle, tool: ToolWindowKind) -> Result<(), String> {
    hide_window(app, tool.label(), tool.display_name())
}

fn hide_window(app: &tauri::AppHandle, label: &str, display_name: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        hide_surface(&window)
            .map_err(|error| format!("failed to hide {display_name} window: {error}"))?;
    }
    Ok(())
}

fn peer_tool_windows(opening: ToolWindowKind) -> impl Iterator<Item = ToolWindowKind> {
    TOOL_WINDOWS
        .into_iter()
        .filter(move |tool| *tool != opening)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_window_labels_are_host_owned_and_stable() {
        assert_eq!(ToolWindowKind::Paper.label(), "paper");
        assert_eq!(ToolWindowKind::QuickLauncher.label(), "launcher");
        assert_eq!(ToolWindowKind::SnapMenu.label(), "snap-menu");
    }

    #[test]
    fn opening_a_tool_hides_only_peer_tool_windows() {
        assert_eq!(
            peer_tool_windows(ToolWindowKind::Paper).collect::<Vec<_>>(),
            vec![ToolWindowKind::QuickLauncher, ToolWindowKind::SnapMenu]
        );
        assert_eq!(
            peer_tool_windows(ToolWindowKind::QuickLauncher).collect::<Vec<_>>(),
            vec![ToolWindowKind::Paper, ToolWindowKind::SnapMenu]
        );
        assert_eq!(
            peer_tool_windows(ToolWindowKind::SnapMenu).collect::<Vec<_>>(),
            vec![ToolWindowKind::Paper, ToolWindowKind::QuickLauncher]
        );
    }

    #[test]
    fn anchored_windows_center_below_the_exact_cell_and_clamp_to_work_areas() {
        let anchor = ToolWindowAnchor {
            x: -22,
            y: -900,
            width: 22,
            height: 22,
        };
        let work_area = tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(-1920, -876),
            size: tauri::PhysicalSize::new(1920, 876),
        };
        assert_eq!(
            anchored_tool_window_position(anchor, tauri::PhysicalSize::new(464, 152), work_area, 6,),
            Some(tauri::PhysicalPosition::new(-464, -872))
        );
        assert_eq!(
            anchored_tool_window_position(
                ToolWindowAnchor { width: 0, ..anchor },
                tauri::PhysicalSize::new(464, 152),
                work_area,
                6,
            ),
            None
        );

        let right_display = tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(1440, 24),
            size: tauri::PhysicalSize::new(2560, 1416),
        };
        assert_eq!(
            anchored_tool_window_position(
                ToolWindowAnchor {
                    x: 3978,
                    y: 0,
                    width: 22,
                    height: 22,
                },
                tauri::PhysicalSize::new(504, 184),
                right_display,
                6,
            ),
            Some(tauri::PhysicalPosition::new(3496, 28))
        );

        let lower_display = tauri::PhysicalRect {
            position: tauri::PhysicalPosition::new(0, 900),
            size: tauri::PhysicalSize::new(1440, 900),
        };
        assert_eq!(
            anchored_tool_window_position(
                ToolWindowAnchor {
                    x: 700,
                    y: 876,
                    width: 22,
                    height: 22,
                },
                tauri::PhysicalSize::new(252, 92),
                lower_display,
                6,
            ),
            Some(tauri::PhysicalPosition::new(585, 904))
        );

        assert_eq!(
            anchored_tool_window_position(
                ToolWindowAnchor {
                    x: 0,
                    y: 0,
                    width: 22,
                    height: 22,
                },
                tauri::PhysicalSize::new(252, 92),
                tauri::PhysicalRect {
                    position: tauri::PhysicalPosition::new(0, 0),
                    size: tauri::PhysicalSize::new(200, 80),
                },
                6,
            ),
            None
        );
    }
}
