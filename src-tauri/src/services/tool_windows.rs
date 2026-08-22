use tauri::Manager;

use crate::services::surface_activity::hide_surface;

const TRAY_WINDOW_LABEL: &str = "tray";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolWindowKind {
    Paper,
    QuickLauncher,
}

impl ToolWindowKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::QuickLauncher => "launcher",
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::Paper => "Zero Paper",
            Self::QuickLauncher => "Zero Launch",
        }
    }
}

const TOOL_WINDOWS: [ToolWindowKind; 2] = [ToolWindowKind::Paper, ToolWindowKind::QuickLauncher];

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
    }

    #[test]
    fn opening_a_tool_hides_only_peer_tool_windows() {
        assert_eq!(
            peer_tool_windows(ToolWindowKind::Paper).collect::<Vec<_>>(),
            vec![ToolWindowKind::QuickLauncher]
        );
        assert_eq!(
            peer_tool_windows(ToolWindowKind::QuickLauncher).collect::<Vec<_>>(),
            vec![ToolWindowKind::Paper]
        );
    }
}
