pub mod cache;
pub mod catalog;
pub mod contracts;
pub mod model;
pub mod platform;
pub mod search;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::Manager;

use self::contracts::{
    launcher_error, QuickLauncherActivateInput, QuickLauncherActivationResult,
    QuickLauncherDiagnostic, QuickLauncherError, QuickLauncherIconBatchInput,
    QuickLauncherIconBatchResult, QuickLauncherIconInput, QuickLauncherIconResult,
    QuickLauncherIndexSnapshot, QuickLauncherIndexSource, QuickLauncherItemKind,
    QuickLauncherPlatformSupport, QuickLauncherRunningSnapshot, QuickLauncherSearchInput,
    QuickLauncherSearchResult,
};
use self::model::{current_platform_key, IndexedItem, LaunchTarget, UsageMap};
use self::search::search_items_thread_local;

pub const RUNNING_CACHE_TTL: Duration = Duration::from_secs(2);
const MAX_ICON_BATCH_ITEMS: usize = 24;
const MAX_ICON_CACHE_ITEMS: usize = 128;
const MAX_ICON_DATA_URL_BYTES: usize = 512 * 1024;
const MAX_ICON_CACHE_BYTES: usize = 8 * 1024 * 1024;
const MAX_DIAGNOSTICS: usize = 64;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 512;

struct LauncherIndex {
    revision: u64,
    source: QuickLauncherIndexSource,
    refreshing: bool,
    items: Arc<Vec<IndexedItem>>,
    usage: Arc<UsageMap>,
    last_updated_at: Option<u64>,
    platform_support: QuickLauncherPlatformSupport,
    diagnostics: Vec<QuickLauncherDiagnostic>,
}

#[derive(Default)]
struct RunningCache {
    captured_at: Option<Instant>,
    revision: u64,
    values: Arc<HashMap<String, contracts::QuickLauncherRunningState>>,
}

#[derive(Default)]
struct IconCache {
    entries: HashMap<String, CachedIcon>,
    total_bytes: usize,
    sequence: u64,
}

struct CachedIcon {
    data_url: Option<String>,
    bytes: usize,
    last_used: u64,
}

impl IconCache {
    fn get(&mut self, key: &str) -> Option<Option<String>> {
        let entry = self.entries.get_mut(key)?;
        self.sequence = self.sequence.saturating_add(1);
        entry.last_used = self.sequence;
        Some(entry.data_url.clone())
    }

    fn insert(&mut self, key: String, data_url: Option<String>) {
        let bytes = data_url.as_ref().map_or(0, String::len);
        if bytes > MAX_ICON_DATA_URL_BYTES {
            return;
        }
        if let Some(previous) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(previous.bytes);
        }
        while !self.entries.is_empty()
            && (self.entries.len() >= MAX_ICON_CACHE_ITEMS
                || self.total_bytes.saturating_add(bytes) > MAX_ICON_CACHE_BYTES)
        {
            let oldest = self
                .entries
                .iter()
                .min_by(|(left_key, left), (right_key, right)| {
                    (left.last_used, left_key.as_str()).cmp(&(right.last_used, right_key.as_str()))
                })
                .map(|(oldest_key, _)| oldest_key.clone());
            if let Some(oldest) = oldest {
                if let Some(removed) = self.entries.remove(&oldest) {
                    self.total_bytes = self.total_bytes.saturating_sub(removed.bytes);
                }
            }
        }
        self.sequence = self.sequence.saturating_add(1);
        self.total_bytes = self.total_bytes.saturating_add(bytes);
        self.entries.insert(
            key,
            CachedIcon {
                data_url,
                bytes,
                last_used: self.sequence,
            },
        );
    }
}

pub struct QuickLauncherState {
    root: std::path::PathBuf,
    index: RwLock<LauncherIndex>,
    refresh_gate: Mutex<()>,
    watcher: Mutex<Option<RecommendedWatcher>>,
    running_cache: Mutex<RunningCache>,
    running_refresh_gate: Mutex<()>,
    icon_cache: Mutex<IconCache>,
    icon_load_gate: Mutex<()>,
    usage_write_gate: Mutex<()>,
    initialization: (Mutex<LauncherInitialization>, Condvar),
    enabled: AtomicBool,
    generation: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LauncherInitialization {
    Idle,
    Running,
    Ready,
}

impl Default for QuickLauncherState {
    fn default() -> Self {
        Self::new(cache::default_root())
    }
}

impl QuickLauncherState {
    pub fn new(root: std::path::PathBuf) -> Self {
        let platform_support = if matches!(current_platform_key(), "macos" | "windows") {
            QuickLauncherPlatformSupport::Supported
        } else {
            QuickLauncherPlatformSupport::Unsupported
        };
        Self {
            root,
            index: RwLock::new(LauncherIndex {
                revision: 0,
                source: QuickLauncherIndexSource::Empty,
                refreshing: false,
                items: Arc::new(Vec::new()),
                usage: Arc::new(UsageMap::new()),
                last_updated_at: None,
                platform_support,
                diagnostics: Vec::new(),
            }),
            refresh_gate: Mutex::new(()),
            watcher: Mutex::new(None),
            running_cache: Mutex::new(RunningCache::default()),
            running_refresh_gate: Mutex::new(()),
            icon_cache: Mutex::new(IconCache::default()),
            icon_load_gate: Mutex::new(()),
            usage_write_gate: Mutex::new(()),
            initialization: (Mutex::new(LauncherInitialization::Idle), Condvar::new()),
            enabled: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let previous = self.enabled.swap(enabled, Ordering::AcqRel);
        if previous == enabled {
            return;
        }
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut initialization) = self.initialization.0.lock() {
            *initialization = LauncherInitialization::Idle;
            self.initialization.1.notify_all();
        }
        if !enabled {
            if let Ok(mut watcher) = self.watcher.lock() {
                watcher.take();
            }
            if let Ok(mut index) = self.index.write() {
                index.refreshing = false;
            }
            if let Ok(mut running) = self.running_cache.lock() {
                *running = RunningCache::default();
            }
            if let Ok(mut icons) = self.icon_cache.lock() {
                *icons = IconCache::default();
            }
        }
    }

    pub fn initialize(&self, app: tauri::AppHandle) -> Result<(), QuickLauncherError> {
        if !self.enabled.load(Ordering::Acquire) {
            return Err(disabled_error());
        }
        let generation = self.generation.load(Ordering::Acquire);
        self.initialize_once(generation, || self.initialize_generation(app, generation))
    }

    fn initialize_once(
        &self,
        generation: u64,
        operation: impl FnOnce() -> Result<(), QuickLauncherError>,
    ) -> Result<(), QuickLauncherError> {
        let mut initialization = self
            .initialization
            .0
            .lock()
            .map_err(|_| lock_error("lock initialization"))?;
        loop {
            match *initialization {
                LauncherInitialization::Ready => return Ok(()),
                LauncherInitialization::Running => {
                    initialization = self
                        .initialization
                        .1
                        .wait(initialization)
                        .map_err(|_| lock_error("wait for initialization"))?;
                    if !self.is_current_generation(generation) {
                        return Err(disabled_error());
                    }
                }
                LauncherInitialization::Idle => {
                    *initialization = LauncherInitialization::Running;
                    break;
                }
            }
        }
        drop(initialization);

        let result = operation();
        if let Ok(mut initialization) = self.initialization.0.lock() {
            *initialization = if result.is_ok() && self.is_current_generation(generation) {
                LauncherInitialization::Ready
            } else {
                LauncherInitialization::Idle
            };
            self.initialization.1.notify_all();
        }
        result
    }

    fn initialize_generation(
        &self,
        app: tauri::AppHandle,
        generation: u64,
    ) -> Result<(), QuickLauncherError> {
        let mut diagnostics = Vec::new();
        let (items, last_updated_at, source) = match cache::load_index(&self.root) {
            Ok(Some((items, updated_at))) => {
                (items, Some(updated_at), QuickLauncherIndexSource::Cache)
            }
            Ok(None) => (Vec::new(), None, QuickLauncherIndexSource::Empty),
            Err(error) => {
                diagnostics.push(QuickLauncherDiagnostic {
                    code: "launcher.cache_rebuilt".into(),
                    message: error,
                });
                (Vec::new(), None, QuickLauncherIndexSource::Empty)
            }
        };
        let usage = match cache::load_usage(&self.root) {
            Ok(usage) => usage,
            Err(error) => {
                diagnostics.push(QuickLauncherDiagnostic {
                    code: "launcher.usage_rebuilt".into(),
                    message: error,
                });
                UsageMap::new()
            }
        };
        if !self.is_current_generation(generation) {
            return Err(disabled_error());
        }
        {
            let mut index = self
                .index
                .write()
                .map_err(|_| lock_error("publish cached index"))?;
            index.revision = u64::from(!items.is_empty());
            index.source = source;
            index.items = Arc::new(items);
            index.usage = Arc::new(usage);
            index.last_updated_at = last_updated_at;
            index.diagnostics = diagnostics;
        }
        if let Err(error) = self.start_watcher(app, generation) {
            self.add_diagnostic("launcher.watcher_unavailable", error);
        }
        self.refresh_generation(&system_language(), generation)?;
        Ok(())
    }

    fn is_current_generation(&self, generation: u64) -> bool {
        self.enabled.load(Ordering::Acquire)
            && self.generation.load(Ordering::Acquire) == generation
    }

    pub fn snapshot(&self) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
        let index = self.index.read().map_err(|_| lock_error("read index"))?;
        Ok(snapshot_from_index(&index))
    }

    pub fn refresh(
        &self,
        language: &str,
    ) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
        let generation = self.generation.load(Ordering::Acquire);
        if !self.is_current_generation(generation) {
            return Err(disabled_error());
        }
        self.refresh_generation(language, generation)
    }

    fn refresh_generation(
        &self,
        language: &str,
        generation: u64,
    ) -> Result<QuickLauncherIndexSnapshot, QuickLauncherError> {
        let _refresh = match self.refresh_gate.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return self.snapshot(),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(lock_error("start refresh"));
            }
        };
        {
            let mut index = self.index.write().map_err(|_| lock_error("mark refresh"))?;
            index.refreshing = true;
        }

        let mut scan = platform::scan();
        scan.items.extend(catalog::system_setting_items(language));
        let running = platform::probe_running(&scan.items);
        for item in &mut scan.items {
            if let Some(state) = running.get(&item.id) {
                item.running = *state;
            }
        }
        scan.items.sort_by(|left, right| left.id.cmp(&right.id));
        scan.items.dedup_by(|left, right| left.id == right.id);
        if let Ok(mut runtime) = self.running_cache.lock() {
            runtime.captured_at = Some(Instant::now());
            runtime.revision = runtime.revision.saturating_add(1);
            runtime.values = Arc::new(
                scan.items
                    .iter()
                    .map(|item| (item.id.clone(), item.running))
                    .collect(),
            );
        }
        let updated_at = now_timestamp();
        let save_error = cache::save_index(&self.root, &scan.items, updated_at).err();

        if !self.is_current_generation(generation) {
            return self.snapshot();
        }

        let mut index = self
            .index
            .write()
            .map_err(|_| lock_error("publish refresh"))?;
        index.revision = index.revision.saturating_add(1).max(1);
        index.source = QuickLauncherIndexSource::Scan;
        index.refreshing = false;
        index.items = Arc::new(scan.items);
        index.last_updated_at = Some(updated_at);
        index.diagnostics = bounded_diagnostics(scan.diagnostics);
        if let Some(error) = save_error {
            push_bounded_diagnostic(&mut index.diagnostics, "launcher.cache_write_failed", error);
            index.platform_support = QuickLauncherPlatformSupport::Degraded;
        } else if current_platform_key() == "unsupported" {
            index.platform_support = QuickLauncherPlatformSupport::Unsupported;
        } else if index.diagnostics.is_empty() {
            index.platform_support = QuickLauncherPlatformSupport::Supported;
        } else {
            index.platform_support = QuickLauncherPlatformSupport::Degraded;
        }
        Ok(snapshot_from_index(&index))
    }

    pub fn search(
        &self,
        input: QuickLauncherSearchInput,
    ) -> Result<QuickLauncherSearchResult, QuickLauncherError> {
        let (revision, items, usage) = {
            let index = self
                .index
                .read()
                .map_err(|_| lock_error("read search index"))?;
            (
                index.revision,
                Arc::clone(&index.items),
                Arc::clone(&index.usage),
            )
        };
        let running = self.running_values()?;
        search_items_thread_local(revision, &items, &usage, &running, input)
    }

    pub fn icon(
        &self,
        input: QuickLauncherIconInput,
    ) -> Result<QuickLauncherIconResult, QuickLauncherError> {
        let item = self.resolve_icon_item(&input)?;
        self.load_icon(item)
    }

    pub fn icons(
        &self,
        input: QuickLauncherIconBatchInput,
    ) -> Result<QuickLauncherIconBatchResult, QuickLauncherError> {
        if input.items.is_empty() || input.items.len() > MAX_ICON_BATCH_ITEMS {
            return Err(launcher_error(
                "launcher.icon_batch",
                "launcher.icon_batch_invalid",
                format!("Icon batch must contain between 1 and {MAX_ICON_BATCH_ITEMS} items."),
                false,
            ));
        }
        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::with_capacity(input.items.len());
        for icon in input.items {
            if seen.insert(icon.item_id.clone()) {
                results.push(self.icon(icon)?);
            }
        }
        Ok(QuickLauncherIconBatchResult { results })
    }

    fn resolve_icon_item(
        &self,
        input: &QuickLauncherIconInput,
    ) -> Result<IndexedItem, QuickLauncherError> {
        let index = self
            .index
            .read()
            .map_err(|_| lock_error("read icon index"))?;
        let item = index
            .items
            .iter()
            .find(|item| item.id == input.item_id)
            .ok_or_else(stale_item_error)?;
        if input
            .icon_key
            .as_deref()
            .is_some_and(|expected| item.icon_key.as_deref() != Some(expected))
        {
            return Err(stale_item_error());
        }
        Ok(item.clone())
    }

    fn load_icon(&self, item: IndexedItem) -> Result<QuickLauncherIconResult, QuickLauncherError> {
        let cache_key = item.icon_key.clone().unwrap_or_else(|| item.id.clone());
        if let Some(data_url) = self.cached_icon(&cache_key)? {
            return Ok(QuickLauncherIconResult {
                item_id: item.id,
                data_url,
            });
        }

        // One native load at a time bounds platform image decoding across concurrent surfaces.
        let _load = self
            .icon_load_gate
            .lock()
            .map_err(|_| lock_error("schedule icon load"))?;
        if let Some(data_url) = self.cached_icon(&cache_key)? {
            return Ok(QuickLauncherIconResult {
                item_id: item.id,
                data_url,
            });
        }
        let data_url = platform::load_icon(&item)?.and_then(|bytes| {
            let encoded = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(bytes));
            (encoded.len() <= MAX_ICON_DATA_URL_BYTES).then_some(encoded)
        });
        self.icon_cache
            .lock()
            .map_err(|_| lock_error("write icon cache"))?
            .insert(cache_key, data_url.clone());
        Ok(QuickLauncherIconResult {
            item_id: item.id,
            data_url,
        })
    }

    fn cached_icon(&self, cache_key: &str) -> Result<Option<Option<String>>, QuickLauncherError> {
        Ok(self
            .icon_cache
            .lock()
            .map_err(|_| lock_error("read icon cache"))?
            .get(cache_key))
    }

    pub fn activate(
        &self,
        input: QuickLauncherActivateInput,
    ) -> Result<QuickLauncherActivationResult, QuickLauncherError> {
        let item = {
            let index = self
                .index
                .read()
                .map_err(|_| lock_error("read activation index"))?;
            if index.revision != input.revision {
                return Err(stale_item_error());
            }
            index
                .items
                .iter()
                .find(|item| item.id == input.item_id)
                .cloned()
                .ok_or_else(stale_item_error)?
        };
        let action = match (&item.kind, &item.target) {
            (QuickLauncherItemKind::Application, LaunchTarget::Application { .. }) => {
                platform::activate(&item)?
            }
            (QuickLauncherItemKind::SystemSetting, LaunchTarget::SystemSetting { uri }) => {
                platform::open_setting(uri)?
            }
            _ => {
                return Err(launcher_error(
                    "launcher.activate",
                    "launcher.item_kind_invalid",
                    "Launcher item kind does not match its private target.",
                    false,
                ));
            }
        };
        let activated_at = now_timestamp();
        let _usage_write = self
            .usage_write_gate
            .lock()
            .map_err(|_| lock_error("serialize usage persistence"))?;
        let mut usage = self
            .index
            .read()
            .map_err(|_| lock_error("read usage"))?
            .usage
            .as_ref()
            .clone();
        let fallback_count = usage
            .get(&item.id)
            .map(|entry| entry.count.saturating_add(1))
            .unwrap_or(1);
        let persisted =
            cache::record_successful_use(&self.root, &mut usage, &item.id, activated_at);
        let mut index = self
            .index
            .write()
            .map_err(|_| lock_error("publish usage"))?;
        let usage_count = match persisted {
            Ok(count) => {
                index.usage = Arc::new(usage);
                count
            }
            Err(error) => {
                push_bounded_diagnostic(
                    &mut index.diagnostics,
                    "launcher.usage_write_failed",
                    error,
                );
                fallback_count
            }
        };
        Ok(QuickLauncherActivationResult {
            item_id: item.id,
            action,
            usage_count,
            activated_at,
        })
    }

    pub fn add_diagnostic(&self, code: &str, message: impl Into<String>) {
        if let Ok(mut index) = self.index.write() {
            push_bounded_diagnostic(&mut index.diagnostics, code, message.into());
            if index.platform_support == QuickLauncherPlatformSupport::Supported {
                index.platform_support = QuickLauncherPlatformSupport::Degraded;
            }
        }
    }

    fn running_values(
        &self,
    ) -> Result<Arc<HashMap<String, contracts::QuickLauncherRunningState>>, QuickLauncherError>
    {
        let cache = self
            .running_cache
            .lock()
            .map_err(|_| lock_error("read running-state cache"))?;
        Ok(Arc::clone(&cache.values))
    }

    pub fn refresh_running_states(
        &self,
    ) -> Result<QuickLauncherRunningSnapshot, QuickLauncherError> {
        self.refresh_running_states_with(platform::probe_running)
    }

    fn refresh_running_states_with(
        &self,
        probe: impl FnOnce(&[IndexedItem]) -> HashMap<String, contracts::QuickLauncherRunningState>,
    ) -> Result<QuickLauncherRunningSnapshot, QuickLauncherError> {
        let _refresh = self
            .running_refresh_gate
            .lock()
            .map_err(|_| lock_error("schedule running-state refresh"))?;
        let index_revision = self
            .index
            .read()
            .map_err(|_| lock_error("read running index revision"))?
            .revision;
        {
            let cache = self
                .running_cache
                .lock()
                .map_err(|_| lock_error("read running-state lifetime"))?;
            if let Some(captured_at) = cache.captured_at {
                let elapsed = captured_at.elapsed();
                if elapsed < RUNNING_CACHE_TTL {
                    let remaining = RUNNING_CACHE_TTL.saturating_sub(elapsed);
                    return Ok(QuickLauncherRunningSnapshot {
                        index_revision,
                        running_revision: cache.revision,
                        expires_at_ms: now_timestamp_ms()
                            .saturating_add(remaining.as_millis().min(u64::MAX as u128) as u64),
                    });
                }
            }
        }
        let (index_revision, items) = {
            let index = self
                .index
                .read()
                .map_err(|_| lock_error("read running index"))?;
            (index.revision, Arc::clone(&index.items))
        };
        let values = probe(&items);
        let mut cache = self
            .running_cache
            .lock()
            .map_err(|_| lock_error("publish running-state cache"))?;
        cache.captured_at = Some(Instant::now());
        cache.revision = cache.revision.saturating_add(1);
        cache.values = Arc::new(values);
        Ok(QuickLauncherRunningSnapshot {
            index_revision,
            running_revision: cache.revision,
            expires_at_ms: now_timestamp_ms()
                .saturating_add(RUNNING_CACHE_TTL.as_millis().min(u64::MAX as u128) as u64),
        })
    }

    fn start_watcher(&self, app: tauri::AppHandle, generation: u64) -> Result<(), String> {
        let roots = platform::application_roots()
            .into_iter()
            .filter(|root| root.exists())
            .collect::<Vec<_>>();
        if roots.is_empty() {
            self.add_diagnostic(
                "launcher.watcher_unavailable",
                "No supported application directory is available to watch.",
            );
            return Ok(());
        }
        let queued = Arc::new(AtomicBool::new(false));
        let callback_queued = queued.clone();
        let callback_app = app.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Err(error) = event {
                    let state = callback_app.state::<QuickLauncherState>();
                    if state.is_current_generation(generation) {
                        state.add_diagnostic("launcher.watcher_event_failed", error.to_string());
                    }
                    return;
                }
                if callback_queued.swap(true, Ordering::AcqRel) {
                    return;
                }
                let delayed_app = callback_app.clone();
                let delayed_queued = callback_queued.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(500));
                    let state = delayed_app.state::<QuickLauncherState>();
                    let _ = state.refresh_generation(&system_language(), generation);
                    delayed_queued.store(false, Ordering::Release);
                });
            })
            .map_err(|error| format!("failed to create launcher watcher: {error}"))?;
        for root in roots {
            if let Err(error) = watcher.watch(&root, RecursiveMode::Recursive) {
                self.add_diagnostic(
                    "launcher.watcher_root_failed",
                    format!("Could not watch {}: {error}", root.display()),
                );
            }
        }
        let mut slot = self
            .watcher
            .lock()
            .map_err(|_| "launcher watcher lock is poisoned".to_string())?;
        if self.is_current_generation(generation) {
            *slot = Some(watcher);
        }
        Ok(())
    }
}

pub fn system_language() -> String {
    std::env::var("LANG").unwrap_or_else(|_| "en-US".into())
}

fn snapshot_from_index(index: &LauncherIndex) -> QuickLauncherIndexSnapshot {
    QuickLauncherIndexSnapshot {
        revision: index.revision,
        source: index.source,
        refreshing: index.refreshing,
        item_count: index.items.len(),
        last_updated_at: index.last_updated_at,
        platform_support: index.platform_support,
        diagnostics: index.diagnostics.clone(),
    }
}

fn bounded_diagnostics(
    mut diagnostics: Vec<QuickLauncherDiagnostic>,
) -> Vec<QuickLauncherDiagnostic> {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        diagnostics.truncate(MAX_DIAGNOSTICS);
    }
    for diagnostic in &mut diagnostics {
        diagnostic.message = bounded_diagnostic_message(&diagnostic.message);
    }
    diagnostics
}

fn push_bounded_diagnostic(
    diagnostics: &mut Vec<QuickLauncherDiagnostic>,
    code: &str,
    message: impl AsRef<str>,
) {
    if diagnostics.len() >= MAX_DIAGNOSTICS {
        diagnostics.remove(0);
    }
    diagnostics.push(QuickLauncherDiagnostic {
        code: code.into(),
        message: bounded_diagnostic_message(message.as_ref()),
    });
}

fn bounded_diagnostic_message(message: &str) -> String {
    let mut end = message.len().min(MAX_DIAGNOSTIC_MESSAGE_BYTES);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_string()
}

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn now_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

fn lock_error(action: &str) -> QuickLauncherError {
    launcher_error(
        "launcher.state",
        "launcher.state_unavailable",
        format!("Could not {action} because launcher state is unavailable."),
        true,
    )
}

fn stale_item_error() -> QuickLauncherError {
    launcher_error(
        "launcher.activate",
        "launcher.item_stale",
        "The selected launcher item is stale or no longer installed.",
        true,
    )
}

fn disabled_error() -> QuickLauncherError {
    launcher_error(
        "launcher.lifecycle",
        "launcher.disabled",
        "Zero Launch is disabled.",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::contracts::QuickLauncherRunningState;
    use super::*;

    #[test]
    fn construction_is_inert_until_enabled_initialization() {
        let root = std::env::temp_dir().join(format!(
            "zero-launch-inert-{}-{}",
            std::process::id(),
            now_timestamp()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("apps_cache.json"), "not-json").unwrap();
        let state = QuickLauncherState::new(root.clone());

        let snapshot = state.snapshot().unwrap();
        assert_eq!(snapshot.source, QuickLauncherIndexSource::Empty);
        assert!(snapshot.diagnostics.is_empty());
        assert_eq!(
            *state.initialization.0.lock().unwrap(),
            LauncherInitialization::Idle
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disabling_advances_generation_and_resets_lifecycle() {
        let state = QuickLauncherState::new(std::env::temp_dir().join("zero-launch-lifecycle"));
        state.set_enabled(true);
        let enabled_generation = state.generation.load(Ordering::Acquire);
        state.set_enabled(false);

        assert!(!state.enabled.load(Ordering::Acquire));
        assert!(state.generation.load(Ordering::Acquire) > enabled_generation);
        assert_eq!(
            *state.initialization.0.lock().unwrap(),
            LauncherInitialization::Idle
        );
    }

    #[test]
    fn concurrent_first_use_joins_one_initialization() {
        use std::sync::atomic::AtomicUsize;
        use std::sync::Barrier;

        let state = Arc::new(QuickLauncherState::new(
            std::env::temp_dir().join("zero-launch-single-flight"),
        ));
        state.set_enabled(true);
        let generation = state.generation.load(Ordering::Acquire);
        let calls = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(3));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let state = Arc::clone(&state);
            let calls = Arc::clone(&calls);
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                state.initialize_once(generation, || {
                    calls.fetch_add(1, Ordering::AcqRel);
                    std::thread::sleep(Duration::from_millis(20));
                    Ok(())
                })
            }));
        }
        barrier.wait();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }

        assert_eq!(calls.load(Ordering::Acquire), 1);
    }

    #[test]
    fn running_probe_is_outside_search_and_respects_ttl() {
        use std::sync::atomic::AtomicUsize;

        let state = QuickLauncherState::new(std::env::temp_dir().join("zero-running-ttl"));
        let calls = AtomicUsize::new(0);
        let first = state
            .refresh_running_states_with(|_| {
                calls.fetch_add(1, Ordering::AcqRel);
                HashMap::new()
            })
            .unwrap();
        let second = state
            .refresh_running_states_with(|_| {
                calls.fetch_add(1, Ordering::AcqRel);
                HashMap::new()
            })
            .unwrap();

        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert_eq!(first.running_revision, second.running_revision);
        state
            .search(QuickLauncherSearchInput {
                query: String::new(),
                limit: Some(24),
            })
            .unwrap();
        assert_eq!(calls.load(Ordering::Acquire), 1);
    }

    #[test]
    fn icon_cache_has_deterministic_count_and_byte_bounds() {
        let mut cache = IconCache::default();
        for index in 0..=MAX_ICON_CACHE_ITEMS {
            cache.insert(format!("icon-{index:03}"), Some("data".into()));
        }
        assert_eq!(cache.entries.len(), MAX_ICON_CACHE_ITEMS);
        assert!(!cache.entries.contains_key("icon-000"));
        assert!(cache.total_bytes <= MAX_ICON_CACHE_BYTES);

        cache.insert(
            "oversized".into(),
            Some("x".repeat(MAX_ICON_DATA_URL_BYTES + 1)),
        );
        assert!(!cache.entries.contains_key("oversized"));
    }

    #[test]
    fn diagnostics_are_bounded_by_count_and_utf8_safe_message_bytes() {
        let state = QuickLauncherState::new(std::env::temp_dir().join("zero-diagnostic-bounds"));
        for index in 0..(MAX_DIAGNOSTICS + 5) {
            state.add_diagnostic(
                &format!("launcher.test_{index}"),
                "测".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES),
            );
        }
        let snapshot = state.snapshot().unwrap();
        assert_eq!(snapshot.diagnostics.len(), MAX_DIAGNOSTICS);
        assert!(snapshot
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES));
        assert_eq!(snapshot.diagnostics[0].code, "launcher.test_5");
    }

    #[test]
    fn running_overlay_does_not_mutate_the_stable_index_snapshot() {
        let state = QuickLauncherState::new(std::env::temp_dir().join("zero-running-overlay"));
        let item = catalog::system_setting_items("en-US")
            .into_iter()
            .next()
            .unwrap();
        let item_id = item.id.clone();
        {
            let mut index = state.index.write().unwrap();
            index.revision = 4;
            index.items = Arc::new(vec![item]);
        }
        {
            let mut running = state.running_cache.lock().unwrap();
            running.values = Arc::new(HashMap::from([(
                item_id,
                QuickLauncherRunningState::Running,
            )]));
        }

        let result = state
            .search(QuickLauncherSearchInput {
                query: String::new(),
                limit: Some(24),
            })
            .unwrap();
        assert_eq!(result.revision, 4);
        assert_eq!(result.items[0].running, QuickLauncherRunningState::Running);
        assert_eq!(
            state.index.read().unwrap().items[0].running,
            QuickLauncherRunningState::NotApplicable
        );
    }

    #[test]
    fn activation_rejects_an_item_from_a_stale_index_revision() {
        let state = QuickLauncherState::new(std::env::temp_dir().join("zero-stale-activation"));
        let item = catalog::system_setting_items("en-US")
            .into_iter()
            .next()
            .unwrap();
        let item_id = item.id.clone();
        {
            let mut index = state.index.write().unwrap();
            index.revision = 8;
            index.items = Arc::new(vec![item]);
        }

        let error = state
            .activate(QuickLauncherActivateInput {
                item_id,
                revision: 7,
            })
            .unwrap_err();
        assert_eq!(error.code, "launcher.item_stale");
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[test]
    fn unsupported_platform_is_explicit() {
        let state = QuickLauncherState::new(std::env::temp_dir().join("zero-unsupported"));
        assert_eq!(
            state.snapshot().unwrap().platform_support,
            QuickLauncherPlatformSupport::Unsupported
        );
    }
}
