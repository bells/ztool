use std::collections::HashMap;
use std::fs;

use zero_lib::brand::{
    PRIMARY_STATUS_ITEM_ID, ZERO_AWAKE_PLUGIN_ID, ZERO_LAUNCH_PLUGIN_ID, ZERO_PAPER_PLUGIN_ID,
    ZERO_SNAP_PLUGIN_ID,
};
use zero_lib::plugins::contracts::{
    PluginContributionStatusBarItem, PluginContributions, PluginHealth, PluginManifest,
    PluginPermission, PluginRecord, PluginSource, StatusBarAction, StatusBarActionType,
    StatusBarIconId,
};
use zero_lib::services::status_bar::{
    grouped_status_item_cell_rect, grouped_status_item_id_at_x, grouped_status_item_ids,
    grouped_status_item_length, load_status_bar_settings, native_status_bar_activation,
    normalize_status_bar_items, primary_status_bar_menu_action, save_status_bar_settings,
    status_bar_action_effects, status_bar_collapse_menu_label, status_bar_icon_png_bytes,
    status_bar_plugin_items_collapse_update, status_bar_quit_menu_label,
    tool_status_bar_menu_action, tool_status_bar_quit_menu_id, NativeStatusBarActivation,
    PrimaryStatusBarMenuAction, StatusBarActionEffect, StatusBarSettings, StatusBarState,
    StatusBarSupport, ToolStatusBarMenuAction, UpdateStatusBarSettingsInput,
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
            first_party_engine: None,
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
    assert!(!settings.plugin_items_collapsed);
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
    assert!(!settings.plugin_items_collapsed);
    assert!(settings.visible_plugin_items["zero.snap"]);

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

    assert!(!settings.plugin_items_collapsed);
    assert!(!settings.visible_plugin_items["zero.snap"]);
    assert!(!settings.visible_plugin_items["zero.awake"]);
    assert!(settings.visible_plugin_items["ztool.third-party"]);
    assert!(!settings
        .visible_plugin_items
        .contains_key("ztool.screenshot"));
    assert!(!settings.visible_plugin_items.contains_key("ztool.caffeine"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn status_bar_settings_persist_collapse_updates_without_losing_visibility() {
    let root = std::env::temp_dir().join(format!(
        "zero-status-bar-collapse-test-{}",
        std::process::id()
    ));
    let path = root.join("status-bar.json");
    let records = [
        plugin_record("zero.snap", true, Some(20)),
        plugin_record("zero.awake", true, Some(10)),
    ];
    let state = StatusBarState::default();
    state.configure_path(path.clone(), &records).unwrap();

    let collapsed = state
        .update(
            &records,
            UpdateStatusBarSettingsInput {
                enabled: None,
                show_plugin_items_on_launch: None,
                plugin_items_collapsed: Some(true),
                visible_plugin_items: Some(HashMap::from([("zero.snap".into(), false)])),
            },
        )
        .unwrap();
    assert!(collapsed.plugin_items_collapsed);
    assert!(!collapsed.visible_plugin_items["zero.snap"]);
    assert!(collapsed.visible_plugin_items["zero.awake"]);

    let reloaded = load_status_bar_settings(&path, &records).unwrap();
    assert!(reloaded.plugin_items_collapsed);
    assert!(!reloaded.visible_plugin_items["zero.snap"]);

    let expanded = state
        .update(
            &records,
            UpdateStatusBarSettingsInput {
                enabled: None,
                show_plugin_items_on_launch: None,
                plugin_items_collapsed: Some(false),
                visible_plugin_items: None,
            },
        )
        .unwrap();
    assert!(!expanded.plugin_items_collapsed);
    assert!(!expanded.visible_plugin_items["zero.snap"]);

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
fn grouped_status_item_expands_tools_to_the_left_of_primary_in_one_native_slot() {
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
        grouped_status_item_ids(&items, false),
        vec![
            "zero.awake.status".to_string(),
            "zero.snap.status".to_string(),
            PRIMARY_STATUS_ITEM_ID.to_string(),
        ]
    );
    assert_eq!(grouped_status_item_length(&items, false), 66.0);
    assert_eq!(
        grouped_status_item_ids(&items, true),
        vec![PRIMARY_STATUS_ITEM_ID.to_string()]
    );
    assert_eq!(grouped_status_item_length(&items, true), 22.0);
}

#[test]
fn grouped_status_item_routes_each_horizontal_cell_and_keeps_primary_on_the_right() {
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
        grouped_status_item_id_at_x(&items, false, 111.0, 100.0, 66.0).as_deref(),
        Some("zero.awake.status")
    );
    assert_eq!(
        grouped_status_item_id_at_x(&items, false, 133.0, 100.0, 66.0).as_deref(),
        Some("zero.snap.status")
    );
    assert_eq!(
        grouped_status_item_id_at_x(&items, false, 155.0, 100.0, 66.0).as_deref(),
        Some(PRIMARY_STATUS_ITEM_ID)
    );
    assert_eq!(
        grouped_status_item_id_at_x(&items, true, 111.0, 100.0, 22.0).as_deref(),
        Some(PRIMARY_STATUS_ITEM_ID)
    );
    assert_eq!(
        grouped_status_item_id_at_x(&items, false, 99.0, 100.0, 66.0),
        None
    );
}

#[test]
fn grouped_status_item_returns_the_resolved_virtual_cell_rectangle() {
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
    let grouped_rect = tauri::Rect {
        position: tauri::PhysicalPosition::new(100, 2).into(),
        size: tauri::PhysicalSize::new(66, 22).into(),
    };

    assert_physical_rect(
        grouped_status_item_cell_rect(&items, false, "zero.awake.status", grouped_rect)
            .expect("awake cell"),
        (100, 2, 22, 22),
    );
    assert_physical_rect(
        grouped_status_item_cell_rect(&items, false, PRIMARY_STATUS_ITEM_ID, grouped_rect)
            .expect("primary cell"),
        (144, 2, 22, 22),
    );
    assert!(
        grouped_status_item_cell_rect(&items, true, "zero.awake.status", grouped_rect).is_none()
    );
    assert!(grouped_status_item_cell_rect(
        &items,
        false,
        "zero.awake.status",
        tauri::Rect {
            position: tauri::LogicalPosition::new(100.0, 2.0).into(),
            size: tauri::LogicalSize::new(66.0, 22.0).into(),
        },
    )
    .is_none());
}

fn assert_physical_rect(rect: tauri::Rect, expected: (i32, i32, u32, u32)) {
    match (rect.position, rect.size) {
        (tauri::Position::Physical(position), tauri::Size::Physical(size)) => {
            assert_eq!((position.x, position.y, size.width, size.height), expected);
        }
        _ => panic!("expected physical rectangle"),
    }
}

#[test]
fn native_status_bar_activation_specializes_only_launch_and_paper() {
    assert_eq!(
        native_status_bar_activation(Some(ZERO_LAUNCH_PLUGIN_ID)),
        NativeStatusBarActivation::Launch
    );
    assert_eq!(
        native_status_bar_activation(Some(ZERO_PAPER_PLUGIN_ID)),
        NativeStatusBarActivation::Paper
    );
    for plugin_name in [
        Some(ZERO_SNAP_PLUGIN_ID),
        Some(ZERO_AWAKE_PLUGIN_ID),
        Some("market.tool"),
        None,
    ] {
        assert_eq!(
            native_status_bar_activation(plugin_name),
            NativeStatusBarActivation::ExistingAction
        );
    }
}

#[test]
fn status_bar_collapse_menu_labels_follow_state_and_language() {
    assert_eq!(status_bar_collapse_menu_label(false, "zh-CN"), "折叠子工具");
    assert_eq!(
        status_bar_collapse_menu_label(true, "zh_CN.UTF-8"),
        "展开子工具"
    );
    assert_eq!(
        status_bar_collapse_menu_label(false, "en-US"),
        "Collapse Tool Icons"
    );
    assert_eq!(
        status_bar_collapse_menu_label(true, "en_US.UTF-8"),
        "Expand Tool Icons"
    );
    assert_eq!(status_bar_quit_menu_label("zh-CN"), "退出 Zero 状态栏");
    assert_eq!(status_bar_quit_menu_label("en-US"), "Quit Zero Status Bar");

    assert_eq!(
        primary_status_bar_menu_action("zero.status-bar.toggle-tool-items"),
        Some(PrimaryStatusBarMenuAction::ToggleToolItems)
    );
    assert_eq!(
        primary_status_bar_menu_action("zero.status-bar.quit"),
        Some(PrimaryStatusBarMenuAction::Quit)
    );
    assert_eq!(primary_status_bar_menu_action("unknown"), None);
}

#[test]
fn tool_status_bar_menu_routes_only_its_unique_quit_action() {
    let launch_quit_id = tool_status_bar_quit_menu_id("zero.launch.status");
    let awake_quit_id = tool_status_bar_quit_menu_id("zero.awake.status");

    assert_ne!(launch_quit_id, awake_quit_id);
    assert_eq!(
        tool_status_bar_menu_action(&launch_quit_id, "zero.launch.status"),
        Some(ToolStatusBarMenuAction::Quit)
    );
    assert_eq!(
        tool_status_bar_menu_action(&launch_quit_id, "zero.awake.status"),
        None
    );
    assert_eq!(
        tool_status_bar_menu_action("zero.status-bar.quit", "zero.launch.status"),
        None
    );
}

#[test]
fn status_bar_collapse_updates_toggle_only_group_layout_state() {
    let collapse = status_bar_plugin_items_collapse_update(false);
    assert_eq!(collapse.plugin_items_collapsed, Some(true));
    assert_eq!(collapse.enabled, None);
    assert_eq!(collapse.show_plugin_items_on_launch, None);
    assert_eq!(collapse.visible_plugin_items, None);

    let expand = status_bar_plugin_items_collapse_update(
        collapse.plugin_items_collapsed.expect("collapse state"),
    );
    assert_eq!(expand.plugin_items_collapsed, Some(false));
}

#[test]
fn collapsed_layout_preserves_primary_and_tool_actions() {
    let records = [
        plugin_record("zero.snap", true, Some(20)),
        plugin_record("zero.awake", true, Some(10)),
    ];
    let mut settings = StatusBarSettings::default_for_records(&records);
    settings.plugin_items_collapsed = true;

    let items = normalize_status_bar_items(
        &records,
        &settings,
        false,
        StatusBarSupport::NativeMultiItem,
    );
    assert_eq!(
        status_bar_action_effects(&items[0].action, None),
        vec![StatusBarActionEffect::ToggleTray]
    );
    assert_eq!(
        items[1].action.action_type,
        StatusBarActionType::ToggleCaffeine
    );
    assert_eq!(
        items[2].action.action_type,
        StatusBarActionType::StartScreenshot
    );
}

#[test]
fn status_bar_items_keep_plugin_actions_available_in_fallback() {
    let records = [plugin_record("zero.awake", true, Some(10))];
    for plugin_items_collapsed in [false, true] {
        let mut settings = StatusBarSettings::default_for_records(&records);
        settings.plugin_items_collapsed = plugin_items_collapsed;

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
