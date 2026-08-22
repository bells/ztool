use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use zero_lib::plugins::package::{
    archive_entry_destination, download_package_with_fetcher, sha256_hex, validate_zplugin_package,
    PluginPackageDownloadRequest,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn unique_staging_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "zero-plugin-package-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ))
}

fn write_zplugin_package(file_name: &str, manifest_json: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = unique_staging_dir();
    fs::create_dir_all(&root).expect("package root");
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

fn manifest_with_main(main: &str) -> String {
    format!(
        r#"{{
            "name": "package-tool",
            "version": "0.1.0",
            "author": "watson",
            "main": "{main}",
            "permissions": ["ui.message"]
        }}"#
    )
}

#[test]
fn stages_downloaded_package_when_checksum_matches() {
    let staging_dir = unique_staging_dir();
    let bytes = b"fake zplugin package".to_vec();
    let expected_sha256 = sha256_hex(&bytes);
    let request = PluginPackageDownloadRequest {
        download_url: "https://github.com/watson/plugin/releases/download/v0.1.0/plugin.zplugin"
            .into(),
        sha256: Some(expected_sha256.clone()),
    };

    let result = download_package_with_fetcher(&request, &staging_dir, |_| {
        Ok::<Vec<u8>, String>(bytes.clone())
    })
    .expect("download should be staged");

    assert_eq!(result.sha256, expected_sha256);
    assert_eq!(fs::read(result.staged_path).expect("staged bytes"), bytes);
}

#[test]
fn checksum_mismatch_removes_staged_package() {
    let staging_dir = unique_staging_dir();
    let request = PluginPackageDownloadRequest {
        download_url: "https://github.com/watson/plugin/releases/download/v0.1.0/plugin.zplugin"
            .into(),
        sha256: Some("0000000000000000000000000000000000000000000000000000000000000000".into()),
    };

    let error = download_package_with_fetcher(&request, &staging_dir, |_| {
        Ok::<Vec<u8>, String>(b"fake zplugin package".to_vec())
    })
    .expect_err("checksum mismatch should fail");

    assert!(error.message.contains("checksum"));
    assert!(!staging_dir.exists());
}

#[test]
fn download_failure_does_not_create_staging_dir() {
    let staging_dir = unique_staging_dir();
    let request = PluginPackageDownloadRequest {
        download_url: "https://github.com/watson/plugin/releases/download/v0.1.0/plugin.zplugin"
            .into(),
        sha256: None,
    };

    let error = download_package_with_fetcher(&request, &staging_dir, |_| {
        Err::<Vec<u8>, _>("network unavailable")
    })
    .expect_err("download failure should fail");

    assert!(error.message.contains("failed to download"));
    assert!(!staging_dir.exists());
}

#[test]
fn rejects_non_zplugin_download_url() {
    let staging_dir = unique_staging_dir();
    let request = PluginPackageDownloadRequest {
        download_url: "https://github.com/watson/plugin/releases/download/v0.1.0/plugin.zip".into(),
        sha256: None,
    };

    let error = download_package_with_fetcher(&request, &staging_dir, |_| {
        Ok::<Vec<u8>, String>(b"fake zip package".to_vec())
    })
    .expect_err("non-zplugin asset should fail");

    assert!(error.message.contains(".zplugin"));
    assert!(!staging_dir.exists());
}

#[test]
fn archive_entry_destination_stays_inside_install_root() {
    let root = unique_staging_dir();
    let destination =
        archive_entry_destination(&root, "dist/index.html").expect("entry path should be accepted");

    assert_eq!(destination, root.join("dist").join("index.html"));
}

#[test]
fn archive_entry_destination_rejects_unsafe_paths() {
    let root = unique_staging_dir();

    for entry in [
        "../evil",
        "/tmp/evil",
        "dist/../../evil",
        "",
        "C:\\\\Temp\\\\evil",
    ] {
        let error =
            archive_entry_destination(&root, entry).expect_err("unsafe archive entry should fail");

        assert!(error.message.contains("unsafe archive entry"));
    }
}

#[test]
fn package_validation_accepts_valid_zplugin_zip() {
    let package = write_zplugin_package(
        "package-tool.zplugin",
        &manifest_with_main("dist/index.html"),
        &[("dist/index.html", "<main>ok</main>")],
    );

    let report = validate_zplugin_package(&package).expect("package should validate");

    assert!(report.valid);
    assert_eq!(report.manifest.expect("manifest").name, "package-tool");
    assert!(report.sha256.len() == 64);
}

#[test]
fn package_validation_canonicalizes_legacy_zero_host_key() {
    let package = write_zplugin_package(
        "package-tool.zplugin",
        r#"{
            "name": "package-tool",
            "version": "0.1.0",
            "author": "watson",
            "main": "dist/index.html",
            "permissions": ["ui.message"],
            "engines": {"ztool": "0.1.0", "api": "1"}
        }"#,
        &[("dist/index.html", "<main>ok</main>")],
    );

    let report = validate_zplugin_package(&package).expect("package should validate");
    assert!(report.valid);
    let engines = report.manifest.expect("manifest").engines.expect("engines");

    assert_eq!(engines.zero.as_deref(), Some("0.1.0"));
    assert_eq!(engines.ztool, None);
}

#[test]
fn package_validation_uses_canonical_host_key_when_both_are_present() {
    let package = write_zplugin_package(
        "package-tool.zplugin",
        r#"{
            "name": "package-tool",
            "version": "0.1.0",
            "author": "watson",
            "main": "dist/index.html",
            "permissions": ["ui.message"],
            "engines": {"zero": "0.1.0", "ztool": "999.0.0", "api": "1"}
        }"#,
        &[("dist/index.html", "<main>ok</main>")],
    );

    let report = validate_zplugin_package(&package).expect("package should validate");
    assert!(report.valid);
    let engines = report.manifest.expect("manifest").engines.expect("engines");

    assert_eq!(engines.zero.as_deref(), Some("0.1.0"));
    assert_eq!(engines.ztool, None);
}

#[test]
fn package_validation_rejects_missing_manifest_main_asset() {
    let package = write_zplugin_package(
        "package-tool.zplugin",
        &manifest_with_main("dist/missing.html"),
        &[("dist/index.html", "<main>wrong</main>")],
    );

    let report = validate_zplugin_package(&package).expect("validation report should return");

    assert!(!report.valid);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "package.main.missing"));
}

#[test]
fn package_validation_rejects_unsafe_archive_entries() {
    let package = write_zplugin_package(
        "package-tool.zplugin",
        &manifest_with_main("dist/index.html"),
        &[
            ("../evil.txt", "evil"),
            ("dist/index.html", "<main>ok</main>"),
        ],
    );

    let report = validate_zplugin_package(&package).expect("validation report should return");

    assert!(!report.valid);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "package.archive.unsafePath"));
}

#[test]
fn generic_plugin_cannot_claim_document_conversion_trust() {
    let package = write_zplugin_package(
        "impostor.zplugin",
        r#"{
            "name": "impostor",
            "version": "0.1.0",
            "author": "attacker",
            "main": "dist/index.html",
            "id": "zero.file",
            "permissions": ["document.convert"]
        }"#,
        &[("dist/index.html", "<main>wrong</main>")],
    );
    let report = validate_zplugin_package(&package).expect("validation report");
    assert!(!report.valid);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "manifest.documentConvert.firstPartyOnly"));
}

#[test]
fn first_party_engine_rejects_unsigned_unsafe_and_digest_mismatched_assets() {
    let package = write_zplugin_package(
        "zero-file.zplugin",
        r#"{
            "name": "zero.file",
            "id": "zero.file",
            "version": "1.0.0",
            "author": "bells",
            "main": "dist/index.html",
            "permissions": ["document.convert"],
            "firstPartyEngine": {
              "protocolVersion": 1,
              "packageVersion": "1.0.0",
              "hostApiRange": "0.1.0",
              "directions": ["pdfToDocx"],
              "platformMinimums": [{"platform": "macos", "version": "11.0"}],
              "assets": [
                {"path": "engine/worker.mjs", "sha256": "0000000000000000000000000000000000000000000000000000000000000000", "bytes": 6, "mediaType": "text/javascript"},
                {"path": "engine/licenses/NOTICE.txt", "sha256": "0000000000000000000000000000000000000000000000000000000000000000", "bytes": 6, "mediaType": "text/plain"},
                {"path": "engine/../escape", "sha256": "0000000000000000000000000000000000000000000000000000000000000000", "bytes": 1, "mediaType": "text/plain"}
              ],
              "notices": ["engine/licenses/NOTICE.txt"],
              "signature": "unsigned"
            }
        }"#,
        &[
            ("dist/index.html", "<main>Zero File</main>"),
            ("engine/worker.mjs", "worker"),
            ("engine/licenses/NOTICE.txt", "notice"),
        ],
    );
    let report = validate_zplugin_package(&package).expect("validation report");
    assert!(!report.valid);
    for code in [
        "engine.asset.pathInvalid",
        "engine.asset.digestMismatch",
        "engine.signature.invalid",
    ] {
        assert!(
            report.issues.iter().any(|issue| issue.code == code),
            "missing {code}"
        );
    }
}

#[test]
fn first_party_engine_rejects_incompatible_duplicate_undeclared_and_oversized_metadata() {
    let wrong_platform = if cfg!(target_os = "linux") {
        "windows"
    } else {
        "linux"
    };
    let manifest = serde_json::json!({
        "name": "zero.file",
        "id": "zero.file",
        "version": "1.0.0",
        "author": "bells",
        "main": "engine/index.html",
        "permissions": ["document.convert"],
        "engines": {"zero": ">=99.0.0", "api": "1"},
        "platforms": [wrong_platform],
        "firstPartyEngine": {
            "protocolVersion": 2,
            "packageVersion": "1.0.0",
            "hostApiRange": ">=99.0.0",
            "directions": ["pdfToDocx"],
            "platformMinimums": [{"platform": wrong_platform, "version": "99.0"}],
            "assets": [
                {"path": "engine/index.html", "sha256": "00".repeat(32), "bytes": 1, "mediaType": "application/x-executable"},
                {"path": "engine/index.html", "sha256": "00".repeat(32), "bytes": 1, "mediaType": "text/html"},
                {"path": "engine/huge.wasm", "sha256": "00".repeat(32), "bytes": 50 * 1024 * 1024, "mediaType": "application/wasm"}
            ],
            "notices": ["NOTICE.txt"],
            "signature": "unsigned"
        }
    })
    .to_string();
    let package = write_zplugin_package(
        "zero-file-incompatible.zplugin",
        &manifest,
        &[
            ("engine/index.html", "x"),
            ("engine/extra.mjs", "undeclared"),
        ],
    );

    let report = validate_zplugin_package(&package).expect("validation report");
    assert!(!report.valid);
    for code in [
        "manifest.zero.incompatible",
        "manifest.platform.incompatible",
        "engine.protocol.incompatible",
        "engine.host.incompatible",
        "engine.asset.pathInvalid",
        "engine.asset.mediaTypeInvalid",
        "engine.asset.sizeInvalid",
        "engine.asset.undeclared",
        "engine.notice.invalid",
        "engine.signature.invalid",
    ] {
        assert!(
            report.issues.iter().any(|issue| issue.code == code),
            "missing {code}: {:?}",
            report.issues
        );
    }
}
