use serde_json::json;
use zero_lib::plugins::contracts::{
    NativeResourceError, NetworkFetchRequest, NetworkFetchResponse, PluginManifest,
    PluginMarketIndex, PluginPermission, StorageWriteFileRequest, StorageWriteFileResult,
    SystemSetWallpaperRequest,
};

#[test]
fn deserializes_plugin_manifest_contract() {
    let manifest: PluginManifest = serde_json::from_value(json!({
        "name": "clipboard-helper",
        "version": "0.1.0",
        "author": "watson",
        "main": "dist/index.html",
        "permissions": ["clipboard.read", "network"],
        "engines": {
            "zero": "0.1.0",
            "api": "1"
        }
    }))
    .expect("manifest should deserialize");

    assert_eq!(manifest.name, "clipboard-helper");
    assert_eq!(
        manifest.permissions,
        vec![PluginPermission::ClipboardRead, PluginPermission::Network,]
    );
    let engines = manifest.engines.expect("engines");
    assert_eq!(engines.zero.as_deref(), Some("0.1.0"));
    assert_eq!(engines.api.as_deref(), Some("1"));
}

#[test]
fn native_resource_contracts_use_stable_camel_case_fields() {
    let network = NetworkFetchRequest {
        url: "https://www.bing.com/".into(),
        method: Some("GET".into()),
    };
    let response = NetworkFetchResponse {
        status: 200,
        content_type: Some("application/json".into()),
        body_base64: "e30=".into(),
    };
    let write = StorageWriteFileRequest {
        relative_path: "images/today.jpg".into(),
        data_base64: "AA==".into(),
    };
    let written = StorageWriteFileResult {
        relative_path: "images/today.jpg".into(),
        bytes_written: 1,
    };
    let wallpaper = SystemSetWallpaperRequest {
        relative_path: "images/today.jpg".into(),
    };
    let error = NativeResourceError {
        operation: "network.fetch".into(),
        code: "network.timeout".into(),
        message: "timed out".into(),
        retryable: true,
    };

    assert_eq!(serde_json::to_value(network).unwrap()["method"], "GET");
    assert_eq!(
        serde_json::to_value(response).unwrap()["contentType"],
        "application/json"
    );
    assert_eq!(
        serde_json::to_value(write).unwrap()["relativePath"],
        "images/today.jpg"
    );
    assert_eq!(serde_json::to_value(written).unwrap()["bytesWritten"], 1);
    assert_eq!(
        serde_json::to_value(wallpaper).unwrap()["relativePath"],
        "images/today.jpg"
    );
    assert_eq!(serde_json::to_value(error).unwrap()["retryable"], true);
    assert!(serde_json::from_value::<NetworkFetchRequest>(json!({
        "url": 42,
        "method": "GET"
    }))
    .is_err());
}

#[test]
fn system_wallpaper_permission_has_stable_wire_value() {
    let serialized = serde_json::to_value(PluginPermission::SystemWallpaper)
        .expect("permission should serialize");
    assert_eq!(serialized, "system.wallpaper");
    assert_eq!(
        serde_json::from_value::<PluginPermission>(json!("system.wallpaper"))
            .expect("permission should deserialize"),
        PluginPermission::SystemWallpaper
    );
}

#[test]
fn launcher_permissions_have_stable_wire_values() {
    for (permission, wire) in [
        (PluginPermission::SystemAppsRead, "system.apps.read"),
        (PluginPermission::SystemAppsExecute, "system.apps.execute"),
        (PluginPermission::SystemWindowFocus, "system.window.focus"),
        (PluginPermission::SystemSettingsOpen, "system.settings.open"),
    ] {
        assert_eq!(serde_json::to_value(&permission).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<PluginPermission>(json!(wire)).unwrap(),
            permission
        );
    }
}

#[test]
fn deserializes_market_index_contract() {
    let market: PluginMarketIndex = serde_json::from_value(json!({
        "schemaVersion": 1,
        "updatedAt": "2026-06-21T00:00:00Z",
        "plugins": [
            {
                "name": "clipboard-helper",
                "version": "0.1.0",
                "author": "watson",
                "repository": "https://github.com/watson/clipboard-helper",
                "releaseUrl": "https://github.com/watson/clipboard-helper/releases/tag/v0.1.0",
                "downloadUrl": "https://github.com/watson/clipboard-helper/releases/download/v0.1.0/clipboard-helper.zplugin",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "permissions": ["clipboard.read"]
            }
        ]
    }))
    .expect("market index should deserialize");

    assert_eq!(market.schema_version, 1);
    assert!(market.plugins[0].download_url.ends_with(".zplugin"));
}
