use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::plugins::contracts::NativeResourceError;

use crate::services::native_resources::{resolve_plugin_file, resource_error};

const MAX_WALLPAPER_BYTES: u64 = 25 * 1024 * 1024;

pub trait WallpaperSetter: Send + Sync {
    fn set_from_path(&self, path: &Path) -> Result<(), NativeResourceError>;
}

#[derive(Debug, Default)]
pub struct SystemWallpaperSetter;

impl WallpaperSetter for SystemWallpaperSetter {
    fn set_from_path(&self, path: &Path) -> Result<(), NativeResourceError> {
        let path = path.to_str().ok_or_else(|| {
            resource_error(
                "system.setWallpaper",
                "wallpaper.path_encoding",
                "Wallpaper path is not valid UTF-8.",
                false,
            )
        })?;

        wallpaper::set_from_path(path).map_err(|error| {
            let message = error.to_string();
            resource_error(
                "system.setWallpaper",
                wallpaper_backend_error_code(&message),
                format!("Operating system rejected the wallpaper change: {message}"),
                true,
            )
        })
    }
}

fn wallpaper_backend_error_code(_message: &str) -> &'static str {
    #[cfg(target_os = "linux")]
    if _message.contains("No such file or directory") || _message.contains("not found") {
        return "dependency_missing";
    }

    "wallpaper.backend"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperPlatformCapability {
    pub platform: String,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub fn platform_capability() -> WallpaperPlatformCapability {
    #[cfg(target_os = "macos")]
    {
        return WallpaperPlatformCapability {
            platform: "macos".into(),
            supported: true,
            detail: None,
        };
    }

    #[cfg(target_os = "windows")]
    {
        return WallpaperPlatformCapability {
            platform: "windows".into(),
            supported: true,
            detail: None,
        };
    }

    #[cfg(target_os = "linux")]
    {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_default();
        let supported = !desktop.trim().is_empty();
        return WallpaperPlatformCapability {
            platform: "linux".into(),
            supported,
            detail: (!supported).then(|| {
                "No supported desktop environment was detected; XDG_CURRENT_DESKTOP and DESKTOP_SESSION are empty."
                    .into()
            }),
        };
    }

    #[allow(unreachable_code)]
    WallpaperPlatformCapability {
        platform: std::env::consts::OS.into(),
        supported: false,
        detail: Some("This platform does not expose a desktop wallpaper API.".into()),
    }
}

pub fn set_plugin_wallpaper(
    setter: &dyn WallpaperSetter,
    plugin_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, NativeResourceError> {
    set_plugin_wallpaper_with_capability(setter, plugin_root, relative_path, platform_capability())
}

fn set_plugin_wallpaper_with_capability(
    setter: &dyn WallpaperSetter,
    plugin_root: &Path,
    relative_path: &str,
    capability: WallpaperPlatformCapability,
) -> Result<PathBuf, NativeResourceError> {
    if !capability.supported {
        return Err(resource_error(
            "system.setWallpaper",
            "platform_unsupported",
            capability
                .detail
                .unwrap_or_else(|| "Desktop wallpaper is not supported on this platform.".into()),
            false,
        ));
    }

    let path = validate_plugin_image(plugin_root, relative_path)?;
    setter.set_from_path(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        set_plugin_wallpaper_with_capability, WallpaperPlatformCapability, WallpaperSetter,
    };
    use crate::plugins::contracts::NativeResourceError;

    struct NoopSetter;

    impl WallpaperSetter for NoopSetter {
        fn set_from_path(&self, _path: &Path) -> Result<(), NativeResourceError> {
            Ok(())
        }
    }

    #[test]
    fn unsupported_platform_fails_before_resolving_or_applying_a_file() {
        let error = set_plugin_wallpaper_with_capability(
            &NoopSetter,
            Path::new("/unused"),
            "wallpaper.jpg",
            WallpaperPlatformCapability {
                platform: "mobile".into(),
                supported: false,
                detail: Some("No desktop wallpaper API is available.".into()),
            },
        )
        .expect_err("unsupported platform should fail");

        assert_eq!(error.code, "platform_unsupported");
        assert!(!error.retryable);
    }
}

pub fn validate_plugin_image(
    plugin_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, NativeResourceError> {
    let path = resolve_plugin_file(plugin_root, relative_path).map_err(|mut error| {
        error.operation = "system.setWallpaper".into();
        error
    })?;
    let metadata = fs::metadata(&path).map_err(|error| {
        resource_error(
            "system.setWallpaper",
            "wallpaper.file_metadata",
            format!("Failed to inspect wallpaper image: {error}"),
            true,
        )
    })?;

    if metadata.len() == 0 || metadata.len() > MAX_WALLPAPER_BYTES {
        return Err(resource_error(
            "system.setWallpaper",
            "wallpaper.file_size",
            "Wallpaper image is empty or exceeds the 25 MiB limit.",
            false,
        ));
    }

    let bytes = fs::read(&path).map_err(|error| {
        resource_error(
            "system.setWallpaper",
            "wallpaper.file_read",
            format!("Failed to read wallpaper image: {error}"),
            true,
        )
    })?;
    image::load_from_memory(&bytes).map_err(|error| {
        resource_error(
            "system.setWallpaper",
            "wallpaper.image_invalid",
            format!("Wallpaper file is not a supported image: {error}"),
            false,
        )
    })?;

    Ok(path)
}
