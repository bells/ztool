use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brand::{
    canonical_data_root, canonical_first_party_contribution_id, default_home, legacy_data_root,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub copied_entries: usize,
    pub normalized_files: usize,
    pub completed_fast_path: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationCompletion {
    schema_version: u16,
}

const MIGRATION_SCHEMA_VERSION: u16 = 1;
const MIGRATION_MARKER_NAME: &str = ".legacy-migration.json";

pub fn migrate_default_home() -> MigrationReport {
    migrate_legacy_data(&default_home())
}

pub fn migrate_legacy_data(home: &Path) -> MigrationReport {
    let legacy_root = legacy_data_root(home);
    let canonical_root = canonical_data_root(home);
    let mut report = MigrationReport::default();

    if !legacy_root.exists() {
        report.completed_fast_path = true;
        return report;
    }

    let marker_path = canonical_root.join(MIGRATION_MARKER_NAME);
    if migration_is_complete(&marker_path) {
        report.completed_fast_path = true;
        return report;
    }

    if let Err(error) = fs::create_dir_all(&canonical_root) {
        report.diagnostics.push(format!(
            "failed to create Zero data root {}: {error}",
            canonical_root.display()
        ));
        return report;
    }

    copy_missing_tree(&legacy_root, &canonical_root, &mut report);
    normalize_registry_file(
        &canonical_root.join("plugins").join("registry.json"),
        &legacy_root,
        &canonical_root,
        &mut report,
    );

    if report.diagnostics.is_empty() {
        if let Err(error) = write_migration_marker(&marker_path) {
            report.diagnostics.push(error);
        }
    }

    report
}

fn migration_is_complete(marker_path: &Path) -> bool {
    fs::read(marker_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<MigrationCompletion>(&bytes).ok())
        .is_some_and(|marker| marker.schema_version == MIGRATION_SCHEMA_VERSION)
}

fn write_migration_marker(marker_path: &Path) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(&MigrationCompletion {
        schema_version: MIGRATION_SCHEMA_VERSION,
    })
    .map_err(|error| format!("failed to serialize Zero migration marker: {error}"))?;
    replace_file_atomic(marker_path, &bytes)
}

fn copy_missing_tree(source: &Path, destination: &Path, report: &mut MigrationReport) {
    let entries = match fs::read_dir(source) {
        Ok(entries) => entries,
        Err(error) => {
            report.diagnostics.push(format!(
                "failed to read legacy data directory {}: {error}",
                source.display()
            ));
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report
                    .diagnostics
                    .push(format!("failed to inspect legacy data entry: {error}"));
                continue;
            }
        };
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                report.diagnostics.push(format!(
                    "failed to inspect legacy data type {}: {error}",
                    source_path.display()
                ));
                continue;
            }
        };

        if file_type.is_symlink() {
            report.diagnostics.push(format!(
                "skipped legacy symlink during migration: {}",
                source_path.display()
            ));
            continue;
        }

        if file_type.is_dir() {
            if let Err(error) = fs::create_dir_all(&destination_path) {
                report.diagnostics.push(format!(
                    "failed to create Zero data directory {}: {error}",
                    destination_path.display()
                ));
                continue;
            }
            copy_missing_tree(&source_path, &destination_path, report);
            continue;
        }

        if !file_type.is_file() || destination_path.exists() {
            continue;
        }

        match copy_file_atomic(&source_path, &destination_path) {
            Ok(()) => report.copied_entries += 1,
            Err(error) => report.diagnostics.push(error),
        }
    }
}

fn copy_file_atomic(source: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source)
        .map_err(|error| format!("failed to read legacy file {}: {error}", source.display()))?;
    write_atomic_if_missing(destination, &bytes)
}

fn write_atomic_if_missing(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create Zero data directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let temporary = temporary_path(destination);
    fs::write(&temporary, bytes).map_err(|error| {
        format!(
            "failed to stage Zero migration file {}: {error}",
            temporary.display()
        )
    })?;

    match fs::rename(&temporary, destination) {
        Ok(()) => Ok(()),
        Err(_) if destination.exists() => {
            let _ = fs::remove_file(&temporary);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(format!(
                "failed to activate Zero migration file {}: {error}",
                destination.display()
            ))
        }
    }
}

fn normalize_registry_file(
    path: &Path,
    legacy_root: &Path,
    canonical_root: &Path,
    report: &mut MigrationReport,
) {
    if !path.exists() {
        return;
    }

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            report.diagnostics.push(format!(
                "failed to read migrated registry {}: {error}",
                path.display()
            ));
            return;
        }
    };
    let mut value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => value,
        Err(error) => {
            report.diagnostics.push(format!(
                "failed to parse migrated registry {}: {error}",
                path.display()
            ));
            return;
        }
    };

    if !normalize_json_value(&mut value, legacy_root, canonical_root) {
        return;
    }

    let normalized = match serde_json::to_vec_pretty(&value) {
        Ok(normalized) => normalized,
        Err(error) => {
            report
                .diagnostics
                .push(format!("failed to serialize migrated registry: {error}"));
            return;
        }
    };

    if let Err(error) = replace_file_atomic(path, &normalized) {
        report.diagnostics.push(error);
        return;
    }
    report.normalized_files += 1;
}

fn normalize_json_value(value: &mut Value, legacy_root: &Path, canonical_root: &Path) -> bool {
    match value {
        Value::Array(values) => {
            let mut changed = false;
            for value in values {
                changed |= normalize_json_value(value, legacy_root, canonical_root);
            }
            changed
        }
        Value::Object(values) => {
            let mut changed = false;
            for value in values.values_mut() {
                changed |= normalize_json_value(value, legacy_root, canonical_root);
            }
            changed
        }
        Value::String(text) => {
            let canonical_id = canonical_first_party_contribution_id(text);
            if canonical_id != *text {
                *text = canonical_id;
                return true;
            }

            if let Ok(relative) = Path::new(text).strip_prefix(legacy_root) {
                let canonical_path = canonical_root.join(relative);
                if canonical_path.exists() {
                    *text = canonical_path.to_string_lossy().into_owned();
                    return true;
                }
            }

            false
        }
        _ => false,
    }
}

fn replace_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = temporary_path(path);
    fs::write(&temporary, bytes).map_err(|error| {
        format!(
            "failed to stage normalized migration file {}: {error}",
            temporary.display()
        )
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!(
            "failed to activate normalized migration file {}: {error}",
            path.display()
        )
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("zero-data");
    path.with_file_name(format!(".{name}.{nonce}.zero-migration.part"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::Value;

    use super::{migrate_legacy_data, MIGRATION_MARKER_NAME};

    fn temp_home(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("zero-migration-{label}-{nonce}"))
    }

    #[test]
    fn copies_and_normalizes_legacy_registry_without_deleting_source() {
        let home = temp_home("registry");
        let legacy_plugins = home.join(".ztool/plugins");
        fs::create_dir_all(legacy_plugins.join("demo/1.0.0")).unwrap();
        fs::write(
            legacy_plugins.join("demo/1.0.0/index.html"),
            "<main>demo</main>",
        )
        .unwrap();
        let legacy_registry = legacy_plugins.join("registry.json");
        fs::write(
            &legacy_registry,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": 3,
                "unknown": {"kept": true},
                "records": [{
                    "name": "ztool.screenshot",
                    "installedPath": home.join(".ztool/plugins/demo/1.0.0"),
                    "manifest": {
                        "name": "ztool.screenshot",
                        "contributes": {
                            "commands": [{"id": "ztool.screenshot.capture"}]
                        }
                    }
                }, {
                    "name": "ztool.third-party"
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let report = migrate_legacy_data(&home);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(report.normalized_files, 1);
        assert!(legacy_registry.exists());

        let canonical_registry = home.join(".zero/plugins/registry.json");
        let value: Value = serde_json::from_slice(&fs::read(canonical_registry).unwrap()).unwrap();
        assert_eq!(value["unknown"]["kept"], true);
        assert_eq!(value["records"][0]["name"], "zero.snap");
        assert_eq!(
            value["records"][0]["manifest"]["contributes"]["commands"][0]["id"],
            "zero.snap.capture"
        );
        assert_eq!(value["records"][1]["name"], "ztool.third-party");
        assert!(value["records"][0]["installedPath"]
            .as_str()
            .unwrap()
            .contains("/.zero/plugins/"));

        let second = migrate_legacy_data(&home);
        assert!(second.completed_fast_path);
        assert_eq!(second.copied_entries, 0);
        assert_eq!(second.normalized_files, 0);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn corrupt_or_old_completion_marker_reruns_migration() {
        let home = temp_home("marker-recovery");
        fs::create_dir_all(home.join(".ztool/data")).unwrap();
        fs::create_dir_all(home.join(".zero")).unwrap();
        fs::write(home.join(".ztool/data/example.txt"), "legacy").unwrap();
        fs::write(
            home.join(".zero").join(MIGRATION_MARKER_NAME),
            r#"{"schemaVersion":0}"#,
        )
        .unwrap();

        let report = migrate_legacy_data(&home);
        assert!(!report.completed_fast_path);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(
            fs::read_to_string(home.join(".zero/data/example.txt")).unwrap(),
            "legacy"
        );

        let second = migrate_legacy_data(&home);
        assert!(second.completed_fast_path);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn canonical_file_wins_over_legacy_file() {
        let home = temp_home("precedence");
        fs::create_dir_all(home.join(".ztool/data/wallpaper")).unwrap();
        fs::create_dir_all(home.join(".zero/data/wallpaper")).unwrap();
        fs::write(home.join(".ztool/data/wallpaper/index.json"), "legacy").unwrap();
        fs::write(home.join(".zero/data/wallpaper/index.json"), "canonical").unwrap();

        let report = migrate_legacy_data(&home);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert_eq!(
            fs::read_to_string(home.join(".zero/data/wallpaper/index.json")).unwrap(),
            "canonical"
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn malformed_registry_is_retained_and_reported() {
        let home = temp_home("malformed");
        fs::create_dir_all(home.join(".ztool/plugins")).unwrap();
        fs::write(home.join(".ztool/plugins/registry.json"), "not-json").unwrap();

        let report = migrate_legacy_data(&home);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("failed to parse migrated registry")));
        assert_eq!(
            fs::read_to_string(home.join(".ztool/plugins/registry.json")).unwrap(),
            "not-json"
        );
        assert_eq!(
            fs::read_to_string(home.join(".zero/plugins/registry.json")).unwrap(),
            "not-json"
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn copies_supported_plugin_wallpaper_and_launcher_data_units() {
        let home = temp_home("data-units");
        let legacy_root = home.join(".ztool");
        fs::create_dir_all(legacy_root.join("plugins/demo/1.0.0/dist")).unwrap();
        fs::create_dir_all(legacy_root.join("data/wallpaper")).unwrap();
        fs::create_dir_all(legacy_root.join("data/quick-launcher/icons")).unwrap();
        fs::write(
            legacy_root.join("plugins/demo/1.0.0/dist/index.html"),
            "<main>legacy plugin</main>",
        )
        .unwrap();
        fs::write(
            legacy_root.join("data/wallpaper/index.json"),
            r#"{"schemaVersion":1}"#,
        )
        .unwrap();
        fs::write(
            legacy_root.join("data/wallpaper/zero-paper.jpg"),
            b"wallpaper",
        )
        .unwrap();
        fs::write(
            legacy_root.join("data/quick-launcher/apps_cache.json"),
            r#"{"schemaVersion":1}"#,
        )
        .unwrap();
        fs::write(
            legacy_root.join("data/quick-launcher/usage.json"),
            r#"{"schemaVersion":1}"#,
        )
        .unwrap();
        fs::write(
            legacy_root.join("data/quick-launcher/icons/app.png"),
            b"icon",
        )
        .unwrap();

        let first = migrate_legacy_data(&home);
        assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
        assert_eq!(first.copied_entries, 6);
        let canonical_root = home.join(".zero");
        for relative in [
            "plugins/demo/1.0.0/dist/index.html",
            "data/wallpaper/index.json",
            "data/wallpaper/zero-paper.jpg",
            "data/quick-launcher/apps_cache.json",
            "data/quick-launcher/usage.json",
            "data/quick-launcher/icons/app.png",
        ] {
            assert!(canonical_root.join(relative).is_file(), "{relative}");
            assert!(legacy_root.join(relative).is_file(), "{relative}");
        }

        let second = migrate_legacy_data(&home);
        assert_eq!(second.copied_entries, 0);
        assert_eq!(second.normalized_files, 0);
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn keeps_legacy_install_path_when_package_copy_is_incomplete() {
        let home = temp_home("legacy-read-through");
        let legacy_plugins = home.join(".ztool/plugins");
        fs::create_dir_all(&legacy_plugins).unwrap();
        let legacy_install_path = legacy_plugins.join("demo/1.0.0");
        fs::write(
            legacy_plugins.join("registry.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": 3,
                "records": [{
                    "name": "third-party",
                    "installedPath": legacy_install_path
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        let report = migrate_legacy_data(&home);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let value: Value =
            serde_json::from_slice(&fs::read(home.join(".zero/plugins/registry.json")).unwrap())
                .unwrap();
        assert_eq!(
            value["records"][0]["installedPath"],
            legacy_install_path.to_string_lossy().as_ref()
        );
        fs::remove_dir_all(home).unwrap();
    }
}
