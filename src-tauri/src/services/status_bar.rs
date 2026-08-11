use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::image::Image;
#[cfg(target_os = "macos")]
use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::Runtime;
use tauri::{Emitter, Manager};
use tauri_plugin_positioner::on_tray_event;

use crate::brand::{
    canonical_first_party_plugin_id, PRIMARY_STATUS_ITEM_ID, PRODUCT_NAME, ZERO_AWAKE_PLUGIN_ID,
};
use crate::plugins::contracts::{
    PluginContributionStatusBarItem, PluginHealth, PluginRecord, PluginSource, StatusBarAction,
    StatusBarActionType, StatusBarIconId,
};
use crate::plugins::registry::PluginRegistryState;
use crate::services::caffeine::CaffeineState;

pub const MACOS_COMPACT_STATUS_ITEM_LENGTH: f64 = 22.0;
pub const PRIMARY_STATUS_ITEM_COLLAPSE_MENU_ID: &str = "zero.status-bar.toggle-tool-items";
pub const PRIMARY_STATUS_ITEM_QUIT_MENU_ID: &str = "zero.status-bar.quit";
pub const TOOL_STATUS_ITEM_QUIT_MENU_ID_PREFIX: &str = "zero.status-bar.tool.quit:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarSettings {
    pub enabled: bool,
    pub show_plugin_items_on_launch: bool,
    #[serde(default)]
    pub plugin_items_collapsed: bool,
    pub visible_plugin_items: HashMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusBarSettingsInput {
    pub enabled: Option<bool>,
    pub show_plugin_items_on_launch: Option<bool>,
    pub plugin_items_collapsed: Option<bool>,
    pub visible_plugin_items: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStatusBarItemActionInput {
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarItemSnapshot {
    pub id: String,
    pub plugin_name: Option<String>,
    pub title: String,
    pub icon: StatusBarIconId,
    pub base_icon: StatusBarIconId,
    pub active_icon: Option<StatusBarIconId>,
    pub action: StatusBarAction,
    pub order: u32,
    pub native_visible: bool,
    pub source: Option<PluginSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarSupport {
    NativeMultiItem,
    FallbackActionRow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStatusItemRole {
    Primary,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryStatusBarMenuAction {
    ToggleToolItems,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatusBarMenuAction {
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusBarActionEffect {
    ToggleTray,
    ToggleCaffeine,
    StartScreenshotCopy,
    ShowMainWindow,
    EmitOpenPlugin(String),
}

pub struct StatusBarState {
    settings: Mutex<StatusBarSettings>,
    settings_path: Mutex<Option<PathBuf>>,
    native_plugin_item_ids: Mutex<Vec<String>>,
    last_primary_toggle_at: Mutex<Option<std::time::Instant>>,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(StatusBarSettings::default()),
            settings_path: Mutex::new(None),
            native_plugin_item_ids: Mutex::new(Vec::new()),
            last_primary_toggle_at: Mutex::new(None),
        }
    }
}

impl StatusBarState {
    pub fn snapshot(&self, records: &[PluginRecord]) -> Result<StatusBarSettings, String> {
        let settings = self
            .settings
            .lock()
            .map_err(|_| "status bar settings lock is poisoned".to_string())?;

        Ok(settings.clone().normalized_for_records(records))
    }

    pub fn configure_path(
        &self,
        path: PathBuf,
        records: &[PluginRecord],
    ) -> Result<StatusBarSettings, String> {
        let settings = load_status_bar_settings(&path, records)?;
        {
            let mut stored_path = self
                .settings_path
                .lock()
                .map_err(|_| "status bar path lock is poisoned".to_string())?;
            *stored_path = Some(path);
        }
        {
            let mut current = self
                .settings
                .lock()
                .map_err(|_| "status bar settings lock is poisoned".to_string())?;
            *current = settings.clone();
        }

        Ok(settings)
    }

    pub fn configured_path(&self) -> Result<Option<PathBuf>, String> {
        self.settings_path
            .lock()
            .map_err(|_| "status bar path lock is poisoned".to_string())
            .map(|path| path.clone())
    }

    pub fn update(
        &self,
        records: &[PluginRecord],
        input: UpdateStatusBarSettingsInput,
    ) -> Result<StatusBarSettings, String> {
        let mut settings = self
            .settings
            .lock()
            .map_err(|_| "status bar settings lock is poisoned".to_string())?
            .clone()
            .apply(input)
            .normalized_for_records(records);

        let path = self
            .settings_path
            .lock()
            .map_err(|_| "status bar path lock is poisoned".to_string())?
            .clone();

        if let Some(path) = path {
            save_status_bar_settings(&path, &settings)?;
        }

        let mut current = self
            .settings
            .lock()
            .map_err(|_| "status bar settings lock is poisoned".to_string())?;
        std::mem::swap(&mut *current, &mut settings);
        Ok(current.clone())
    }

    fn replace_native_plugin_item_ids(&self, ids: Vec<String>) -> Result<Vec<String>, String> {
        let mut current = self
            .native_plugin_item_ids
            .lock()
            .map_err(|_| "status bar native item lock is poisoned".to_string())?;
        let previous = std::mem::replace(&mut *current, ids);
        Ok(previous)
    }

    fn native_plugin_item_ids(&self) -> Result<Vec<String>, String> {
        self.native_plugin_item_ids
            .lock()
            .map_err(|_| "status bar native item lock is poisoned".to_string())
            .map(|ids| ids.clone())
    }

    fn should_accept_primary_toggle(&self, now: std::time::Instant) -> bool {
        const PRIMARY_TOGGLE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(280);

        let Ok(mut last_toggle_at) = self.last_primary_toggle_at.lock() else {
            return true;
        };

        if last_toggle_at
            .is_some_and(|previous| now.duration_since(previous) < PRIMARY_TOGGLE_DEBOUNCE)
        {
            return false;
        }

        *last_toggle_at = Some(now);
        true
    }
}

impl Default for StatusBarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_plugin_items_on_launch: true,
            plugin_items_collapsed: false,
            visible_plugin_items: HashMap::new(),
        }
    }
}

impl StatusBarSettings {
    pub fn default_for_records(records: &[PluginRecord]) -> Self {
        Self::default().normalized_for_records(records)
    }

    pub fn normalized_for_records(mut self, records: &[PluginRecord]) -> Self {
        self.visible_plugin_items =
            canonicalize_visible_plugin_items(std::mem::take(&mut self.visible_plugin_items));
        for record in records {
            self.visible_plugin_items
                .entry(record.name.clone())
                .or_insert(true);
        }

        self
    }

    fn apply(mut self, input: UpdateStatusBarSettingsInput) -> Self {
        if let Some(enabled) = input.enabled {
            self.enabled = enabled;
        }
        if let Some(show_plugin_items_on_launch) = input.show_plugin_items_on_launch {
            self.show_plugin_items_on_launch = show_plugin_items_on_launch;
        }
        if let Some(plugin_items_collapsed) = input.plugin_items_collapsed {
            self.plugin_items_collapsed = plugin_items_collapsed;
        }
        if let Some(visible_plugin_items) = input.visible_plugin_items {
            for (plugin_name, visible) in visible_plugin_items {
                self.visible_plugin_items.insert(plugin_name, visible);
            }
        }

        self
    }
}

fn canonicalize_visible_plugin_items(values: HashMap<String, bool>) -> HashMap<String, bool> {
    let mut canonical = HashMap::new();

    for (plugin_name, visible) in values
        .iter()
        .filter(|(plugin_name, _)| canonical_first_party_plugin_id(plugin_name) == *plugin_name)
    {
        canonical.insert(plugin_name.clone(), *visible);
    }

    for (plugin_name, visible) in values {
        canonical
            .entry(canonical_first_party_plugin_id(&plugin_name).to_string())
            .or_insert(visible);
    }

    canonical
}

pub fn load_status_bar_settings(
    path: &Path,
    records: &[PluginRecord],
) -> Result<StatusBarSettings, String> {
    if !path.exists() {
        return Ok(StatusBarSettings::default_for_records(records));
    }

    match fs::read_to_string(path)
        .map_err(|error| format!("failed to read status bar settings: {error}"))
        .and_then(|content| {
            serde_json::from_str::<StatusBarSettings>(&content)
                .map_err(|error| format!("failed to parse status bar settings: {error}"))
        }) {
        Ok(settings) => Ok(settings.normalized_for_records(records)),
        Err(_) => Ok(StatusBarSettings::default_for_records(records)),
    }
}

pub fn save_status_bar_settings(path: &Path, settings: &StatusBarSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create status bar settings dir: {error}"))?;
    }

    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("failed to serialize status bar settings: {error}"))?;
    fs::write(path, content)
        .map_err(|error| format!("failed to write status bar settings: {error}"))
}

pub fn normalize_status_bar_items(
    records: &[PluginRecord],
    settings: &StatusBarSettings,
    caffeine_enabled: bool,
    support: StatusBarSupport,
) -> Vec<StatusBarItemSnapshot> {
    let mut items = vec![primary_item()];

    if !settings.enabled || !settings.show_plugin_items_on_launch {
        return items;
    }

    let native_visible = support == StatusBarSupport::NativeMultiItem;
    let mut plugin_items = records
        .iter()
        .enumerate()
        .filter(|(_, record)| is_status_bar_plugin_available(record))
        .filter(|(_, record)| {
            settings
                .visible_plugin_items
                .get(&record.name)
                .copied()
                .unwrap_or(true)
        })
        .flat_map(|(plugin_index, record)| {
            status_bar_contributions(record)
                .into_iter()
                .filter(|item| item.visible_by_default.unwrap_or(true))
                .map(move |item| {
                    status_bar_item_snapshot(
                        record,
                        item,
                        plugin_index,
                        caffeine_enabled,
                        native_visible,
                    )
                })
        })
        .collect::<Vec<_>>();

    plugin_items.sort_by_key(|item| item.order);
    items.extend(plugin_items);
    items
}

pub fn status_bar_settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("status-bar.json")
}

pub fn refresh_status_bar(app: &tauri::AppHandle) -> Result<Vec<StatusBarItemSnapshot>, String> {
    let records = plugin_records(app)?;
    let settings = ensure_status_bar_settings(app, &records)?;
    let caffeine_enabled = app
        .state::<CaffeineState>()
        .snapshot()
        .map(|snapshot| snapshot.enabled)
        .unwrap_or(false);
    let support = current_status_bar_support();
    let items = normalize_status_bar_items(&records, &settings, caffeine_enabled, support);
    sync_native_plugin_items(app, &items, support, settings.plugin_items_collapsed)?;
    Ok(items)
}

pub fn status_bar_items(app: &tauri::AppHandle) -> Result<Vec<StatusBarItemSnapshot>, String> {
    let records = plugin_records(app)?;
    let settings = ensure_status_bar_settings(app, &records)?;
    let caffeine_enabled = app
        .state::<CaffeineState>()
        .snapshot()
        .map(|snapshot| snapshot.enabled)
        .unwrap_or(false);

    Ok(normalize_status_bar_items(
        &records,
        &settings,
        caffeine_enabled,
        current_status_bar_support(),
    ))
}

pub fn ensure_status_bar_settings(
    app: &tauri::AppHandle,
    records: &[PluginRecord],
) -> Result<StatusBarSettings, String> {
    let state = app.state::<StatusBarState>();
    if state.configured_path()?.is_some() {
        return state.snapshot(records);
    }

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve app config dir: {error}"))?;
    state.configure_path(status_bar_settings_path(&config_dir), records)
}

pub fn update_status_bar_settings(
    app: &tauri::AppHandle,
    input: UpdateStatusBarSettingsInput,
) -> Result<StatusBarSettings, String> {
    let records = plugin_records(app)?;
    ensure_status_bar_settings(app, &records)?;
    let settings = app.state::<StatusBarState>().update(&records, input)?;
    let _ = refresh_status_bar(app)?;
    Ok(settings)
}

pub fn toggle_status_bar_plugin_items(app: &tauri::AppHandle) -> Result<StatusBarSettings, String> {
    let records = plugin_records(app)?;
    let current = ensure_status_bar_settings(app, &records)?;
    perform_status_bar_plugin_items_toggle(
        current.plugin_items_collapsed,
        |collapsed| apply_existing_status_bar_layout(app, collapsed),
        |input| app.state::<StatusBarState>().update(&records, input),
    )
}

fn perform_status_bar_plugin_items_toggle(
    current_collapsed: bool,
    apply_layout: impl Fn(bool) -> Result<(), String>,
    persist: impl FnOnce(UpdateStatusBarSettingsInput) -> Result<StatusBarSettings, String>,
) -> Result<StatusBarSettings, String> {
    let next_collapsed = !current_collapsed;
    if let Err(error) = apply_layout(next_collapsed) {
        let _ = apply_layout(current_collapsed);
        return Err(error);
    }

    match persist(status_bar_plugin_items_collapse_update(current_collapsed)) {
        Ok(settings) => Ok(settings),
        Err(error) => {
            let _ = apply_layout(current_collapsed);
            Err(error)
        }
    }
}

pub fn status_bar_plugin_items_collapse_update(
    plugin_items_collapsed: bool,
) -> UpdateStatusBarSettingsInput {
    UpdateStatusBarSettingsInput {
        enabled: None,
        show_plugin_items_on_launch: None,
        plugin_items_collapsed: Some(!plugin_items_collapsed),
        visible_plugin_items: None,
    }
}

pub fn status_bar_collapse_menu_label(
    plugin_items_collapsed: bool,
    language: &str,
) -> &'static str {
    match (language.starts_with("zh"), plugin_items_collapsed) {
        (true, true) => "展开子工具",
        (true, false) => "折叠子工具",
        (false, true) => "Expand Tool Icons",
        (false, false) => "Collapse Tool Icons",
    }
}

pub fn status_bar_quit_menu_label(language: &str) -> &'static str {
    if language.starts_with("zh") {
        "退出 Zero 状态栏"
    } else {
        "Quit Zero Status Bar"
    }
}

pub fn primary_status_bar_menu_action(menu_id: &str) -> Option<PrimaryStatusBarMenuAction> {
    match menu_id {
        PRIMARY_STATUS_ITEM_COLLAPSE_MENU_ID => Some(PrimaryStatusBarMenuAction::ToggleToolItems),
        PRIMARY_STATUS_ITEM_QUIT_MENU_ID => Some(PrimaryStatusBarMenuAction::Quit),
        _ => None,
    }
}

pub fn tool_status_bar_quit_menu_id(item_id: &str) -> String {
    format!("{TOOL_STATUS_ITEM_QUIT_MENU_ID_PREFIX}{item_id}")
}

pub fn tool_status_bar_menu_action(
    menu_id: &str,
    item_id: &str,
) -> Option<ToolStatusBarMenuAction> {
    (menu_id == tool_status_bar_quit_menu_id(item_id)).then_some(ToolStatusBarMenuAction::Quit)
}

pub fn handle_status_bar_action(
    app: tauri::AppHandle,
    action: StatusBarAction,
    plugin_name: Option<String>,
) -> Result<(), String> {
    for effect in status_bar_action_effects(&action, plugin_name.as_deref()) {
        match effect {
            StatusBarActionEffect::ToggleTray => crate::toggle_tray_quick_panel(&app)?,
            StatusBarActionEffect::ToggleCaffeine => {
                toggle_caffeine_from_status_bar(app.clone())?;
            }
            StatusBarActionEffect::StartScreenshotCopy => {
                crate::services::screenshot::start_screenshot_session(app.clone(), "copy".into())?;
            }
            StatusBarActionEffect::ShowMainWindow => {
                crate::commands::app::show_main_window(app.clone())?;
            }
            StatusBarActionEffect::EmitOpenPlugin(plugin_name) => {
                let _ = app.emit("status-bar-open-plugin", plugin_name);
            }
        }
    }

    Ok(())
}

pub fn status_bar_action_effects(
    action: &StatusBarAction,
    plugin_name: Option<&str>,
) -> Vec<StatusBarActionEffect> {
    match action.action_type {
        StatusBarActionType::ToggleTray => vec![StatusBarActionEffect::ToggleTray],
        StatusBarActionType::ToggleCaffeine => vec![StatusBarActionEffect::ToggleCaffeine],
        StatusBarActionType::StartScreenshot => vec![StatusBarActionEffect::StartScreenshotCopy],
        StatusBarActionType::OpenPlugin => {
            let mut effects = vec![StatusBarActionEffect::ShowMainWindow];
            if let Some(plugin_name) = plugin_name {
                effects.push(StatusBarActionEffect::EmitOpenPlugin(plugin_name.into()));
            }
            effects
        }
    }
}

pub fn run_status_bar_item_action(app: &tauri::AppHandle, item_id: &str) -> Result<(), String> {
    let item = status_bar_items(app)?
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| format!("status bar item {item_id} was not found"))?;

    handle_status_bar_action(app.clone(), item.action, item.plugin_name)
}

pub fn native_status_item_creation_order(items: &[StatusBarItemSnapshot]) -> Vec<String> {
    let mut order = items
        .iter()
        .filter(|item| item.plugin_name.is_some() && item.native_visible)
        .rev()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    order.push(PRIMARY_STATUS_ITEM_ID.into());
    order
}

pub fn native_status_item_length(
    support: StatusBarSupport,
    _role: NativeStatusItemRole,
    _plugin_items_collapsed: bool,
) -> Option<f64> {
    if support != StatusBarSupport::NativeMultiItem {
        return None;
    }

    Some(MACOS_COMPACT_STATUS_ITEM_LENGTH)
}

pub fn native_status_item_visible(
    support: StatusBarSupport,
    role: NativeStatusItemRole,
    plugin_items_collapsed: bool,
) -> bool {
    match role {
        NativeStatusItemRole::Primary => true,
        NativeStatusItemRole::Tool => {
            support == StatusBarSupport::NativeMultiItem && !plugin_items_collapsed
        }
    }
}

pub fn native_tool_status_item_visibility_updates(
    tool_item_ids: &[String],
    plugin_items_collapsed: bool,
) -> Vec<(String, bool)> {
    tool_item_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                native_status_item_visible(
                    StatusBarSupport::NativeMultiItem,
                    NativeStatusItemRole::Tool,
                    plugin_items_collapsed,
                ),
            )
        })
        .collect()
}

fn primary_item() -> StatusBarItemSnapshot {
    StatusBarItemSnapshot {
        id: PRIMARY_STATUS_ITEM_ID.into(),
        plugin_name: None,
        title: PRODUCT_NAME.into(),
        icon: StatusBarIconId::Zero,
        base_icon: StatusBarIconId::Zero,
        active_icon: None,
        action: StatusBarAction {
            action_type: StatusBarActionType::ToggleTray,
            command_id: None,
        },
        order: 0,
        native_visible: true,
        source: None,
    }
}

fn plugin_records(app: &tauri::AppHandle) -> Result<Vec<PluginRecord>, String> {
    app.state::<PluginRegistryState>()
        .with_registry(|registry| Ok(registry.records().to_vec()))
}

fn current_status_bar_support() -> StatusBarSupport {
    if cfg!(target_os = "macos") {
        StatusBarSupport::NativeMultiItem
    } else {
        StatusBarSupport::FallbackActionRow
    }
}

fn sync_native_plugin_items(
    app: &tauri::AppHandle,
    items: &[StatusBarItemSnapshot],
    support: StatusBarSupport,
    plugin_items_collapsed: bool,
) -> Result<(), String> {
    let state = app.state::<StatusBarState>();
    let previous_ids = state.replace_native_plugin_item_ids(Vec::new())?;
    for id in previous_ids {
        let _ = app.remove_tray_by_id(&id);
    }
    let _ = app.remove_tray_by_id(PRIMARY_STATUS_ITEM_ID);

    if support != StatusBarSupport::NativeMultiItem {
        sync_primary_status_item(app, support, plugin_items_collapsed)?;
        return Ok(());
    }

    let mut next_ids = Vec::new();
    for item in items
        .iter()
        .filter(|item| item.plugin_name.is_some() && item.native_visible)
        .rev()
    {
        let id = native_tray_id(&item.id);
        let action = item.action.clone();
        let plugin_name = item.plugin_name.clone();
        let icon = status_bar_icon_image(&item.icon)?;
        let title = item.title.clone();
        let app_for_event = app.clone();

        let builder = TrayIconBuilder::with_id(id.clone())
            .icon(icon)
            .icon_as_template(true)
            .tooltip(title)
            .show_menu_on_left_click(false)
            .on_tray_icon_event(move |_tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    let _ = handle_status_bar_action(
                        app_for_event.clone(),
                        action.clone(),
                        plugin_name.clone(),
                    );
                }
            });

        #[cfg(target_os = "macos")]
        let builder = {
            let menu = tool_status_bar_menu(app, &item.id)?;
            let tool_item_id = item.id.clone();

            builder.menu(&menu).on_menu_event(move |app, event| {
                if tool_status_bar_menu_action(event.id().as_ref(), &tool_item_id)
                    == Some(ToolStatusBarMenuAction::Quit)
                {
                    crate::commands::app::quit_app(app.clone());
                }
            })
        };

        let tray = builder
            .build(app)
            .map_err(|error| format!("failed to create status bar item: {error}"))?;
        apply_native_status_item_visibility(
            &tray,
            native_status_item_visible(support, NativeStatusItemRole::Tool, plugin_items_collapsed),
        )?;
        if !plugin_items_collapsed {
            apply_native_status_item_length(
                &tray,
                native_status_item_length(
                    support,
                    NativeStatusItemRole::Tool,
                    plugin_items_collapsed,
                ),
            )?;
        }
        next_ids.push(id);
    }

    let _ = state.replace_native_plugin_item_ids(next_ids)?;
    sync_primary_status_item(app, support, plugin_items_collapsed)?;
    Ok(())
}

fn native_tray_id(item_id: &str) -> String {
    format!("status-bar:{item_id}")
}

fn apply_existing_status_bar_layout(
    app: &tauri::AppHandle,
    plugin_items_collapsed: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let support = StatusBarSupport::NativeMultiItem;
        let primary = app
            .tray_by_id(PRIMARY_STATUS_ITEM_ID)
            .ok_or_else(|| "primary status bar item is unavailable".to_string())?;

        let tool_item_ids = app.state::<StatusBarState>().native_plugin_item_ids()?;
        for (id, visible) in
            native_tool_status_item_visibility_updates(&tool_item_ids, plugin_items_collapsed)
        {
            let tray = app
                .tray_by_id(&id)
                .ok_or_else(|| format!("status bar tool item is unavailable: {id}"))?;
            apply_native_status_item_visibility(&tray, visible)?;
            if visible {
                apply_native_status_item_length(
                    &tray,
                    native_status_item_length(
                        support,
                        NativeStatusItemRole::Tool,
                        plugin_items_collapsed,
                    ),
                )?;
            }
        }

        apply_native_status_item_length(
            &primary,
            native_status_item_length(
                support,
                NativeStatusItemRole::Primary,
                plugin_items_collapsed,
            ),
        )?;

        primary
            .set_menu(Some(primary_status_bar_menu(app, plugin_items_collapsed)?))
            .map_err(|error| format!("failed to update primary status bar menu: {error}"))?;
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app, plugin_items_collapsed);

    Ok(())
}

fn apply_native_status_item_visibility<R: Runtime>(
    tray: &TrayIcon<R>,
    visible: bool,
) -> Result<(), String> {
    tray.set_visible(visible)
        .map_err(|error| format!("failed to update status bar item visibility: {error}"))
}

#[cfg(target_os = "macos")]
fn primary_status_bar_menu<R: Runtime>(
    app: &tauri::AppHandle<R>,
    plugin_items_collapsed: bool,
) -> Result<tauri::menu::Menu<R>, String> {
    let language = crate::services::quick_launcher::system_language();
    MenuBuilder::new(app)
        .text(
            PRIMARY_STATUS_ITEM_COLLAPSE_MENU_ID,
            status_bar_collapse_menu_label(plugin_items_collapsed, &language),
        )
        .text(
            PRIMARY_STATUS_ITEM_QUIT_MENU_ID,
            status_bar_quit_menu_label(&language),
        )
        .build()
        .map_err(|error| format!("failed to create primary status bar menu: {error}"))
}

#[cfg(target_os = "macos")]
fn tool_status_bar_menu<R: Runtime>(
    app: &tauri::AppHandle<R>,
    item_id: &str,
) -> Result<tauri::menu::Menu<R>, String> {
    let language = crate::services::quick_launcher::system_language();
    MenuBuilder::new(app)
        .text(
            tool_status_bar_quit_menu_id(item_id),
            status_bar_quit_menu_label(&language),
        )
        .build()
        .map_err(|error| format!("failed to create tool status bar menu: {error}"))
}

fn sync_primary_status_item(
    app: &tauri::AppHandle,
    support: StatusBarSupport,
    plugin_items_collapsed: bool,
) -> Result<(), String> {
    let builder = TrayIconBuilder::with_id(PRIMARY_STATUS_ITEM_ID)
        .icon(status_bar_icon_image(&StatusBarIconId::Zero)?)
        .icon_as_template(true)
        .tooltip(PRODUCT_NAME)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |tray, event| {
            let app_handle = tray.app_handle();
            on_tray_event(&app_handle, &event);

            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                let state = app_handle.state::<StatusBarState>();
                if !state.should_accept_primary_toggle(std::time::Instant::now()) {
                    return;
                }

                let _ = crate::toggle_tray_quick_panel(tray.app_handle());
            }
        });

    #[cfg(target_os = "macos")]
    let builder = {
        let menu = primary_status_bar_menu(app, plugin_items_collapsed)?;

        builder.menu(&menu).on_menu_event(|app, event| {
            match primary_status_bar_menu_action(event.id().as_ref()) {
                Some(PrimaryStatusBarMenuAction::ToggleToolItems) => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = toggle_status_bar_plugin_items(&app);
                    });
                }
                Some(PrimaryStatusBarMenuAction::Quit) => {
                    crate::commands::app::quit_app(app.clone());
                }
                None => {}
            }
        })
    };

    let tray = builder
        .build(app)
        .map_err(|error| format!("failed to create primary status bar item: {error}"))?;
    apply_native_status_item_length(
        &tray,
        native_status_item_length(
            support,
            NativeStatusItemRole::Primary,
            plugin_items_collapsed,
        ),
    )?;

    Ok(())
}

fn apply_native_status_item_length<R: Runtime>(
    tray: &TrayIcon<R>,
    length: Option<f64>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if let Some(length) = length {
        return tray
            .with_inner_tray_icon(move |inner| {
                let status_item = inner
                    .ns_status_item()
                    .ok_or_else(|| "macOS status item is unavailable".to_string())?;
                status_item.setLength(length);
                Ok(())
            })
            .map_err(|error| format!("failed to access macOS status item: {error}"))?;
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (tray, length);

    Ok(())
}

fn toggle_caffeine_from_status_bar(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<CaffeineState>();
    let current = state.snapshot()?;
    let transition = state.set_enabled(!current.enabled, None)?;

    if let Some(expiry) = transition.expiry {
        crate::commands::caffeine::schedule_expiry(app.clone(), expiry);
    }

    let _ = refresh_status_bar(&app)?;
    Ok(())
}

fn status_bar_icon_image(icon: &StatusBarIconId) -> Result<Image<'static>, String> {
    let decoded = image::load_from_memory(status_bar_icon_png_bytes(icon))
        .map_err(|error| format!("failed to decode status bar icon {icon:?}: {error}"))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    Ok(Image::new_owned(decoded.into_raw(), width, height))
}

pub fn status_bar_icon_png_bytes(icon: &StatusBarIconId) -> &'static [u8] {
    match icon {
        StatusBarIconId::Zero => include_bytes!("../../icons/tray/zero.png"),
        StatusBarIconId::Launch => include_bytes!("../../icons/tray/zero-launch.png"),
        StatusBarIconId::CaffeineEmpty => include_bytes!("../../icons/tray/zero-awake.png"),
        StatusBarIconId::CaffeineFull => {
            include_bytes!("../../icons/tray/zero-awake-active.png")
        }
        StatusBarIconId::Screenshot => include_bytes!("../../icons/tray/zero-snap.png"),
        StatusBarIconId::Paper => include_bytes!("../../icons/tray/zero-paper.png"),
        StatusBarIconId::Extension => include_bytes!("../../icons/tray/extension.png"),
    }
}

fn status_bar_item_snapshot(
    record: &PluginRecord,
    item: PluginContributionStatusBarItem,
    plugin_index: usize,
    caffeine_enabled: bool,
    native_visible: bool,
) -> StatusBarItemSnapshot {
    let icon = if record.name == ZERO_AWAKE_PLUGIN_ID && caffeine_enabled {
        item.active_icon.clone().unwrap_or(item.icon.clone())
    } else {
        item.icon.clone()
    };

    StatusBarItemSnapshot {
        id: item.id,
        plugin_name: Some(record.name.clone()),
        title: item.title,
        icon,
        base_icon: item.icon,
        active_icon: item.active_icon,
        action: item.action,
        order: normalize_order(item.order, plugin_index),
        native_visible,
        source: Some(record.source.clone()),
    }
}

fn is_status_bar_plugin_available(record: &PluginRecord) -> bool {
    record.enabled
        && record.health != PluginHealth::Disabled
        && record.health != PluginHealth::Incompatible
}

fn status_bar_contributions(record: &PluginRecord) -> Vec<PluginContributionStatusBarItem> {
    record
        .manifest
        .contributes
        .as_ref()
        .and_then(|contributions| contributions.status_bar_items.clone())
        .unwrap_or_else(|| vec![default_status_bar_item(record)])
        .into_iter()
        .filter(is_supported_status_bar_contribution)
        .collect()
}

fn default_status_bar_item(record: &PluginRecord) -> PluginContributionStatusBarItem {
    PluginContributionStatusBarItem {
        id: format!("{}.status", record.name),
        title: record
            .manifest
            .display_name
            .clone()
            .unwrap_or_else(|| record.name.clone()),
        icon: StatusBarIconId::Extension,
        active_icon: None,
        action: StatusBarAction {
            action_type: StatusBarActionType::OpenPlugin,
            command_id: None,
        },
        order: Some(1000),
        visible_by_default: Some(true),
    }
}

fn is_supported_status_bar_contribution(item: &PluginContributionStatusBarItem) -> bool {
    matches!(
        item.action.action_type,
        StatusBarActionType::ToggleTray
            | StatusBarActionType::ToggleCaffeine
            | StatusBarActionType::StartScreenshot
            | StatusBarActionType::OpenPlugin
    )
}

fn normalize_order(order: Option<u32>, plugin_index: usize) -> u32 {
    order
        .unwrap_or(1000)
        .saturating_mul(1000)
        .saturating_add(u32::try_from(plugin_index).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[test]
    fn collapse_toggle_applies_existing_layout_before_persisting() {
        let applied_layouts = RefCell::new(Vec::new());
        let persisted = Cell::new(false);

        let settings = perform_status_bar_plugin_items_toggle(
            false,
            |collapsed| {
                applied_layouts.borrow_mut().push(collapsed);
                Ok(())
            },
            |input| {
                persisted.set(true);
                assert_eq!(input.plugin_items_collapsed, Some(true));
                Ok(StatusBarSettings {
                    plugin_items_collapsed: true,
                    ..StatusBarSettings::default()
                })
            },
        )
        .expect("collapse transition should succeed");

        assert_eq!(*applied_layouts.borrow(), vec![true]);
        assert!(persisted.get());
        assert!(settings.plugin_items_collapsed);
    }

    #[test]
    fn collapse_toggle_rolls_back_existing_layout_when_persistence_fails() {
        let applied_layouts = RefCell::new(Vec::new());

        let error = perform_status_bar_plugin_items_toggle(
            false,
            |collapsed| {
                applied_layouts.borrow_mut().push(collapsed);
                Ok(())
            },
            |_| Err("cannot persist status bar settings".to_string()),
        )
        .expect_err("failed persistence should reject the transition");

        assert_eq!(error, "cannot persist status bar settings");
        assert_eq!(*applied_layouts.borrow(), vec![true, false]);
    }
}
