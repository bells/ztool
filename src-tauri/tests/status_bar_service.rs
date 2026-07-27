use std::collections::HashMap;
use std::fs;

use zero_lib::plugins::contracts::{
    PluginContributionStatusBarItem, PluginContributions, PluginHealth, PluginManifest,
    PluginPermission, PluginRecord, PluginSource, StatusBarAction, StatusBarActionType,
    StatusBarIconId,
};
use zero_lib::services::status_bar::{
    load_status_bar_settings, native_status_item_creation_order, normalize_status_bar_items,
    save_status_bar_settings, status_bar_action_effects, status_bar_icon_png_bytes,
    StatusBarActionEffect, StatusBarSettings, StatusBarSupport,
};

fn plugin_record(name: &str, enabled: bool, order: Option<u32>) -> PluginRecord {
    let is_caffeine = name == "zero.awake";
    let status_item = PluginContributionStatusBarItem {
        id: format!("{name}.status"),
        title: if is_caffeine {
            "Zero Awake".into()
        } else {
            "Zero Snap".into()
        },
        icon: if is_caffeine {
            StatusBarIconId::CaffeineEmpty
        } else {
            StatusBarIconId::Screenshot
        },
        active_icon: if is_caffeine {
            Some(StatusBarIconId::CaffeineFull)
        } else {
            None
        },
        action: StatusBarAction {
            action_type: if is_caffeine {
                StatusBarActionType::ToggleCaffeine
            } else {
                StatusBarActionType::StartScreenshot
            },
            command_id: None,
        },
        order,
        visible_by_default: Some(true),
    };

    PluginRecord {
        name: name.into(),
        version: "0.1.0".into(),
        author: "watson".into(),
        source: PluginSource::Bundled,
        enabled,
        health: if enabled {
            PluginHealth::Ready
        } else {
            PluginHealth::Disabled
        },
        manifest: PluginManifest {
            name: name.into(),
            version: "0.1.0".into(),
            author: "watson".into(),
            main: format!("plugins/{name}"),
            permissions: vec![PluginPermission::UiMessage],
            id: Some(name.into()),
            display_name: None,
            description: None,
            engines: None,
            platforms: None,
            runtime: None,
            contributes: Some(PluginContributions {
                views: None,
                commands: None,
                settings: None,
                status_bar_items: Some(vec![status_item]),
            }),
        },
        installed_path: None,
        approved_permissions: vec![PluginPermission::UiMessage],
        package_sha256: None,
    }
}

#[test]
fn status_bar_settings_default_enabled_plugins_visible() {
    let records = [
        plugin_record("zero.snap", true, Some(20)),
        plugin_record("zero.awake", true, Some(10)),
    ];

    let settings = StatusBarSettings::default_for_records(&records);

    assert!(settings.enabled);
    assert!(settings.show_plugin_items_on_launch);
    assert_eq!(
        settings.visible_plugin_items,
        HashMap::from([
            ("zero.snap".to_string(), true),
            ("zero.awake".to_string(), true),
        ]),
    );
}

#[test]
fn status_bar_settings_recovers_from_invalid_json() {
    let root = std::env::temp_dir().join(format!("zero-status-bar-test-{}", std::process::id()));
    let path = root.join("status-bar.json");
    fs::create_dir_all(&root).unwrap();
    fs::write(&path, "{not-json").unwrap();

    let records = [plugin_record("zero.snap", true, Some(20))];
    let settings = load_status_bar_settings(&path, &records).unwrap();

    assert!(settings.enabled);
    assert_eq!(settings.visible_plugin_items["zero.snap"], true);

    save_status_bar_settings(&path, &settings).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("zero.snap"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn status_bar_settings_migrate_legacy_keys_with_canonical_precedence() {
    let root = std::env::temp_dir().join(format!(
        "zero-status-bar-legacy-test-{}",
        std::process::id()
    ));
    let path = root.join("status-bar.json");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "enabled": true,
            "showPluginItemsOnLaunch": true,
            "visiblePluginItems": {
                "zero.snap": false,
                "ztool.screenshot": true,
                "ztool.caffeine": false,
                "ztool.third-party": true
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let records = [
        plugin_record("zero.snap", true, Some(20)),
        plugin_record("zero.awake", true, Some(10)),
        plugin_record("ztool.third-party", true, Some(30)),
    ];
    let settings = load_status_bar_settings(&path, &records).unwrap();

    assert_eq!(settings.visible_plugin_items["zero.snap"], false);
    assert_eq!(settings.visible_plugin_items["zero.awake"], false);
    assert_eq!(settings.visible_plugin_items["ztool.third-party"], true);
    assert!(!settings
        .visible_plugin_items
        .contains_key("ztool.screenshot"));
    assert!(!settings.visible_plugin_items.contains_key("ztool.caffeine"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn status_bar_items_filter_sort_and_reflect_caffeine_state() {
    let records = [
        plugin_record("zero.snap", true, Some(20)),
        plugin_record("zero.awake", true, Some(10)),
        plugin_record("ztool.disabled", false, Some(5)),
    ];
    let mut settings = StatusBarSettings::default_for_records(&records);
    settings
        .visible_plugin_items
        .insert("zero.snap".into(), true);

    let items =
        normalize_status_bar_items(&records, &settings, true, StatusBarSupport::NativeMultiItem);

    assert_eq!(
        items
            .iter()
            .map(|item| (item.id.as_str(), item.plugin_name.as_deref(), &item.icon))
            .collect::<Vec<_>>(),
        vec![
            ("zero.primary", None, &StatusBarIconId::Zero),
            (
                "zero.awake.status",
                Some("zero.awake"),
                &StatusBarIconId::CaffeineFull,
            ),
            (
                "zero.snap.status",
                Some("zero.snap"),
                &StatusBarIconId::Screenshot,
            ),
        ],
    );
    assert!(items.iter().all(|item| item.native_visible));
}

#[test]
fn status_bar_native_creation_order_rebuilds_primary_last_for_visual_order() {
    let records = [
        plugin_record("zero.snap", true, Some(20)),
        plugin_record("zero.awake", true, Some(10)),
    ];
    let settings = StatusBarSettings::default_for_records(&records);
    let items = normalize_status_bar_items(
        &records,
        &settings,
        false,
        StatusBarSupport::NativeMultiItem,
    );

    assert_eq!(
        native_status_item_creation_order(&items),
        vec![
            "zero.snap.status".to_string(),
            "zero.awake.status".to_string(),
            "zero.primary".to_string(),
        ],
    );
}

#[test]
fn status_bar_items_keep_plugin_actions_available_in_fallback() {
    let records = [plugin_record("zero.awake", true, Some(10))];
    let settings = StatusBarSettings::default_for_records(&records);

    let items = normalize_status_bar_items(
        &records,
        &settings,
        false,
        StatusBarSupport::FallbackActionRow,
    );

    assert_eq!(items[0].id, "zero.primary");
    assert!(items[0].native_visible);
    assert_eq!(
        items[1].action.action_type,
        StatusBarActionType::ToggleCaffeine
    );
    assert!(!items[1].native_visible);
}

#[test]
fn status_bar_open_plugin_action_shows_main_window_before_selecting_plugin() {
    let action = StatusBarAction {
        action_type: StatusBarActionType::OpenPlugin,
        command_id: None,
    };

    assert_eq!(
        status_bar_action_effects(&action, Some("market-tool")),
        vec![
            StatusBarActionEffect::ShowMainWindow,
            StatusBarActionEffect::EmitOpenPlugin("market-tool".into()),
        ],
    );
}

#[test]
fn status_bar_icon_ids_keep_existing_names_and_add_first_party_variants() {
    let cases = [
        ("zero", StatusBarIconId::Zero),
        ("launch", StatusBarIconId::Launch),
        ("caffeine-empty", StatusBarIconId::CaffeineEmpty),
        ("caffeine-full", StatusBarIconId::CaffeineFull),
        ("screenshot", StatusBarIconId::Screenshot),
        ("paper", StatusBarIconId::Paper),
        ("extension", StatusBarIconId::Extension),
    ];

    for (serialized, expected) in cases {
        let parsed: StatusBarIconId = serde_json::from_str(&format!("\"{serialized}\"")).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            format!("\"{serialized}\"")
        );
    }
}

#[test]
fn status_bar_icon_assets_are_transparent_monochrome_rgba() {
    let icons = [
        StatusBarIconId::Zero,
        StatusBarIconId::Launch,
        StatusBarIconId::CaffeineEmpty,
        StatusBarIconId::CaffeineFull,
        StatusBarIconId::Screenshot,
        StatusBarIconId::Paper,
        StatusBarIconId::Extension,
    ];

    for icon in icons {
        let image = image::load_from_memory(status_bar_icon_png_bytes(&icon)).unwrap();
        assert_eq!((image.width(), image.height()), (18, 18), "{icon:?}");

        let rgba = image.to_rgba8();
        assert!(rgba.pixels().any(|pixel| pixel.0[3] == 0), "{icon:?}");
        assert!(rgba.pixels().any(|pixel| pixel.0[3] > 0), "{icon:?}");
        assert!(
            rgba.pixels()
                .all(|pixel| pixel.0[0] == pixel.0[1] && pixel.0[1] == pixel.0[2]),
            "{icon:?}",
        );
    }
}

#[test]
fn status_bar_awake_state_uses_distinct_canonical_derivatives() {
    assert_ne!(
        status_bar_icon_png_bytes(&StatusBarIconId::CaffeineEmpty),
        status_bar_icon_png_bytes(&StatusBarIconId::CaffeineFull),
    );
}
