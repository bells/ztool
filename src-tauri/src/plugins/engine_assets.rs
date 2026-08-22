use std::fs;
use std::path::{Component, Path, PathBuf};

use tauri::http::{header, Method, Request, Response, StatusCode};
use tauri::Manager;

use crate::brand::ZERO_FILE_PLUGIN_ID;
use crate::services::file::engine_bridge::FILE_ENGINE_LABEL;

use super::registry::PluginRegistryState;

pub const FILE_ENGINE_SCHEME: &str = "zero-file-engine";
const MAX_ENGINE_ASSET_BYTES: u64 = 48 * 1024 * 1024;
const ENGINE_CSP: &str = "default-src 'none'; connect-src ipc: http://ipc.localhost; img-src 'self' data: blob:; font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; worker-src 'self' blob:; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'; navigate-to 'none'";

pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_uri_scheme_protocol(FILE_ENGINE_SCHEME, |context, request| {
        serve_engine_asset(context.app_handle(), context.webview_label(), &request)
    })
}

fn serve_engine_asset(
    app: &tauri::AppHandle,
    webview_label: &str,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if webview_label != FILE_ENGINE_LABEL {
        return response(
            StatusCode::FORBIDDEN,
            "text/plain; charset=utf-8",
            b"forbidden".to_vec(),
        );
    }
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return response(
            StatusCode::METHOD_NOT_ALLOWED,
            "text/plain; charset=utf-8",
            b"method not allowed".to_vec(),
        );
    }
    let Some((version, relative_path)) = parse_asset_request_path(request.uri().path()) else {
        return response(
            StatusCode::BAD_REQUEST,
            "text/plain; charset=utf-8",
            b"invalid asset path".to_vec(),
        );
    };
    let root = match app
        .state::<PluginRegistryState>()
        .with_registry(|registry| registry.engine_asset_root(ZERO_FILE_PLUGIN_ID, &version))
    {
        Ok(root) => root,
        Err(_) => {
            return response(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"engine version unavailable".to_vec(),
            )
        }
    };
    let target = match resolve_read_only_asset(&root, &relative_path) {
        Ok(target) => target,
        Err(_) => {
            return response(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"asset unavailable".to_vec(),
            )
        }
    };
    let metadata = match fs::metadata(&target) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_ENGINE_ASSET_BYTES => metadata,
        _ => {
            return response(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                b"asset unavailable".to_vec(),
            )
        }
    };
    let media_type = match media_type_for(&target) {
        Some(media_type) => media_type,
        None => {
            return response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "text/plain; charset=utf-8",
                b"unsupported asset type".to_vec(),
            )
        }
    };
    let body = if request.method() == Method::HEAD {
        Vec::new()
    } else {
        match fs::read(&target) {
            Ok(bytes) => bytes,
            Err(_) => {
                return response(
                    StatusCode::NOT_FOUND,
                    "text/plain; charset=utf-8",
                    b"asset unavailable".to_vec(),
                )
            }
        }
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, media_type)
        .header(header::CONTENT_LENGTH, metadata.len().to_string())
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CONTENT_SECURITY_POLICY, ENGINE_CSP)
        .body(body)
        .expect("static engine asset response is valid")
}

fn parse_asset_request_path(path: &str) -> Option<(String, PathBuf)> {
    if path.contains('%') || path.contains('\\') || path.contains('\0') {
        return None;
    }
    let mut segments = path.trim_start_matches('/').split('/');
    let version = segments.next()?.to_string();
    if version.is_empty()
        || version.len() > 64
        || !version.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
    {
        return None;
    }
    let relative = segments.collect::<PathBuf>();
    if relative.as_os_str().is_empty()
        || relative.components().any(|component| {
            !matches!(component, Component::Normal(_))
                || component.as_os_str().to_str().is_none_or(|segment| {
                    segment.is_empty()
                        || !segment.chars().all(|character| {
                            character.is_ascii_alphanumeric()
                                || matches!(character, '.' | '-' | '_')
                        })
                })
        })
    {
        return None;
    }
    Some((version, relative))
}

fn resolve_read_only_asset(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let canonical_root =
        fs::canonicalize(root).map_err(|_| "engine asset root is unavailable".to_string())?;
    let mut candidate = canonical_root.clone();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            return Err("engine asset path is unsafe".into());
        };
        candidate.push(segment);
        if fs::symlink_metadata(&candidate)
            .map_err(|_| "engine asset is unavailable".to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("engine asset symlinks are forbidden".into());
        }
    }
    let canonical_target =
        fs::canonicalize(&candidate).map_err(|_| "engine asset is unavailable".to_string())?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err("engine asset escaped its package root".into());
    }
    Ok(canonical_target)
}

fn media_type_for(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => Some("text/html; charset=utf-8"),
        Some("js" | "mjs") => Some("text/javascript; charset=utf-8"),
        Some("css") => Some("text/css; charset=utf-8"),
        Some("json") => Some("application/json; charset=utf-8"),
        Some("wasm") => Some("application/wasm"),
        Some("bcmap" | "pfb") => Some("application/octet-stream"),
        Some("ttf") => Some("font/ttf"),
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("svg") => Some("image/svg+xml"),
        Some("md" | "txt") => Some("text/plain; charset=utf-8"),
        _ => None,
    }
}

fn response(status: StatusCode, media_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, media_type)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(body)
        .expect("static engine error response is valid")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn request_path_requires_a_version_and_safe_relative_asset() {
        assert_eq!(
            parse_asset_request_path("/1.0.0/assets/engine.mjs"),
            Some(("1.0.0".into(), PathBuf::from("assets/engine.mjs")))
        );
        for path in [
            "/1.0.0/../manifest.json",
            "/1.0.0/%2e%2e/manifest.json",
            "/1.0.0/assets\\engine.mjs",
            "/1.0.0/",
            "/bad version/index.html",
        ] {
            assert!(parse_asset_request_path(path).is_none(), "accepted {path}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn resolver_rejects_symlinks_even_when_they_point_inside_root() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "zero-engine-assets-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("index.html"), "ok").unwrap();
        symlink(root.join("index.html"), root.join("linked.html")).unwrap();
        assert!(resolve_read_only_asset(&root, Path::new("index.html")).is_ok());
        assert!(resolve_read_only_asset(&root, Path::new("linked.html")).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
