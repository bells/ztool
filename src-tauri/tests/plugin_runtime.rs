use zero_lib::plugins::contracts::{
    PluginHealth, PluginManifest, PluginPermission, PluginRecord, PluginRuntime, PluginSource,
};
use zero_lib::plugins::runtime::{execute_plugin_entrypoint, PluginEntrypointExecutionRequest};

fn plugin_record(
    runtime: PluginRuntime,
    main: &str,
    approved_permissions: Vec<PluginPermission>,
) -> PluginRecord {
    let manifest = PluginManifest {
        name: "runtime-tool".into(),
        version: "0.1.0".into(),
        author: "watson".into(),
        main: main.into(),
        permissions: approved_permissions.clone(),
        id: Some("runtime-tool".into()),
        display_name: None,
        description: None,
        engines: None,
        platforms: None,
        runtime: Some(runtime),
        contributes: None,
        first_party_engine: None,
    };

    PluginRecord {
        name: "runtime-tool".into(),
        version: "0.1.0".into(),
        author: "watson".into(),
        source: PluginSource::Local,
        enabled: true,
        health: PluginHealth::Ready,
        manifest,
        installed_path: Some(std::env::temp_dir().to_string_lossy().into_owned()),
        approved_permissions,
        package_sha256: None,
    }
}

#[test]
fn denies_webview_entrypoint_process_execution() {
    let request = PluginEntrypointExecutionRequest {
        record: plugin_record(PluginRuntime::Webview, "dist/index.html", vec![]),
        args: vec![],
        timeout_ms: 100,
    };

    let error = execute_plugin_entrypoint(request)
        .expect_err("webview entrypoint should not execute as process");

    assert!(error.contains("unsupported runtime"));
}

#[test]
fn denies_process_execution_without_approved_permission() {
    let request = PluginEntrypointExecutionRequest {
        record: plugin_record(PluginRuntime::Binary, "bin/tool", vec![]),
        args: vec![],
        timeout_ms: 100,
    };

    let error = execute_plugin_entrypoint(request)
        .expect_err("missing process.execute permission should fail");

    assert!(error.contains("process.execute"));
}

#[test]
fn denies_unsafe_entrypoint_paths_before_launch() {
    let request = PluginEntrypointExecutionRequest {
        record: plugin_record(
            PluginRuntime::Binary,
            "../escape",
            vec![PluginPermission::ProcessExecute],
        ),
        args: vec![],
        timeout_ms: 100,
    };

    let error =
        execute_plugin_entrypoint(request).expect_err("unsafe main path should fail before launch");

    assert!(error.contains("safe package-relative path"));
}
