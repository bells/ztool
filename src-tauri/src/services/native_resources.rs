use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use reqwest::redirect::Policy;
use reqwest::{Client, Url};

use crate::plugins::contracts::{
    NativeResourceError, NetworkFetchRequest, NetworkFetchResponse, StorageWriteFileResult,
};

pub const DEFAULT_NETWORK_LIMIT_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_STORAGE_LIMIT_BYTES: usize = 25 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

pub async fn fetch_https(
    request: &NetworkFetchRequest,
    allowed_hosts: &[&str],
    max_bytes: usize,
) -> Result<NetworkFetchResponse, NativeResourceError> {
    if !request
        .method
        .as_deref()
        .unwrap_or("GET")
        .eq_ignore_ascii_case("GET")
    {
        return Err(resource_error(
            "network.fetch",
            "network.method_unsupported",
            "Only GET requests are supported.",
            false,
        ));
    }

    let allowed = allowed_hosts
        .iter()
        .map(|host| host.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let url = parse_allowed_https_url(&request.url, &allowed)?;
    let redirect_hosts = allowed.clone();
    let redirect_policy = Policy::custom(move |attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("redirect limit exceeded");
        }

        match validate_allowed_https_url(attempt.url(), &redirect_hosts) {
            Ok(()) => attempt.follow(),
            Err(_) => attempt.error("redirect target is not allowed"),
        }
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(redirect_policy)
        .build()
        .map_err(|error| {
            resource_error(
                "network.fetch",
                "network.client",
                format!("Failed to build network client: {error}"),
                true,
            )
        })?;

    let mut response = client.get(url).send().await.map_err(|error| {
        resource_error(
            "network.fetch",
            if error.is_timeout() {
                "network.timeout"
            } else {
                "network.request"
            },
            format!("Network request failed: {error}"),
            true,
        )
    })?;

    validate_allowed_https_url(response.url(), &allowed)?;

    validate_declared_response_size(response.content_length(), max_bytes)?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut body = Vec::new();

    while let Some(chunk) = response.chunk().await.map_err(|error| {
        resource_error(
            "network.fetch",
            "network.read",
            format!("Failed to read network response: {error}"),
            true,
        )
    })? {
        extend_bounded_response(&mut body, &chunk, max_bytes)?;
    }

    Ok(NetworkFetchResponse {
        status,
        content_type,
        body_base64: BASE64_STANDARD.encode(body),
    })
}

fn validate_declared_response_size(
    content_length: Option<u64>,
    max_bytes: usize,
) -> Result<(), NativeResourceError> {
    if content_length.is_some_and(|length| length > max_bytes as u64) {
        return Err(response_too_large_error());
    }
    Ok(())
}

fn extend_bounded_response(
    body: &mut Vec<u8>,
    chunk: &[u8],
    max_bytes: usize,
) -> Result<(), NativeResourceError> {
    if body.len().saturating_add(chunk.len()) > max_bytes {
        return Err(response_too_large_error());
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn response_too_large_error() -> NativeResourceError {
    resource_error(
        "network.fetch",
        "network.response_too_large",
        "Network response exceeds the configured limit.",
        false,
    )
}

pub fn write_plugin_file(
    root: &Path,
    relative_path: &str,
    bytes: &[u8],
    max_bytes: usize,
) -> Result<StorageWriteFileResult, NativeResourceError> {
    if bytes.len() > max_bytes {
        return Err(resource_error(
            "storage.writeFile",
            "storage.quota",
            "File content exceeds the configured write limit.",
            false,
        ));
    }

    let relative = validate_relative_path(relative_path)?;
    fs::create_dir_all(root).map_err(|error| storage_io_error("create data root", error))?;
    reject_symlink_components(root, &relative)?;

    let canonical_root = root
        .canonicalize()
        .map_err(|error| storage_io_error("resolve data root", error))?;
    let destination = root.join(&relative);
    let parent = destination.parent().ok_or_else(|| {
        resource_error(
            "storage.writeFile",
            "storage.path_invalid",
            "Destination has no parent directory.",
            false,
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| storage_io_error("create parent", error))?;

    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| storage_io_error("resolve parent", error))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(path_escape_error());
    }

    if destination.exists()
        && fs::symlink_metadata(&destination)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    {
        return Err(path_escape_error());
    }

    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            resource_error(
                "storage.writeFile",
                "storage.path_invalid",
                "Destination filename is invalid.",
                false,
            )
        })?;
    let temporary = canonical_parent.join(format!(".{file_name}.{}.part", std::process::id()));

    let result = (|| -> Result<(), NativeResourceError> {
        let mut file = File::create(&temporary)
            .map_err(|error| storage_io_error("create temporary file", error))?;
        file.write_all(bytes)
            .map_err(|error| storage_io_error("write temporary file", error))?;
        file.sync_all()
            .map_err(|error| storage_io_error("flush temporary file", error))?;

        replace_file(&temporary, &destination)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;

    Ok(StorageWriteFileResult {
        relative_path: relative_path.to_owned(),
        bytes_written: bytes.len(),
    })
}

#[cfg(not(target_os = "windows"))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), NativeResourceError> {
    fs::rename(temporary, destination)
        .map_err(|error| storage_io_error("atomically activate file", error))
}

#[cfg(target_os = "windows")]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), NativeResourceError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(storage_io_error(
            "atomically activate file",
            std::io::Error::last_os_error(),
        ));
    }
    Ok(())
}

pub fn resolve_plugin_file(
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, NativeResourceError> {
    let relative = validate_relative_path(relative_path)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| storage_io_error("resolve data root", error))?;
    let path = root.join(relative);
    let canonical_path = path
        .canonicalize()
        .map_err(|error| storage_io_error("resolve plugin file", error))?;

    if !canonical_path.starts_with(canonical_root) || !canonical_path.is_file() {
        return Err(path_escape_error());
    }

    Ok(canonical_path)
}

pub fn resource_error(
    operation: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
) -> NativeResourceError {
    NativeResourceError {
        operation: operation.into(),
        code: code.into(),
        message: message.into(),
        retryable,
    }
}

fn parse_allowed_https_url(
    value: &str,
    allowed_hosts: &HashSet<String>,
) -> Result<Url, NativeResourceError> {
    let url = Url::parse(value).map_err(|_| {
        resource_error(
            "network.fetch",
            "network.url_invalid",
            "Network URL is invalid.",
            false,
        )
    })?;
    validate_allowed_https_url(&url, allowed_hosts)?;
    Ok(url)
}

fn validate_allowed_https_url(
    url: &Url,
    allowed_hosts: &HashSet<String>,
) -> Result<(), NativeResourceError> {
    if url.scheme() != "https" {
        return Err(resource_error(
            "network.fetch",
            "network.scheme_denied",
            "Only HTTPS network requests are allowed.",
            false,
        ));
    }

    let host = url.host_str().ok_or_else(|| {
        resource_error(
            "network.fetch",
            "network.host_missing",
            "Network URL has no host.",
            false,
        )
    })?;
    let normalized_host = host.to_ascii_lowercase();

    if !allowed_hosts.contains(&normalized_host) {
        return Err(resource_error(
            "network.fetch",
            "network.host_denied",
            format!("Network host {host} is not allowed."),
            false,
        ));
    }

    if normalized_host == "localhost"
        || normalized_host.ends_with(".localhost")
        || host.parse::<IpAddr>().is_ok_and(is_private_ip)
    {
        return Err(resource_error(
            "network.fetch",
            "network.private_address_denied",
            "Loopback and private network destinations are not allowed.",
            false,
        ));
    }

    Ok(())
}

fn validate_relative_path(value: &str) -> Result<PathBuf, NativeResourceError> {
    if value.is_empty() || value.trim() != value || value.contains('\0') || value.contains('\\') {
        return Err(invalid_path_error());
    }

    let path = PathBuf::from(value);
    if path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(invalid_path_error());
    }

    Ok(path)
}

fn reject_symlink_components(root: &Path, relative: &Path) -> Result<(), NativeResourceError> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            if current.exists()
                && fs::symlink_metadata(&current)
                    .map(|metadata| metadata.file_type().is_symlink())
                    .unwrap_or(false)
            {
                return Err(path_escape_error());
            }
        }
    }
    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

fn invalid_path_error() -> NativeResourceError {
    resource_error(
        "storage.writeFile",
        "storage.path_invalid",
        "Path must be a normalized plugin-relative path.",
        false,
    )
}

fn path_escape_error() -> NativeResourceError {
    resource_error(
        "storage.writeFile",
        "storage.path_escape",
        "Path escapes the plugin data root.",
        false,
    )
}

fn storage_io_error(action: &str, error: std::io::Error) -> NativeResourceError {
    resource_error(
        "storage.writeFile",
        "storage.io",
        format!("Failed to {action}: {error}"),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{extend_bounded_response, validate_declared_response_size};

    #[test]
    fn network_size_policy_rejects_declared_and_streamed_oversize_responses() {
        let declared = validate_declared_response_size(Some(5), 4)
            .expect_err("declared oversized response should fail");
        assert_eq!(declared.code, "network.response_too_large");

        let mut body = vec![1, 2, 3];
        let streamed = extend_bounded_response(&mut body, &[4, 5], 4)
            .expect_err("streamed oversized response should fail");
        assert_eq!(streamed.code, "network.response_too_large");
        assert_eq!(body, vec![1, 2, 3]);
    }
}
