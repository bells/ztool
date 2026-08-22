use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use zero_lib::plugins::contracts::{
    InstallPluginPackageInput, PluginMarketEntry, PluginPermission, PluginPlatform, PluginRuntime,
    PluginSource,
};
use zero_lib::plugins::registry::PluginRegistry;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn unique_registry_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "zero-plugin-registry-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ))
}

fn write_zplugin_package(
    root: &std::path::Path,
    file_name: &str,
    manifest_json: &str,
    files: &[(&str, &str)],
) -> PathBuf {
    fs::create_dir_all(root).expect("package root");
    let package_path = root.join(file_name);
    let file = fs::File::create(&package_path).expect("package file");
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);

    zip.start_file("manifest.json", options)
        .expect("manifest entry");
    zip.write_all(manifest_json.as_bytes())
        .expect("manifest bytes");

    for (path, content) in files {
        zip.start_file(path, options).expect("content entry");
        zip.write_all(content.as_bytes()).expect("content bytes");
    }

    zip.finish().expect("finish package");
    package_path
}

fn valid_manifest(name: &str) -> String {
    valid_manifest_version(name, "0.1.0")
}

fn valid_manifest_version(name: &str, version: &str) -> String {
    format!(
        r#"{{
            "name": "{name}",
            "version": "{version}",
            "author": "watson",
            "main": "dist/index.html",
            "permissions": ["ui.message"]
        }}"#
    )
}

fn install_input(package_path: &std::path::Path) -> InstallPluginPackageInput {
    InstallPluginPackageInput {
        package_path: package_path.to_string_lossy().into_owned(),
        approved_permissions: vec![PluginPermission::UiMessage],
        enabled: Some(true),
    }
}

#[test]
fn first_load_seeds_bundled_plugins() {
    let root = unique_registry_root();

    let registry = PluginRegistry::load_or_seed(root).expect("registry should load");

    let names = registry
        .records()
        .iter()
        .map(|record| record.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "zero.snap",
            "zero.awake",
            "zero.paper",
            "zero.launch",
            "zero.file",
        ]
    );
    assert!(registry.records().iter().all(|record| record.enabled));
}

#[test]
fn bundled_plugin_records_include_host_manifest_contributions() {
    let root = unique_registry_root();

    let registry = PluginRegistry::load_or_seed(root).expect("registry should load");
    let screenshot = registry
        .records()
        .iter()
        .find(|record| record.name == "zero.snap")
        .expect("screenshot record");
    let caffeine = registry
        .records()
        .iter()
        .find(|record| record.name == "zero.awake")
        .expect("caffeine record");
    let bing = registry
        .records()
        .iter()
        .find(|record| record.name == "zero.paper")
        .expect("Bing wallpaper record");
    let launcher = registry
        .records()
        .iter()
        .find(|record| record.name == "zero.launch")
        .expect("Zero Launch record");
    let file = registry
        .records()
        .iter()
        .find(|record| record.name == "zero.file")
        .expect("Zero File record");

    assert_eq!(screenshot.manifest.runtime, Some(PluginRuntime::Webview));
    assert_eq!(screenshot.manifest.main, "plugins/screenshot");
    assert!(screenshot
        .manifest
        .contributes
        .as_ref()
        .and_then(|contributes| contributes.views.as_ref())
        .is_some_and(|views| views.iter().any(|view| view.id == "zero.snap.main")));
    assert!(caffeine
        .manifest
        .contributes
        .as_ref()
        .and_then(|contributes| contributes.commands.as_ref())
        .is_some_and(|commands| commands
            .iter()
            .any(|command| command.id == "zero.awake.toggle")));
    assert_eq!(bing.author, "bells");
    assert_eq!(bing.version, "1.0.0");
    assert_eq!(bing.manifest.version, "1.0.0");
    assert_eq!(bing.manifest.id.as_deref(), Some("zero.paper"));
    assert_eq!(
        bing.manifest.permissions,
        vec![
            PluginPermission::Network,
            PluginPermission::StoragePlugin,
            PluginPermission::SystemWallpaper,
        ]
    );
    assert!(bing
        .manifest
        .contributes
        .as_ref()
        .and_then(|contributes| contributes.commands.as_ref())
        .is_some_and(|commands| commands
            .iter()
            .any(|command| command.id == "zero.paper.apply")));
    assert_eq!(launcher.author, "bells");
    assert_eq!(launcher.version, "1.0.0");
    assert_eq!(launcher.manifest.id.as_deref(), Some("zero.launch"));
    assert_eq!(
        launcher.manifest.permissions,
        vec![
            PluginPermission::SystemAppsRead,
            PluginPermission::SystemAppsExecute,
            PluginPermission::SystemWindowFocus,
            PluginPermission::SystemSettingsOpen,
        ]
    );
    assert_eq!(file.author, "bells");
    assert_eq!(file.version, "1.0.0");
    assert_eq!(file.manifest.id.as_deref(), Some("zero.file"));
    assert!(file.manifest.permissions.is_empty());
    assert_eq!(
        file.manifest.platforms,
        Some(vec![PluginPlatform::Macos, PluginPlatform::Windows])
    );
    let file_contributions = file
        .manifest
        .contributes
        .as_ref()
        .expect("File contributions");
    assert!(file_contributions
        .views
        .as_ref()
        .is_some_and(|views| views.len() == 1 && views[0].id == "zero.file.main"));
    assert!(file_contributions.commands.is_none());
    assert!(file_contributions.status_bar_items.is_none());
}

#[test]
fn registry_state_persists_across_reloads() {
    let root = unique_registry_root();
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");

    registry
        .set_enabled("zero.awake", false)
        .expect("plugin should update");
    registry.save().expect("registry should save");

    let reloaded = PluginRegistry::load_or_seed(root).expect("registry should reload");
    let caffeine = reloaded
        .records()
        .iter()
        .find(|record| record.name == "zero.awake")
        .expect("caffeine record");

    assert!(!caffeine.enabled);
}

#[test]
fn older_registry_migration_adds_only_quick_launcher_and_preserves_lifecycle_state() {
    let root = unique_registry_root();
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");
    registry
        .set_enabled("zero.awake", false)
        .expect("plugin should update");
    registry.save().expect("registry should save");

    let path = root.join("registry.json");
    let mut disk: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    disk["schemaVersion"] = serde_json::json!(2);
    disk["records"] = serde_json::Value::Array(
        disk["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|record| record["name"] != "zero.launch")
            .cloned()
            .collect(),
    );
    fs::write(&path, serde_json::to_vec_pretty(&disk).unwrap()).unwrap();

    let migrated = PluginRegistry::load_or_seed(root).expect("registry should migrate");
    assert!(migrated
        .records()
        .iter()
        .any(|record| record.name == "zero.launch" && record.enabled));
    assert!(migrated
        .records()
        .iter()
        .any(|record| record.name == "zero.awake" && !record.enabled));
}

#[test]
fn legacy_first_party_records_are_canonicalized_and_canonical_record_wins() {
    let root = unique_registry_root();
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");
    registry
        .set_enabled("zero.awake", false)
        .expect("canonical record should update");
    registry.save().expect("registry should save");

    let path = root.join("registry.json");
    let mut disk: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let canonical = disk["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["name"] == "zero.awake")
        .unwrap()
        .clone();
    let mut legacy = canonical;
    legacy["name"] = serde_json::json!("ztool.caffeine");
    legacy["enabled"] = serde_json::json!(true);
    legacy["manifest"]["name"] = serde_json::json!("ztool.caffeine");
    legacy["manifest"]["id"] = serde_json::json!("ztool.caffeine");
    legacy["manifest"]["contributes"]["commands"][0]["id"] =
        serde_json::json!("ztool.caffeine.toggle");
    disk["records"].as_array_mut().unwrap().push(legacy);
    fs::write(&path, serde_json::to_vec_pretty(&disk).unwrap()).unwrap();

    let migrated = PluginRegistry::load_or_seed(root).expect("registry should migrate");
    let awake = migrated
        .records()
        .iter()
        .filter(|record| record.name == "zero.awake")
        .collect::<Vec<_>>();

    assert_eq!(awake.len(), 1);
    assert!(!awake[0].enabled);
    assert_eq!(
        awake[0]
            .manifest
            .contributes
            .as_ref()
            .unwrap()
            .commands
            .as_ref()
            .unwrap()[0]
            .id,
        "zero.awake.toggle"
    );
}

#[test]
fn corrupt_registry_recovers_with_bundled_plugins() {
    let root = unique_registry_root();
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("registry.json"), "{ definitely not json").expect("corrupt registry");

    let registry = PluginRegistry::load_or_seed(root).expect("registry should recover");

    assert_eq!(registry.records().len(), 5);
    assert!(registry
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.contains("registry recovery")));
}

#[test]
fn local_package_install_extracts_versioned_assets_and_persists_record() {
    let root = unique_registry_root();
    let package = write_zplugin_package(
        &root.join("packages"),
        "local-tool.zplugin",
        &valid_manifest("local-tool"),
        &[("dist/index.html", "<main>hello</main>")],
    );
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");

    let record = registry
        .install_local_package(install_input(&package))
        .expect("local package should install");

    let installed_path = root.join("local-tool").join("0.1.0");
    assert_eq!(record.name, "local-tool");
    assert_eq!(record.source, PluginSource::Local);
    assert_eq!(
        record.installed_path.as_deref(),
        Some(installed_path.to_str().unwrap())
    );
    assert!(record.package_sha256.is_some());
    assert!(installed_path.join("manifest.json").exists());
    assert!(installed_path.join("dist").join("index.html").exists());

    let reloaded = PluginRegistry::load_or_seed(root).expect("registry should reload");
    assert!(reloaded
        .records()
        .iter()
        .any(|record| record.name == "local-tool" && record.enabled));
}

#[test]
fn install_validation_failure_leaves_registry_and_files_unchanged() {
    let root = unique_registry_root();
    let package = write_zplugin_package(
        &root.join("packages"),
        "bad-tool.zplugin",
        &valid_manifest("bad-tool"),
        &[],
    );
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");

    let error = registry
        .install_local_package(install_input(&package))
        .expect_err("missing main asset should fail install");

    assert!(error.contains("package.main.missing"));
    assert_eq!(registry.records().len(), 5);
    assert!(!root.join("bad-tool").exists());
}

#[test]
fn duplicate_plugin_install_is_rejected_without_overwriting_existing_assets() {
    let root = unique_registry_root();
    let package = write_zplugin_package(
        &root.join("packages"),
        "duplicate-tool.zplugin",
        &valid_manifest("duplicate-tool"),
        &[("dist/index.html", "first")],
    );
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");

    registry
        .install_local_package(install_input(&package))
        .expect("first install should succeed");
    let error = registry
        .install_local_package(install_input(&package))
        .expect_err("duplicate install should fail");

    assert!(error.contains("already installed"));
    let installed_file = root
        .join("duplicate-tool")
        .join("0.1.0")
        .join("dist")
        .join("index.html");
    assert_eq!(
        fs::read_to_string(installed_file).expect("installed file"),
        "first"
    );
}

#[test]
fn plugin_update_atomically_switches_record_and_retains_one_rollback_version() {
    let root = unique_registry_root();
    let packages = root.join("packages");
    let first = write_zplugin_package(
        &packages,
        "update-tool-0.1.0.zplugin",
        &valid_manifest_version("update-tool", "0.1.0"),
        &[("dist/index.html", "first")],
    );
    let second = write_zplugin_package(
        &packages,
        "update-tool-0.2.0.zplugin",
        &valid_manifest_version("update-tool", "0.2.0"),
        &[("dist/index.html", "second")],
    );
    let third = write_zplugin_package(
        &packages,
        "update-tool-0.3.0.zplugin",
        &valid_manifest_version("update-tool", "0.3.0"),
        &[("dist/index.html", "third")],
    );
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");
    registry
        .install_local_package(install_input(&first))
        .expect("first version should install");
    registry
        .set_enabled("update-tool", false)
        .expect("installed plugin should disable");
    registry.save().expect("disabled state should persist");

    let updated = registry
        .install_local_package(InstallPluginPackageInput {
            package_path: second.to_string_lossy().into_owned(),
            approved_permissions: vec![PluginPermission::UiMessage],
            enabled: Some(true),
        })
        .expect("new version should activate");

    assert_eq!(updated.version, "0.2.0");
    assert!(
        !updated.enabled,
        "updates preserve the user's enabled state"
    );
    assert!(root.join("update-tool").join("0.1.0").exists());
    assert_eq!(
        fs::read_to_string(root.join("update-tool/0.2.0/dist/index.html")).unwrap(),
        "second"
    );
    let updated = registry
        .install_local_package(install_input(&third))
        .expect("third version should activate");
    assert_eq!(updated.version, "0.3.0");
    assert!(!root.join("update-tool/0.1.0").exists());
    assert!(root.join("update-tool/0.2.0").exists());

    let reloaded = PluginRegistry::load_or_seed(root).expect("updated registry should reload");
    assert!(reloaded
        .records()
        .iter()
        .any(|record| record.name == "update-tool"
            && record.version == "0.3.0"
            && !record.enabled));
}

#[test]
fn forged_registry_engine_metadata_is_not_treated_as_trusted_install_evidence() {
    let root = unique_registry_root();
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");
    let engine_root = root.join("zero.file/1.0.0/engine");
    fs::create_dir_all(&engine_root).expect("engine root");
    let mut disk: serde_json::Value = serde_json::from_str(
        &serde_json::to_string(&serde_json::json!({
            "schemaVersion": 5,
            "records": registry.records(),
        }))
        .unwrap(),
    )
    .unwrap();
    let record = disk["records"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["name"] == "zero.file")
        .unwrap();
    record["source"] = serde_json::json!("local");
    record["installedPath"] = serde_json::json!(root.join("zero.file/1.0.0"));
    record["approvedPermissions"] = serde_json::json!(["document.convert"]);
    record["manifest"]["permissions"] = serde_json::json!(["document.convert"]);
    record["manifest"]["firstPartyEngine"] = serde_json::json!({
        "protocolVersion": 1,
        "packageVersion": "1.0.0",
        "hostApiRange": "*",
        "directions": ["pdfToDocx"],
        "platformMinimums": [],
        "assets": [{"path": "engine/index.html", "sha256": "00", "bytes": 1, "mediaType": "text/html"}],
        "notices": ["engine/licenses/NOTICE"],
        "signature": "test"
    });
    fs::write(
        root.join("registry.json"),
        serde_json::to_vec_pretty(&disk).unwrap(),
    )
    .unwrap();
    registry = PluginRegistry::load_or_seed(root.clone()).expect("trusted test record should load");

    let error = registry
        .acquire_active_engine("zero.file")
        .expect_err("unsigned registry metadata must not grant engine trust");
    assert!(error.contains("integrity verification failed"));
}

#[test]
fn market_package_install_requires_manifest_to_match_market_entry() {
    let root = unique_registry_root();
    let package = write_zplugin_package(
        &root.join("packages"),
        "market-tool.zplugin",
        &valid_manifest("market-tool"),
        &[("dist/index.html", "<main>market</main>")],
    );
    let entry = PluginMarketEntry {
        name: "market-tool".into(),
        version: "0.1.0".into(),
        author: "watson".into(),
        repository: "https://github.com/watson/market-tool".into(),
        release_url: "https://github.com/watson/market-tool/releases/tag/v0.1.0".into(),
        download_url:
            "https://github.com/watson/market-tool/releases/download/v0.1.0/market-tool.zplugin"
                .into(),
        permissions: vec![PluginPermission::UiMessage],
        description: None,
        sha256: None,
        installed_version: None,
    };
    let mut registry = PluginRegistry::load_or_seed(root).expect("registry should load");

    let record = registry
        .install_market_package_from_path(&entry, install_input(&package))
        .expect("market package should install");

    assert_eq!(record.name, "market-tool");
    assert_eq!(record.source, PluginSource::Market);
}

#[test]
fn uninstall_removes_market_or_local_assets_but_keeps_host_registry_usable() {
    let root = unique_registry_root();
    let package = write_zplugin_package(
        &root.join("packages"),
        "remove-me.zplugin",
        &valid_manifest("remove-me"),
        &[("dist/index.html", "<main>bye</main>")],
    );
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");
    registry
        .install_local_package(install_input(&package))
        .expect("install should succeed");

    let result = registry
        .uninstall_plugin("remove-me")
        .expect("uninstall should succeed");

    assert!(result.message.contains("uninstalled"));
    assert!(!root.join("remove-me").exists());
    assert!(!registry
        .records()
        .iter()
        .any(|record| record.name == "remove-me"));
    assert!(registry
        .records()
        .iter()
        .any(|record| record.name == "zero.snap"));
}

#[test]
fn bundled_restore_readds_removed_default_plugins() {
    let root = unique_registry_root();
    let mut registry = PluginRegistry::load_or_seed(root.clone()).expect("registry should load");

    registry
        .uninstall_plugin("zero.snap")
        .expect("bundled uninstall should remove active record");
    assert!(!registry
        .records()
        .iter()
        .any(|record| record.name == "zero.snap"));

    let restored = registry
        .restore_bundled_defaults()
        .expect("restore should succeed");

    assert!(restored.iter().any(|record| record.name == "zero.snap"));
    let reloaded = PluginRegistry::load_or_seed(root).expect("registry should reload");
    assert!(reloaded
        .records()
        .iter()
        .any(|record| record.name == "zero.snap" && record.enabled));
}
