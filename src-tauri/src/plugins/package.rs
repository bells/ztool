use std::collections::HashSet;
use std::fs;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use super::contracts::{
    PluginFirstPartyEngine, PluginManifest, PluginPackageValidationReport, PluginPermission,
    PluginPlatform, PluginValidationIssue,
};

const ZERO_FILE_PLUGIN_ID: &str = "zero.file";
const ZERO_FILE_RELEASE_PUBLIC_KEY_BASE64: &str = "IEUnZngZj5k5vPRKkumGQ60Qs5hfQT8WAFGAD8V/ZGI=";
const MAX_PLUGIN_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PLUGIN_ENTRY_BYTES: u64 = 48 * 1024 * 1024;
const MAX_ENGINE_INSTALLED_BYTES: u64 = 45 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageDownloadRequest {
    pub download_url: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageDownload {
    pub staged_path: PathBuf,
    pub sha256: String,
    pub bytes_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPackageError {
    pub message: String,
}

pub fn download_package_with_fetcher<F, E>(
    request: &PluginPackageDownloadRequest,
    staging_dir: &Path,
    fetcher: F,
) -> Result<PluginPackageDownload, PluginPackageError>
where
    F: FnOnce(&str) -> Result<Vec<u8>, E>,
    E: std::fmt::Display,
{
    let file_name = zplugin_file_name(&request.download_url)?;
    let bytes = fetcher(&request.download_url)
        .map_err(|error| package_error(format!("failed to download package: {error}")))?;

    stage_package_bytes(staging_dir, file_name, &bytes, request.sha256.as_deref())
}

pub async fn download_package_to_staging(
    request: &PluginPackageDownloadRequest,
    staging_dir: &Path,
) -> Result<PluginPackageDownload, PluginPackageError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| package_error(format!("failed to create http client: {error}")))?;
    let response = client
        .get(&request.download_url)
        .send()
        .await
        .map_err(|error| package_error(format!("failed to download package: {error}")))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| package_error(format!("failed to read package bytes: {error}")))?;

    let file_name = zplugin_file_name(&request.download_url)?;
    stage_package_bytes(staging_dir, file_name, &bytes, request.sha256.as_deref())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub fn archive_entry_destination(
    install_root: &Path,
    entry_name: &str,
) -> Result<PathBuf, PluginPackageError> {
    let normalized_entry = normalize_archive_entry(entry_name)
        .ok_or_else(|| package_error("unsafe archive entry path"))?;

    if !is_safe_archive_entry(&normalized_entry) {
        return Err(package_error("unsafe archive entry path"));
    }

    let mut destination = install_root.to_path_buf();
    for segment in normalized_entry.split('/') {
        destination.push(segment);
    }

    Ok(destination)
}

pub fn validate_zplugin_package(
    package_path: &Path,
) -> Result<PluginPackageValidationReport, PluginPackageError> {
    let bytes = fs::read(package_path)
        .map_err(|error| package_error(format!("failed to read package: {error}")))?;
    let package_sha256 = sha256_hex(&bytes);
    let mut issues = Vec::new();

    if bytes.len() as u64 > MAX_PLUGIN_PACKAGE_BYTES {
        issues.push(validation_issue(
            "package.size.exceeded",
            "",
            "Plugin package exceeds the approved 64 MiB archive limit.",
        ));
    }

    if package_path.extension().and_then(|value| value.to_str()) != Some("zplugin") {
        issues.push(validation_issue(
            "package.extension",
            "",
            "Plugin package must use the .zplugin extension.",
        ));
    }

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(archive) => archive,
        Err(error) => {
            issues.push(validation_issue(
                "package.archive.invalid",
                "",
                format!("Package must be a readable ZIP archive: {error}"),
            ));
            return Ok(package_report(package_path, package_sha256, issues, None));
        }
    };

    let archive_entries = inspect_archive_entries(&mut archive, &mut issues)?;
    let mut manifest = read_manifest_from_archive(&mut archive, &mut issues)?;

    if let Some(manifest) = &mut manifest {
        issues.extend(validate_manifest(manifest));

        if is_safe_package_relative_path(&manifest.main)
            && !archive_entries.contains(&normalize_manifest_path(&manifest.main))
        {
            issues.push(validation_issue(
                "package.main.missing",
                "main",
                "Plugin package is missing the manifest-declared main entrypoint.",
            ));
        }

        normalize_manifest_engines(manifest);
        issues.extend(validate_first_party_engine_package(
            manifest,
            &archive_entries,
            &mut archive,
        )?);
    }

    Ok(package_report(
        package_path,
        package_sha256,
        issues,
        manifest,
    ))
}

pub fn extract_zplugin_package(
    package_path: &Path,
    destination_root: &Path,
) -> Result<PluginPackageValidationReport, PluginPackageError> {
    let report = validate_zplugin_package(package_path)?;
    if !report.valid {
        return Err(package_error(format_validation_issues(&report.issues)));
    }

    if destination_root.exists() {
        return Err(package_error("plugin install destination already exists"));
    }

    let file = fs::File::open(package_path)
        .map_err(|error| package_error(format!("failed to open package: {error}")))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| package_error(format!("failed to read package archive: {error}")))?;

    let result = extract_archive_entries(&mut archive, destination_root);
    if result.is_err() {
        let _ = fs::remove_dir_all(destination_root);
    }

    result?;
    Ok(report)
}

pub fn format_validation_issues(issues: &[PluginValidationIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("{}: {}", issue.code, issue.message))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn validate_installed_first_party_engine(
    version_root: &Path,
    activated_manifest: &PluginManifest,
) -> Result<(), String> {
    let key = zero_file_release_key()?;
    validate_installed_first_party_engine_with_key(version_root, activated_manifest, &key)
}

pub fn validate_approved_first_party_engine_package(
    plugin_id: &str,
    package_version: &str,
    package_sha256: &str,
) -> Result<(), String> {
    validate_engine_policy_approval(
        include_str!("../../file-engine-policy.json"),
        plugin_id,
        package_version,
        package_sha256,
    )
}

fn validate_engine_policy_approval(
    policy_json: &str,
    plugin_id: &str,
    package_version: &str,
    package_sha256: &str,
) -> Result<(), String> {
    let policy: serde_json::Value = serde_json::from_str(policy_json)
        .map_err(|_| "the embedded File engine release policy is invalid".to_string())?;
    let approved = policy
        .get("approvedEnginePackages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "the embedded File engine approval list is invalid".to_string())?;
    let matches_release = approved.iter().any(|entry| {
        entry.get("approved").and_then(serde_json::Value::as_bool) == Some(true)
            && entry.get("pluginId").and_then(serde_json::Value::as_str) == Some(plugin_id)
            && entry.get("version").and_then(serde_json::Value::as_str) == Some(package_version)
            && entry
                .get("packageSha256")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|digest| {
                    digest.len() == 64 && digest.eq_ignore_ascii_case(package_sha256)
                })
    });
    matches_release
        .then_some(())
        .ok_or_else(|| "the installed File engine package is not approved by release policy".into())
}

pub(crate) fn validate_installed_first_party_engine_with_key(
    version_root: &Path,
    activated_manifest: &PluginManifest,
    key: &[u8; 32],
) -> Result<(), String> {
    let manifest_path = version_root.join("manifest.json");
    let packaged_manifest: PluginManifest = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("installed engine manifest is unreadable: {error}"))?,
    )
    .map_err(|error| format!("installed engine manifest is invalid: {error}"))?;
    if packaged_manifest.name != activated_manifest.name
        || packaged_manifest.version != activated_manifest.version
        || packaged_manifest.author != activated_manifest.author
        || packaged_manifest.id != activated_manifest.id
        || packaged_manifest.permissions != activated_manifest.permissions
        || packaged_manifest.first_party_engine != activated_manifest.first_party_engine
    {
        return Err(
            "installed engine manifest does not match the activated registry record".into(),
        );
    }
    let engine = packaged_manifest
        .first_party_engine
        .as_ref()
        .ok_or_else(|| "installed package has no first-party engine metadata".to_string())?;
    verify_first_party_engine_signature_with_key(&packaged_manifest, engine, key)?;

    let engine_root = version_root.join("engine");
    let canonical_version_root = fs::canonicalize(version_root)
        .map_err(|_| "installed engine version root is missing".to_string())?;
    let canonical_engine_root = fs::canonicalize(&engine_root)
        .map_err(|_| "installed engine asset root is missing".to_string())?;
    let installed_files = collect_installed_engine_files(&canonical_engine_root)?;
    let declared = engine
        .assets
        .iter()
        .map(|asset| asset.path.as_str())
        .collect::<HashSet<_>>();
    let installed = installed_files
        .iter()
        .map(|path| {
            path.strip_prefix(&canonical_version_root)
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                .map_err(|_| "installed engine asset escaped its version root".to_string())
        })
        .collect::<Result<HashSet<_>, _>>()?;
    if declared.len() != engine.assets.len()
        || installed.len() != engine.assets.len()
        || !installed
            .iter()
            .all(|path| declared.contains(path.as_str()))
    {
        return Err("installed engine asset inventory does not match the signed manifest".into());
    }

    let mut total_bytes = 0_u64;
    for asset in &engine.assets {
        if !is_safe_package_relative_path(&asset.path)
            || !asset.path.starts_with("engine/")
            || engine_media_type(&asset.path) != Some(asset.media_type.as_str())
        {
            return Err(format!(
                "installed engine asset metadata is invalid: {}",
                asset.path
            ));
        }
        let target = version_root.join(&asset.path);
        let canonical_target = fs::canonicalize(&target)
            .map_err(|_| format!("installed engine asset is missing: {}", asset.path))?;
        if !canonical_target.starts_with(&canonical_engine_root) {
            return Err(format!(
                "installed engine asset escaped its root: {}",
                asset.path
            ));
        }
        let bytes = fs::read(&canonical_target)
            .map_err(|_| format!("installed engine asset is unreadable: {}", asset.path))?;
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        if bytes.len() as u64 != asset.bytes
            || !asset.sha256.eq_ignore_ascii_case(&sha256_hex(&bytes))
        {
            return Err(format!(
                "installed engine asset failed integrity verification: {}",
                asset.path
            ));
        }
    }
    if total_bytes > MAX_ENGINE_INSTALLED_BYTES {
        return Err("installed engine assets exceed the approved size limit".into());
    }
    if engine.notices.iter().any(|notice| {
        !notice.starts_with("engine/licenses/")
            || !declared.contains(notice.as_str())
            || !installed.contains(notice)
    }) {
        return Err("installed engine notices do not match the signed manifest".into());
    }
    Ok(())
}

fn collect_installed_engine_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)
        .map_err(|error| format!("installed engine directory is unreadable: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("installed engine entry is unreadable: {error}"))?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| format!("installed engine metadata is unreadable: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("installed engine assets must not contain symlinks".into());
        }
        if metadata.is_dir() {
            files.extend(collect_installed_engine_files(&entry.path())?);
        } else if metadata.is_file() {
            files.push(entry.path());
        } else {
            return Err("installed engine assets contain an unsupported file type".into());
        }
    }
    Ok(files)
}

fn stage_package_bytes(
    staging_dir: &Path,
    file_name: &str,
    bytes: &[u8],
    expected_sha256: Option<&str>,
) -> Result<PluginPackageDownload, PluginPackageError> {
    fs::create_dir_all(staging_dir)
        .map_err(|error| package_error(format!("failed to create staging directory: {error}")))?;

    let staged_path = staging_dir.join(file_name);
    let actual_sha256 = sha256_hex(bytes);

    let result = fs::write(&staged_path, bytes)
        .map_err(|error| package_error(format!("failed to write staged package: {error}")))
        .and_then(|_| {
            if let Some(expected) =
                expected_sha256.filter(|expected| !expected.eq_ignore_ascii_case(&actual_sha256))
            {
                return Err(package_error(format!(
                    "package checksum mismatch: expected {expected}, got {actual_sha256}"
                )));
            }

            Ok(PluginPackageDownload {
                staged_path: staged_path.clone(),
                sha256: actual_sha256,
                bytes_len: bytes.len() as u64,
            })
        });

    if result.is_err() {
        let _ = fs::remove_dir_all(staging_dir);
    }

    result
}

fn is_safe_archive_entry(entry_name: &str) -> bool {
    if entry_name.is_empty() || entry_name.contains('\0') {
        return false;
    }

    if entry_name.starts_with('/')
        || entry_name.starts_with('\\')
        || entry_name.contains('\\')
        || entry_name.contains(':')
    {
        return false;
    }

    entry_name
        .split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn inspect_archive_entries<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    issues: &mut Vec<PluginValidationIssue>,
) -> Result<HashSet<String>, PluginPackageError> {
    let mut entries = HashSet::new();

    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|error| package_error(format!("failed to inspect archive entry: {error}")))?;
        let raw_name = file.name().to_string();

        let Some(normalized_name) = normalize_archive_entry(&raw_name) else {
            issues.push(validation_issue(
                "package.archive.unsafePath",
                raw_name,
                "Package archive contains an unsafe entry path.",
            ));
            continue;
        };

        if !is_safe_archive_entry(&normalized_name) {
            issues.push(validation_issue(
                "package.archive.unsafePath",
                raw_name,
                "Package archive contains an unsafe entry path.",
            ));
            continue;
        }

        if is_zip_symlink(file.unix_mode()) {
            issues.push(validation_issue(
                "package.archive.symlink",
                raw_name,
                "Package archive must not contain symlinks.",
            ));
            continue;
        }

        if file.size() > MAX_PLUGIN_ENTRY_BYTES {
            issues.push(validation_issue(
                "package.archive.entryOversized",
                raw_name,
                "Package entry exceeds the approved 48 MiB extracted-file limit.",
            ));
            continue;
        }

        if !file.is_dir() && !entries.insert(normalized_name) {
            issues.push(validation_issue(
                "package.archive.duplicate",
                raw_name,
                "Package archive contains a duplicate entry.",
            ));
        }
    }

    Ok(entries)
}

fn read_manifest_from_archive<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    issues: &mut Vec<PluginValidationIssue>,
) -> Result<Option<PluginManifest>, PluginPackageError> {
    let mut manifest_file = match archive.by_name("manifest.json") {
        Ok(file) => file,
        Err(_) => {
            issues.push(validation_issue(
                "package.manifest.missing",
                "manifest.json",
                "Plugin package must contain a root manifest.json.",
            ));
            return Ok(None);
        }
    };

    let mut manifest_json = String::new();
    manifest_file
        .read_to_string(&mut manifest_json)
        .map_err(|error| package_error(format!("failed to read manifest.json: {error}")))?;

    match serde_json::from_str::<PluginManifest>(&manifest_json) {
        Ok(manifest) => Ok(Some(manifest)),
        Err(error) => {
            issues.push(validation_issue(
                "package.manifest.invalid",
                "manifest.json",
                format!("Plugin manifest is invalid: {error}"),
            ));
            Ok(None)
        }
    }
}

fn extract_archive_entries<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    destination_root: &Path,
) -> Result<(), PluginPackageError> {
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| package_error(format!("failed to read archive entry: {error}")))?;

        if is_zip_symlink(file.unix_mode()) {
            return Err(package_error("package archive contains an unsafe symlink"));
        }

        let destination = archive_entry_destination(destination_root, file.name())?;

        if file.is_dir() {
            fs::create_dir_all(&destination).map_err(|error| {
                package_error(format!("failed to create plugin directory: {error}"))
            })?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                package_error(format!("failed to create plugin directory: {error}"))
            })?;
        }

        let mut output = fs::File::create(&destination)
            .map_err(|error| package_error(format!("failed to create plugin file: {error}")))?;
        std::io::copy(&mut file, &mut output)
            .map_err(|error| package_error(format!("failed to extract plugin file: {error}")))?;
    }

    Ok(())
}

fn validate_manifest(manifest: &PluginManifest) -> Vec<PluginValidationIssue> {
    let mut issues = Vec::new();

    if !is_valid_plugin_name(&manifest.name) {
        issues.push(validation_issue(
            "manifest.name.invalid",
            "name",
            "Plugin name must use lowercase letters, numbers, dots, underscores, or dashes.",
        ));
    }

    if !is_semver(&manifest.version) {
        issues.push(validation_issue(
            "manifest.version.invalid",
            "version",
            "Plugin version must be a semantic version.",
        ));
    }

    if !is_safe_package_relative_path(&manifest.main) {
        issues.push(validation_issue(
            "manifest.main.unsafe",
            "main",
            "Plugin main must be a safe package-relative path.",
        ));
    }

    if let Some(engines) = &manifest.engines {
        if engines.api.as_deref().is_some_and(|api| api != "1") {
            issues.push(validation_issue(
                "manifest.api.incompatible",
                "engines.api",
                "Plugin targets an unsupported Extension API version.",
            ));
        }

        let (host_range, host_path, issue_code) = if let Some(zero) = engines.zero.as_deref() {
            (Some(zero), "engines.zero", "manifest.zero.incompatible")
        } else {
            (
                engines.ztool.as_deref(),
                "engines.ztool",
                "manifest.ztool.incompatible",
            )
        };

        if host_range.is_some_and(|range| !is_compatible_zero_host_range(range)) {
            issues.push(validation_issue(
                issue_code,
                host_path,
                "Plugin targets an unsupported Zero host version.",
            ));
        }
    }

    if manifest
        .platforms
        .as_ref()
        .is_some_and(|platforms| !platforms.contains(&current_plugin_platform()))
    {
        issues.push(validation_issue(
            "manifest.platform.incompatible",
            "platforms",
            "Plugin package does not support the current platform.",
        ));
    }

    let requests_document_conversion = manifest
        .permissions
        .contains(&PluginPermission::DocumentConvert);
    if requests_document_conversion
        && (manifest.name != ZERO_FILE_PLUGIN_ID
            || manifest.id.as_deref() != Some(ZERO_FILE_PLUGIN_ID)
            || manifest.author != "bells"
            || manifest.first_party_engine.is_none())
    {
        issues.push(validation_issue(
            "manifest.documentConvert.firstPartyOnly",
            "permissions",
            "document.convert is reserved for the signed first-party Zero File package.",
        ));
    }
    if manifest.first_party_engine.is_some() && !requests_document_conversion {
        issues.push(validation_issue(
            "manifest.firstPartyEngine.permissionMissing",
            "firstPartyEngine",
            "First-party engine metadata requires document.convert.",
        ));
    }

    issues
}

fn validate_first_party_engine_package<R: Read + Seek>(
    manifest: &PluginManifest,
    archive_entries: &HashSet<String>,
    archive: &mut ZipArchive<R>,
) -> Result<Vec<PluginValidationIssue>, PluginPackageError> {
    let Some(engine) = &manifest.first_party_engine else {
        return Ok(Vec::new());
    };
    let mut issues = Vec::new();
    if engine.protocol_version != 1 {
        issues.push(validation_issue(
            "engine.protocol.incompatible",
            "firstPartyEngine.protocolVersion",
            "The engine protocol version is not supported by this Zero host.",
        ));
    }
    if engine.package_version != manifest.version
        || !is_compatible_zero_host_range(&engine.host_api_range)
    {
        issues.push(validation_issue(
            "engine.host.incompatible",
            "firstPartyEngine",
            "The engine package version or Zero host API range is incompatible.",
        ));
    }
    if engine.directions.is_empty() || engine.assets.is_empty() || engine.notices.is_empty() {
        issues.push(validation_issue(
            "engine.manifest.incomplete",
            "firstPartyEngine",
            "The first-party engine must declare directions, assets, and notices.",
        ));
    }

    let mut declared_paths = HashSet::new();
    let mut installed_bytes = 0_u64;
    for asset in &engine.assets {
        if !is_safe_package_relative_path(&asset.path)
            || !asset.path.starts_with("engine/")
            || !declared_paths.insert(normalize_manifest_path(&asset.path))
        {
            issues.push(validation_issue(
                "engine.asset.pathInvalid",
                &asset.path,
                "Engine asset paths must be unique, safe, and remain under engine/.",
            ));
            continue;
        }
        if asset.sha256.len() != 64
            || !asset
                .sha256
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            issues.push(validation_issue(
                "engine.asset.digestInvalid",
                &asset.path,
                "Engine asset SHA-256 must be a 64-character hexadecimal digest.",
            ));
            continue;
        }
        if asset.bytes == 0 || asset.bytes > MAX_PLUGIN_ENTRY_BYTES {
            issues.push(validation_issue(
                "engine.asset.sizeInvalid",
                &asset.path,
                "Engine asset size is outside the approved limit.",
            ));
            continue;
        }
        installed_bytes = installed_bytes.saturating_add(asset.bytes);
        if engine_media_type(&asset.path) != Some(asset.media_type.as_str()) {
            issues.push(validation_issue(
                "engine.asset.mediaTypeInvalid",
                &asset.path,
                "Engine asset mediaType must match the host allowlist for its extension.",
            ));
        }
        if !archive_entries.contains(&asset.path) {
            issues.push(validation_issue(
                "engine.asset.missing",
                &asset.path,
                "A manifest-declared engine asset is missing from the package.",
            ));
            continue;
        }
        let mut file = archive.by_name(&asset.path).map_err(|error| {
            package_error(format!(
                "failed to read engine asset {}: {error}",
                asset.path
            ))
        })?;
        if file.size() != asset.bytes {
            issues.push(validation_issue(
                "engine.asset.sizeMismatch",
                &asset.path,
                "Engine asset size does not match the signed manifest.",
            ));
            continue;
        }
        let mut bytes = Vec::with_capacity(asset.bytes as usize);
        file.read_to_end(&mut bytes).map_err(|error| {
            package_error(format!(
                "failed to hash engine asset {}: {error}",
                asset.path
            ))
        })?;
        if !asset.sha256.eq_ignore_ascii_case(&sha256_hex(&bytes)) {
            issues.push(validation_issue(
                "engine.asset.digestMismatch",
                &asset.path,
                "Engine asset digest does not match the signed manifest.",
            ));
        }
    }
    if installed_bytes > MAX_ENGINE_INSTALLED_BYTES {
        issues.push(validation_issue(
            "engine.assets.oversized",
            "firstPartyEngine.assets",
            "Engine assets exceed the approved 45 MiB installed limit.",
        ));
    }
    for entry in archive_entries
        .iter()
        .filter(|entry| entry.starts_with("engine/"))
    {
        if !declared_paths.contains(entry) {
            issues.push(validation_issue(
                "engine.asset.undeclared",
                entry,
                "The package contains an undeclared engine asset.",
            ));
        }
    }
    for notice in &engine.notices {
        if !is_safe_package_relative_path(notice)
            || !notice.starts_with("engine/licenses/")
            || !archive_entries.contains(notice)
            || !declared_paths.contains(notice)
        {
            issues.push(validation_issue(
                "engine.notice.invalid",
                notice,
                "Every engine notice must be a packaged file under engine/licenses/.",
            ));
        }
    }
    if let Err(message) = verify_first_party_engine_signature(manifest, engine) {
        issues.push(validation_issue(
            "engine.signature.invalid",
            "firstPartyEngine.signature",
            message,
        ));
    }
    Ok(issues)
}

fn engine_media_type(path: &str) -> Option<&'static str> {
    if Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("LICENSE") || name.ends_with("-LICENSE"))
    {
        return Some("text/plain");
    }
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("html") => Some("text/html"),
        Some("js" | "mjs") => Some("text/javascript"),
        Some("css") => Some("text/css"),
        Some("json") => Some("application/json"),
        Some("wasm") => Some("application/wasm"),
        Some("bcmap" | "pfb") => Some("application/octet-stream"),
        Some("ttf") => Some("font/ttf"),
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("svg") => Some("image/svg+xml"),
        Some("md") => Some("text/markdown"),
        Some("txt") => Some("text/plain"),
        _ => None,
    }
}

fn verify_first_party_engine_signature(
    manifest: &PluginManifest,
    engine: &PluginFirstPartyEngine,
) -> Result<(), String> {
    let key = zero_file_release_key()?;
    verify_first_party_engine_signature_with_key(manifest, engine, &key)
}

fn zero_file_release_key() -> Result<[u8; 32], String> {
    let key_bytes = BASE64_STANDARD
        .decode(ZERO_FILE_RELEASE_PUBLIC_KEY_BASE64)
        .map_err(|_| "The pinned Zero File verification key is invalid.".to_string())?;
    key_bytes
        .try_into()
        .map_err(|_| "The pinned Zero File verification key has the wrong length.".to_string())
}

fn verify_first_party_engine_signature_with_key(
    manifest: &PluginManifest,
    engine: &PluginFirstPartyEngine,
    key: &[u8; 32],
) -> Result<(), String> {
    let verifying_key = VerifyingKey::from_bytes(key)
        .map_err(|_| "The pinned Zero File verification key could not be loaded.".to_string())?;
    let signature_bytes = BASE64_STANDARD
        .decode(&engine.signature)
        .map_err(|_| "The detached engine manifest signature is not valid base64.".to_string())?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| "The detached engine manifest signature has the wrong length.".to_string())?;
    verifying_key
        .verify(
            &first_party_engine_signature_payload(manifest, engine),
            &signature,
        )
        .map_err(|_| "The detached engine manifest signature did not verify.".to_string())
}

pub(crate) fn first_party_engine_signature_payload(
    manifest: &PluginManifest,
    engine: &PluginFirstPartyEngine,
) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema": "zero-file-engine-signature-v1",
        "pluginName": manifest.name,
        "pluginVersion": manifest.version,
        "protocolVersion": engine.protocol_version,
        "packageVersion": engine.package_version,
        "hostApiRange": engine.host_api_range,
        "directions": engine.directions,
        "platformMinimums": engine.platform_minimums,
        "assets": engine.assets,
        "notices": engine.notices,
    }))
    .expect("first-party engine signature payload is serializable")
}

fn normalize_archive_entry(entry_name: &str) -> Option<String> {
    let normalized = entry_name.trim_end_matches('/');
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn normalize_manifest_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn is_safe_package_relative_path(value: &str) -> bool {
    if value.is_empty() || value.trim() != value || value.contains('\0') {
        return false;
    }

    if value.starts_with('/')
        || value.starts_with('\\')
        || value.contains(':')
        || value.contains('\\')
    {
        return false;
    }

    value
        .split('/')
        .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn is_valid_plugin_name(value: &str) -> bool {
    let length_ok = (2..=64).contains(&value.len());
    length_ok
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '.'
                || character == '_'
                || character == '-'
        })
}

fn is_semver(value: &str) -> bool {
    let core = value
        .split_once('-')
        .map(|(core, _)| core)
        .unwrap_or(value)
        .split_once('+')
        .map(|(core, _)| core)
        .unwrap_or(value);
    let parts = core.split('.').collect::<Vec<_>>();

    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
        })
}

fn normalize_manifest_engines(manifest: &mut PluginManifest) {
    if let Some(engines) = &mut manifest.engines {
        if engines.zero.is_none() {
            engines.zero = engines.ztool.take();
        } else {
            engines.ztool = None;
        }
    }
}

fn is_compatible_zero_host_range(value: &str) -> bool {
    value == "*"
        || value == env!("CARGO_PKG_VERSION")
        || value == format!("^{}", env!("CARGO_PKG_VERSION"))
        || value == format!(">={}", env!("CARGO_PKG_VERSION"))
}

fn current_plugin_platform() -> PluginPlatform {
    #[cfg(target_os = "macos")]
    return PluginPlatform::Macos;
    #[cfg(target_os = "windows")]
    return PluginPlatform::Windows;
    #[cfg(target_os = "linux")]
    return PluginPlatform::Linux;
}

fn is_zip_symlink(mode: Option<u32>) -> bool {
    mode.is_some_and(|mode| mode & 0o170000 == 0o120000)
}

fn package_report(
    package_path: &Path,
    sha256: String,
    issues: Vec<PluginValidationIssue>,
    manifest: Option<PluginManifest>,
) -> PluginPackageValidationReport {
    PluginPackageValidationReport {
        valid: issues.is_empty(),
        issues,
        manifest,
        package_path: package_path.to_string_lossy().into_owned(),
        sha256,
    }
}

fn validation_issue(
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> PluginValidationIssue {
    PluginValidationIssue {
        code: code.into(),
        path: path.into(),
        message: message.into(),
    }
}

fn zplugin_file_name(download_url: &str) -> Result<&str, PluginPackageError> {
    let file_name = download_url
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| package_error("download URL must include a .zplugin file name"))?;

    if !file_name.ends_with(".zplugin") {
        return Err(package_error("download URL must point to a .zplugin asset"));
    }

    Ok(file_name)
}

fn package_error(message: impl Into<String>) -> PluginPackageError {
    PluginPackageError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use ed25519_dalek::{Signer, SigningKey};

    use super::*;
    use crate::plugins::contracts::{
        PluginDocumentDirection, PluginEngineAsset, PluginEnginePlatformMinimum, PluginPlatform,
        PluginRuntime,
    };

    fn signed_manifest(signing_key: &SigningKey) -> PluginManifest {
        let mut manifest = PluginManifest {
            name: ZERO_FILE_PLUGIN_ID.into(),
            version: "1.0.0".into(),
            author: "bells".into(),
            main: "engine/index.html".into(),
            permissions: vec![PluginPermission::DocumentConvert],
            id: Some(ZERO_FILE_PLUGIN_ID.into()),
            display_name: Some("Zero File".into()),
            description: None,
            engines: None,
            platforms: Some(vec![PluginPlatform::Macos]),
            runtime: Some(PluginRuntime::Webview),
            contributes: None,
            first_party_engine: Some(PluginFirstPartyEngine {
                protocol_version: 1,
                package_version: "1.0.0".into(),
                host_api_range: ">=0.1.0".into(),
                directions: vec![PluginDocumentDirection::PdfToDocx],
                platform_minimums: vec![PluginEnginePlatformMinimum {
                    platform: PluginPlatform::Macos,
                    version: "11.0".into(),
                }],
                assets: vec![PluginEngineAsset {
                    path: "engine/index.html".into(),
                    sha256: "00".repeat(32),
                    bytes: 1,
                    media_type: "text/html".into(),
                }],
                notices: vec!["engine/licenses/NOTICE".into()],
                signature: String::new(),
            }),
        };
        let payload = first_party_engine_signature_payload(
            &manifest,
            manifest.first_party_engine.as_ref().unwrap(),
        );
        manifest.first_party_engine.as_mut().unwrap().signature =
            BASE64_STANDARD.encode(signing_key.sign(&payload).to_bytes());
        manifest
    }

    #[test]
    fn detached_signature_accepts_only_the_matching_test_key_and_payload() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let manifest = signed_manifest(&signing_key);
        let engine = manifest.first_party_engine.as_ref().unwrap();

        assert!(verify_first_party_engine_signature_with_key(
            &manifest,
            engine,
            signing_key.verifying_key().as_bytes(),
        )
        .is_ok());
        assert!(verify_first_party_engine_signature_with_key(&manifest, engine, &[8; 32]).is_err());

        let mut changed = manifest.clone();
        changed.version = "1.0.1".into();
        assert!(verify_first_party_engine_signature_with_key(
            &changed,
            changed.first_party_engine.as_ref().unwrap(),
            signing_key.verifying_key().as_bytes(),
        )
        .is_err());
    }

    #[test]
    fn installed_engine_revalidates_signed_inventory_and_detects_tampering() {
        let signing_key = SigningKey::from_bytes(&[11; 32]);
        let version_root = std::env::temp_dir().join(format!(
            "zero-installed-engine-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let index = b"<!doctype html><title>engine</title>";
        let notice = b"test notice";
        fs::create_dir_all(version_root.join("engine/licenses")).unwrap();
        fs::write(version_root.join("engine/index.html"), index).unwrap();
        fs::write(version_root.join("engine/licenses/NOTICE.txt"), notice).unwrap();

        let mut manifest = signed_manifest(&signing_key);
        manifest.first_party_engine.as_mut().unwrap().assets = vec![
            PluginEngineAsset {
                path: "engine/index.html".into(),
                sha256: sha256_hex(index),
                bytes: index.len() as u64,
                media_type: "text/html".into(),
            },
            PluginEngineAsset {
                path: "engine/licenses/NOTICE.txt".into(),
                sha256: sha256_hex(notice),
                bytes: notice.len() as u64,
                media_type: "text/plain".into(),
            },
        ];
        manifest.first_party_engine.as_mut().unwrap().notices =
            vec!["engine/licenses/NOTICE.txt".into()];
        let payload = first_party_engine_signature_payload(
            &manifest,
            manifest.first_party_engine.as_ref().unwrap(),
        );
        manifest.first_party_engine.as_mut().unwrap().signature =
            BASE64_STANDARD.encode(signing_key.sign(&payload).to_bytes());
        fs::write(
            version_root.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let validation = validate_installed_first_party_engine_with_key(
            &version_root,
            &manifest,
            signing_key.verifying_key().as_bytes(),
        );
        assert!(validation.is_ok(), "{validation:?}");
        fs::write(version_root.join("engine/index.html"), "tampered").unwrap();
        assert!(validate_installed_first_party_engine_with_key(
            &version_root,
            &manifest,
            signing_key.verifying_key().as_bytes(),
        )
        .is_err());
        fs::remove_dir_all(version_root).unwrap();
    }

    #[test]
    fn release_policy_accepts_only_the_exact_approved_package_digest() {
        let digest = "ab".repeat(32);
        let policy = serde_json::json!({
            "approvedEnginePackages": [{
                "pluginId": "zero.file",
                "version": "1.0.0",
                "approved": true,
                "packageSha256": digest,
            }]
        })
        .to_string();
        assert!(
            validate_engine_policy_approval(&policy, "zero.file", "1.0.0", &"ab".repeat(32),)
                .is_ok()
        );
        assert!(
            validate_engine_policy_approval(&policy, "zero.file", "1.0.0", &"cd".repeat(32),)
                .is_err()
        );
        assert!(validate_approved_first_party_engine_package(
            "zero.file",
            "1.0.0",
            &"ab".repeat(32),
        )
        .is_err());
    }
}
