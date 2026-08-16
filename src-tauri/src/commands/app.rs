use tauri::{Manager, WebviewUrl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppShellWindow {
    Main,
    Preferences,
    About,
}

struct AppShellWindowOptions {
    label: &'static str,
    title: &'static str,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
    resizable: bool,
}

impl AppShellWindow {
    fn options(self) -> AppShellWindowOptions {
        match self {
            Self::Main => AppShellWindowOptions {
                label: "main",
                title: "Zero",
                width: 920.0,
                height: 660.0,
                min_width: 760.0,
                min_height: 520.0,
                resizable: true,
            },
            Self::Preferences => AppShellWindowOptions {
                label: "preferences",
                title: "Zero Preferences",
                width: 840.0,
                height: 640.0,
                min_width: 420.0,
                min_height: 500.0,
                resizable: true,
            },
            Self::About => AppShellWindowOptions {
                label: "about",
                title: "About Zero",
                width: 460.0,
                height: 420.0,
                min_width: 420.0,
                min_height: 360.0,
                resizable: false,
            },
        }
    }
}

#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    show_app_shell_window(&app, AppShellWindow::Main)
}

#[tauri::command]
pub fn show_preferences_window(app: tauri::AppHandle) -> Result<(), String> {
    show_app_shell_window(&app, AppShellWindow::Preferences)
}

#[tauri::command]
pub fn show_about_window(app: tauri::AppHandle) -> Result<(), String> {
    show_app_shell_window(&app, AppShellWindow::About)
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

fn show_app_shell_window(app: &tauri::AppHandle, target: AppShellWindow) -> Result<(), String> {
    let options = target.options();
    promote_app_for_app_window(app);

    if let Some(existing) = app.get_webview_window(options.label) {
        existing.show().map_err(|e| format!("显示窗口失败: {e}"))?;
        existing
            .set_focus()
            .map_err(|e| format!("聚焦窗口失败: {e}"))?;
        return Ok(());
    }

    let window = tauri::WebviewWindowBuilder::new(app, options.label, WebviewUrl::App("".into()))
        .title(options.title)
        .inner_size(options.width, options.height)
        .min_inner_size(options.min_width, options.min_height)
        .resizable(options.resizable)
        .decorations(true)
        .transparent(false)
        .skip_taskbar(false)
        .focused(true)
        .visible(false)
        .build()
        .map_err(|e| format!("创建窗口失败: {e}"))?;

    window.show().map_err(|e| format!("显示窗口失败: {e}"))?;
    window
        .set_focus()
        .map_err(|e| format!("聚焦窗口失败: {e}"))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn promote_app_for_app_window(app: &tauri::AppHandle) {
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
}

#[cfg(not(target_os = "macos"))]
fn promote_app_for_app_window(_app: &tauri::AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_shell_window_labels_are_explicit() {
        assert_eq!(AppShellWindow::Main.options().label, "main");
        assert_eq!(AppShellWindow::Preferences.options().label, "preferences");
        assert_eq!(AppShellWindow::About.options().label, "about");
    }

    #[test]
    fn main_window_is_roomier_than_system_windows() {
        let main = AppShellWindow::Main.options();
        let preferences = AppShellWindow::Preferences.options();
        let about = AppShellWindow::About.options();

        assert!(main.width > preferences.width);
        assert!(main.height > about.height);
        assert!(main.resizable);
        assert!(!about.resizable);
    }

    #[test]
    fn preferences_window_supports_regular_and_compact_layouts() {
        let preferences = AppShellWindow::Preferences.options();

        assert_eq!(preferences.label, "preferences");
        assert_eq!((preferences.width, preferences.height), (840.0, 640.0));
        assert!(preferences.min_width <= 420.0);
        assert!(preferences.resizable);
    }
}
