use std::fs;
use std::path::{Path, PathBuf};

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn bing_wallpaper_owns_paper_window_and_wallpaper_adapter_modules() {
    assert!(manifest_path("src/commands/bing_wallpaper/mod.rs").is_file());
    assert!(manifest_path("src/commands/bing_wallpaper/window.rs").is_file());
    assert!(manifest_path("src/services/bing_wallpaper/mod.rs").is_file());
    assert!(manifest_path("src/services/bing_wallpaper/wallpaper.rs").is_file());
    assert!(!manifest_path("src/commands/paper.rs").exists());
    assert!(!manifest_path("src/services/wallpaper.rs").exists());
}

#[test]
fn concrete_window_commands_depend_on_the_host_coordinator_not_peer_plugins() {
    let launcher = fs::read_to_string(manifest_path("src/commands/quick_launcher.rs")).unwrap();
    let paper = fs::read_to_string(manifest_path("src/commands/bing_wallpaper/window.rs")).unwrap();

    assert!(launcher.contains("services::tool_windows"));
    assert!(paper.contains("services::tool_windows"));
    assert!(!launcher.contains("commands::bing_wallpaper"));
    assert!(!paper.contains("commands::quick_launcher"));
}

#[test]
fn rust_composition_registers_each_bundled_plugin_state_and_handler_group() {
    let composition = fs::read_to_string(manifest_path("src/bundled_plugins.rs")).unwrap();
    for state in [
        "CaffeineState",
        "BingWallpaperState",
        "FileConversionState",
        "QuickLauncherState",
        "ScreenshotSessionStore",
    ] {
        assert!(composition.contains(state), "missing bundled state {state}");
    }

    let lib = fs::read_to_string(manifest_path("src/lib.rs")).unwrap();
    for handler in [
        "commands::caffeine::get_caffeine_state",
        "commands::file::get_file_conversion_capabilities",
        "commands::bing_wallpaper::get_bing_wallpaper_snapshot",
        "commands::quick_launcher::get_quick_launcher_snapshot",
        "commands::screenshot::get_screenshot_capabilities",
    ] {
        assert!(lib.contains(handler), "missing bundled handler {handler}");
    }
}
