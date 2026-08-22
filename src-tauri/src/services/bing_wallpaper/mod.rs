use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use futures_util::stream::{self, StreamExt};
use getrandom::fill;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::plugins::contracts::{NativeResourceError, NetworkFetchRequest, NetworkFetchResponse};

use self::wallpaper::{set_plugin_wallpaper, validate_plugin_image};
use super::native_resources::{fetch_https, write_plugin_file, DEFAULT_STORAGE_LIMIT_BYTES};

pub mod wallpaper;

pub use self::wallpaper::{
    platform_capability, SystemWallpaperSetter, WallpaperPlatformCapability, WallpaperSetter,
};

pub const BING_MARKET: &str = "zh-CN";
pub const BING_ARCHIVE_URL: &str =
    "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=10&mkt=zh-CN";
const BING_HOST: &str = "www.bing.com";
const CACHE_SCHEMA_VERSION: u16 = 2;
const PREVIOUS_CACHE_SCHEMA_VERSION: u16 = 1;
const MAX_HISTORY_ITEMS: usize = 10;
const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;
const PREVIEW_VERSION: u16 = 1;
const PREVIEW_MAX_WIDTH: u32 = 960;
const PREVIEW_MAX_HEIGHT: u32 = 600;
const PREVIEW_JPEG_QUALITY: u8 = 82;
const MAX_PREVIEW_BYTES: usize = 2 * 1024 * 1024;
const MAX_PREVIEW_LEASES: usize = 16;
const MAX_PREVIEW_LEASE_BYTES: usize = 8 * 1024 * 1024;
const PREVIEW_LEASE_TTL_MS: u64 = 5 * 60 * 1000;

pub type BingFetchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<NetworkFetchResponse, NativeResourceError>> + Send + 'a>>;

pub trait BingWallpaperFetcher: Send + Sync {
    fn fetch(&self, request: NetworkFetchRequest, max_bytes: usize) -> BingFetchFuture<'_>;
}

#[derive(Debug, Default)]
pub struct SystemBingWallpaperFetcher;

impl BingWallpaperFetcher for SystemBingWallpaperFetcher {
    fn fetch(&self, request: NetworkFetchRequest, max_bytes: usize) -> BingFetchFuture<'_> {
        Box::pin(async move { fetch_https(&request, &[BING_HOST], max_bytes).await })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperItem {
    pub id: String,
    pub start_date: String,
    pub title: String,
    pub attribution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright_url: Option<String>,
    pub remote_url: String,
    pub cache_file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_file_name: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperSnapshot {
    pub items: Vec<BingWallpaperItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refreshed_at: Option<String>,
    pub market: String,
    pub stale: bool,
    pub platform: WallpaperPlatformCapability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BingWallpaperError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperPreview {
    pub wallpaper_id: String,
    pub token: String,
    pub mime_type: String,
    pub byte_length: u64,
    pub width: u32,
    pub height: u32,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperPreviewResourceInput {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperActionInput {
    pub wallpaper_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperActionResult {
    pub wallpaper_id: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BingWallpaperError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for BingWallpaperError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BingWallpaperError {}

impl From<NativeResourceError> for BingWallpaperError {
    fn from(error: NativeResourceError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            retryable: error.retryable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BingWallpaperCacheIndex {
    schema_version: u16,
    refreshed_at: Option<String>,
    market: String,
    items: Vec<BingWallpaperItem>,
}

#[derive(Debug, Deserialize)]
struct BingArchiveResponse {
    #[serde(default)]
    images: Vec<BingArchiveImage>,
}

#[derive(Debug, Deserialize)]
struct BingArchiveImage {
    #[serde(default)]
    startdate: String,
    title: Option<String>,
    copyright: Option<String>,
    copyrightlink: Option<String>,
    url: Option<String>,
    urlbase: Option<String>,
    hsh: Option<String>,
}

pub struct BingWallpaperState {
    root: PathBuf,
    refresh_lock: Mutex<()>,
    refresh_sequence: AtomicU64,
    last_refresh: StdMutex<Option<BingWallpaperSnapshot>>,
    preview_generation_locks: StdMutex<HashMap<String, Arc<Mutex<()>>>>,
    preview_leases: StdMutex<PreviewLeaseCache>,
    preview_access_sequence: AtomicU64,
    fetcher: Arc<dyn BingWallpaperFetcher>,
}

#[derive(Debug, Clone)]
struct PreviewDerivative {
    path: PathBuf,
    byte_length: u64,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
struct PreviewLease {
    wallpaper_id: String,
    path: PathBuf,
    byte_length: u64,
    width: u32,
    height: u32,
    expires_at_ms: u64,
    last_access: u64,
    reference_count: usize,
}

#[derive(Default)]
struct PreviewLeaseCache {
    entries: HashMap<String, PreviewLease>,
}

impl Default for BingWallpaperState {
    fn default() -> Self {
        Self::new(default_cache_root())
    }
}

impl BingWallpaperState {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_fetcher(root, Arc::new(SystemBingWallpaperFetcher))
    }

    pub fn with_fetcher(root: impl Into<PathBuf>, fetcher: Arc<dyn BingWallpaperFetcher>) -> Self {
        Self {
            root: root.into(),
            refresh_lock: Mutex::new(()),
            refresh_sequence: AtomicU64::new(0),
            last_refresh: StdMutex::new(None),
            preview_generation_locks: StdMutex::new(HashMap::new()),
            preview_leases: StdMutex::new(PreviewLeaseCache::default()),
            preview_access_sequence: AtomicU64::new(0),
            fetcher,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn snapshot(&self) -> BingWallpaperSnapshot {
        match load_cache_index(&self.root) {
            Ok(index) => snapshot_from_index(index, false, None),
            Err(error) => BingWallpaperSnapshot {
                items: Vec::new(),
                refreshed_at: None,
                market: BING_MARKET.into(),
                stale: true,
                platform: platform_capability(),
                error: Some(error),
            },
        }
    }

    pub async fn refresh(&self) -> BingWallpaperSnapshot {
        let observed_sequence = self.refresh_sequence.load(Ordering::Acquire);
        let _guard = self.refresh_lock.lock().await;
        if self.refresh_sequence.load(Ordering::Acquire) != observed_sequence {
            return self
                .last_refresh
                .lock()
                .ok()
                .and_then(|snapshot| snapshot.clone())
                .unwrap_or_else(|| self.snapshot());
        }

        let snapshot = self.refresh_once().await;
        if let Ok(mut last_refresh) = self.last_refresh.lock() {
            *last_refresh = Some(snapshot.clone());
        }
        self.refresh_sequence.fetch_add(1, Ordering::Release);
        snapshot
    }

    async fn refresh_once(&self) -> BingWallpaperSnapshot {
        let cached_index = load_cache_index(&self.root).unwrap_or_else(|_| empty_cache_index());
        let cached_items = cached_index.items.clone();
        let metadata = match self
            .fetcher
            .fetch(
                NetworkFetchRequest {
                    url: BING_ARCHIVE_URL.into(),
                    method: Some("GET".into()),
                },
                MAX_METADATA_BYTES,
            )
            .await
        {
            Ok(response) if (200..300).contains(&response.status) => response,
            Ok(response) => {
                return snapshot_from_index(
                    cached_index,
                    true,
                    Some(error(
                        "bing.http_status",
                        format!("Bing returned HTTP status {}.", response.status),
                        true,
                    )),
                )
            }
            Err(fetch_error) => {
                return snapshot_from_index(cached_index, true, Some(fetch_error.into()))
            }
        };

        let metadata_bytes = match BASE64_STANDARD.decode(metadata.body_base64) {
            Ok(bytes) => bytes,
            Err(decode_error) => {
                return snapshot_from_index(
                    cached_index,
                    true,
                    Some(error(
                        "bing.response_decode",
                        format!("Failed to decode Bing response: {decode_error}"),
                        true,
                    )),
                )
            }
        };
        let metadata_json = match String::from_utf8(metadata_bytes) {
            Ok(json) => json,
            Err(utf8_error) => {
                return snapshot_from_index(
                    cached_index,
                    true,
                    Some(error(
                        "bing.response_encoding",
                        format!("Bing response is not UTF-8: {utf8_error}"),
                        true,
                    )),
                )
            }
        };
        let remote_items = match parse_bing_archive(&metadata_json) {
            Ok(items) => items,
            Err(parse_error) => return snapshot_from_index(cached_index, true, Some(parse_error)),
        };

        let mut merged = merge_wallpaper_items(remote_items, cached_items.clone());
        let pending = merged
            .iter()
            .filter(|item| !item.cached)
            .cloned()
            .collect::<Vec<_>>();
        let root = self.root.clone();
        let fetcher = Arc::clone(&self.fetcher);
        let results = stream::iter(pending.into_iter().map(|item| {
            let root = root.clone();
            let fetcher = Arc::clone(&fetcher);
            async move {
                let id = item.id.clone();
                let result = download_wallpaper_item(&root, &item, fetcher.as_ref()).await;
                (id, result)
            }
        }))
        .buffer_unordered(2)
        .collect::<Vec<_>>()
        .await;

        let mut partial_error = None;
        for (id, result) in results {
            match result {
                Ok(()) => {
                    if let Some(item) = merged.iter_mut().find(|item| item.id == id) {
                        item.cached = true;
                    }
                }
                Err(download_error) => {
                    partial_error.get_or_insert(download_error);
                }
            }
        }

        let refreshed_at = Some(now_timestamp());
        let next_index = BingWallpaperCacheIndex {
            schema_version: CACHE_SCHEMA_VERSION,
            refreshed_at: refreshed_at.clone(),
            market: BING_MARKET.into(),
            items: merged,
        };
        if let Err(persist_error) = persist_cache_index(&self.root, &next_index) {
            return snapshot_from_index(cached_index, true, Some(persist_error));
        }
        cleanup_obsolete_files(&self.root, &cached_items, &next_index.items);

        snapshot_from_index(next_index, false, partial_error)
    }

    pub async fn preview(
        &self,
        wallpaper_id: &str,
    ) -> Result<BingWallpaperPreview, BingWallpaperError> {
        let item = self.ensure_cached(wallpaper_id).await?;
        let generation_lock = self.preview_generation_lock(&item.id)?;
        let _generation_guard = generation_lock.lock().await;
        let _cache_guard = self.refresh_lock.lock().await;
        let mut index = load_cache_index(&self.root)?;
        let position = index
            .items
            .iter()
            .position(|candidate| candidate.id == item.id)
            .ok_or_else(|| error("wallpaper.not_found", "Wallpaper was not found.", false))?;
        let validation_root = self.root.clone();
        let cache_file_name = index.items[position].cache_file_name.clone();
        let source_path = tauri::async_runtime::spawn_blocking(move || {
            validate_plugin_image(&validation_root, &cache_file_name)
                .map_err(BingWallpaperError::from)
        })
        .await
        .map_err(|_| {
            error(
                "preview.validation_worker",
                "The wallpaper preview validation worker stopped unexpectedly.",
                true,
            )
        })??;
        let preview_file_name = preview_file_name(&index.items[position].cache_file_name)?;
        let preview_path = self.root.join(&preview_file_name);
        let derivative_preview_path = preview_path.clone();
        let derivative = tauri::async_runtime::spawn_blocking(move || {
            validate_preview_derivative(&derivative_preview_path)
                .or_else(|_| generate_preview_derivative(&source_path, &derivative_preview_path))
        })
        .await
        .map_err(|_| {
            error(
                "preview.worker",
                "The wallpaper preview worker stopped unexpectedly.",
                true,
            )
        })??;
        if index.items[position].preview_file_name.as_deref() != Some(&preview_file_name) {
            index.items[position].preview_file_name = Some(preview_file_name);
            index.schema_version = CACHE_SCHEMA_VERSION;
            persist_cache_index(&self.root, &index)?;
        }
        self.issue_preview_lease(&item.id, derivative)
    }

    pub fn read_preview_bytes(&self, token: &str) -> Result<Vec<u8>, BingWallpaperError> {
        let lease = {
            let now = now_ms();
            let mut cache = self.preview_leases.lock().map_err(|_| {
                error(
                    "preview.cache",
                    "The wallpaper preview lease cache is unavailable.",
                    true,
                )
            })?;
            cache.remove_expired(now);
            let access = self.preview_access_sequence.fetch_add(1, Ordering::Relaxed) + 1;
            let lease = cache.entries.get_mut(token).ok_or_else(|| {
                error(
                    "preview.token",
                    "The wallpaper preview token is invalid or expired.",
                    false,
                )
            })?;
            lease.last_access = access;
            lease.clone()
        };
        if lease.expires_at_ms <= now_ms()
            || lease.byte_length == 0
            || lease.byte_length as usize > MAX_PREVIEW_BYTES
            || !lease.path.starts_with(&self.root)
        {
            self.release_preview(token);
            return Err(error(
                "preview.token",
                "The wallpaper preview token is invalid or expired.",
                false,
            ));
        }
        let bytes = fs::read(&lease.path).map_err(|read_error| {
            error(
                "preview.read",
                format!("Failed to read cached wallpaper preview: {read_error}"),
                true,
            )
        })?;
        if bytes.len() as u64 != lease.byte_length || bytes.len() > MAX_PREVIEW_BYTES {
            self.release_preview(token);
            return Err(error(
                "preview.changed",
                "The cached wallpaper preview changed after its lease was issued.",
                true,
            ));
        }
        Ok(bytes)
    }

    pub fn release_preview(&self, token: &str) {
        if let Ok(mut cache) = self.preview_leases.lock() {
            if let Some(lease) = cache.entries.get_mut(token) {
                lease.reference_count = lease.reference_count.saturating_sub(1);
                if lease.reference_count > 0 {
                    return;
                }
            }
            cache.entries.remove(token);
        }
    }

    fn preview_generation_lock(
        &self,
        wallpaper_id: &str,
    ) -> Result<Arc<Mutex<()>>, BingWallpaperError> {
        let mut locks = self.preview_generation_locks.lock().map_err(|_| {
            error(
                "preview.lock",
                "The wallpaper preview generation lock is unavailable.",
                true,
            )
        })?;
        if let Some(lock) = locks.get(wallpaper_id) {
            return Ok(Arc::clone(lock));
        }
        while locks.len() >= MAX_HISTORY_ITEMS {
            let candidate = locks
                .iter()
                .filter(|(_, lock)| Arc::strong_count(lock) == 1)
                .map(|(id, _)| id)
                .min()
                .cloned();
            let Some(candidate) = candidate else {
                return Err(error(
                    "preview.lock_limit",
                    "Too many wallpaper previews are being generated concurrently.",
                    true,
                ));
            };
            locks.remove(&candidate);
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(wallpaper_id.to_string(), Arc::clone(&lock));
        Ok(lock)
    }

    fn issue_preview_lease(
        &self,
        wallpaper_id: &str,
        derivative: PreviewDerivative,
    ) -> Result<BingWallpaperPreview, BingWallpaperError> {
        let now = now_ms();
        let access = self.preview_access_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let mut cache = self.preview_leases.lock().map_err(|_| {
            error(
                "preview.cache",
                "The wallpaper preview lease cache is unavailable.",
                true,
            )
        })?;
        cache.remove_expired(now);
        if let Some((token, lease)) = cache.entries.iter_mut().find(|(_, lease)| {
            lease.wallpaper_id == wallpaper_id
                && lease.path == derivative.path
                && lease.byte_length == derivative.byte_length
                && lease.width == derivative.width
                && lease.height == derivative.height
                && lease.expires_at_ms > now
        }) {
            lease.reference_count = lease.reference_count.saturating_add(1);
            lease.last_access = access;
            return Ok(preview_descriptor(token, lease));
        }
        cache.evict_for(derivative.byte_length as usize);
        let token = random_preview_token()?;
        let lease = PreviewLease {
            wallpaper_id: wallpaper_id.to_string(),
            path: derivative.path,
            byte_length: derivative.byte_length,
            width: derivative.width,
            height: derivative.height,
            expires_at_ms: now.saturating_add(PREVIEW_LEASE_TTL_MS),
            last_access: access,
            reference_count: 1,
        };
        let descriptor = preview_descriptor(&token, &lease);
        cache.entries.insert(token, lease);
        Ok(descriptor)
    }

    pub async fn apply(
        &self,
        wallpaper_id: &str,
        setter: &dyn WallpaperSetter,
    ) -> Result<BingWallpaperActionResult, BingWallpaperError> {
        let item = self.ensure_cached(wallpaper_id).await?;
        let path = set_plugin_wallpaper(setter, &self.root, &item.cache_file_name)
            .map_err(BingWallpaperError::from)?;

        Ok(BingWallpaperActionResult {
            wallpaper_id: item.id,
            path: path.to_string_lossy().into_owned(),
            message: "Wallpaper applied.".into(),
        })
    }

    pub async fn apply_with_system_setter(
        &self,
        wallpaper_id: &str,
    ) -> Result<BingWallpaperActionResult, BingWallpaperError> {
        let item = self.ensure_cached(wallpaper_id).await?;
        let root = self.root.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let path = set_plugin_wallpaper(&SystemWallpaperSetter, &root, &item.cache_file_name)
                .map_err(BingWallpaperError::from)?;
            Ok(BingWallpaperActionResult {
                wallpaper_id: item.id,
                path: path.to_string_lossy().into_owned(),
                message: "Wallpaper applied.".into(),
            })
        })
        .await
        .map_err(|_| {
            error(
                "wallpaper.worker",
                "The wallpaper apply worker stopped unexpectedly.",
                true,
            )
        })?
    }

    pub async fn save_to_downloads(
        &self,
        wallpaper_id: &str,
        downloads: &Path,
    ) -> Result<BingWallpaperActionResult, BingWallpaperError> {
        let item = self.ensure_cached(wallpaper_id).await?;
        let root = self.root.clone();
        let downloads = downloads.to_path_buf();
        tauri::async_runtime::spawn_blocking(move || {
            let source = validate_plugin_image(&root, &item.cache_file_name)
                .map_err(BingWallpaperError::from)?;
            fs::create_dir_all(&downloads).map_err(|create_error| {
                error(
                    "downloads.unavailable",
                    format!("Failed to create Downloads directory: {create_error}"),
                    true,
                )
            })?;

            let base = format!("{}-bing-wallpaper", sanitize_date(&item.start_date));
            let mut destination = downloads.join(format!("{base}.jpg"));
            let mut suffix = 2usize;
            while destination.exists() {
                destination = downloads.join(format!("{base}-{suffix}.jpg"));
                suffix += 1;
            }
            fs::copy(&source, &destination).map_err(|copy_error| {
                error(
                    "downloads.copy",
                    format!("Failed to save wallpaper to Downloads: {copy_error}"),
                    true,
                )
            })?;

            Ok(BingWallpaperActionResult {
                wallpaper_id: item.id,
                path: destination.to_string_lossy().into_owned(),
                message: "Wallpaper saved to Downloads.".into(),
            })
        })
        .await
        .map_err(|_| {
            error(
                "downloads.worker",
                "The wallpaper save worker stopped unexpectedly.",
                true,
            )
        })?
    }

    async fn ensure_cached(
        &self,
        wallpaper_id: &str,
    ) -> Result<BingWallpaperItem, BingWallpaperError> {
        let _guard = self.refresh_lock.lock().await;
        let mut index = load_cache_index(&self.root)?;
        let position = index
            .items
            .iter()
            .position(|item| item.id == wallpaper_id)
            .ok_or_else(|| error("wallpaper.not_found", "Wallpaper was not found.", false))?;

        let cached_image_valid = if index.items[position].cached {
            let root = self.root.clone();
            let cache_file_name = index.items[position].cache_file_name.clone();
            tauri::async_runtime::spawn_blocking(move || {
                validate_plugin_image(&root, &cache_file_name).is_ok()
            })
            .await
            .map_err(|_| {
                error(
                    "cache.validation_worker",
                    "The wallpaper cache validation worker stopped unexpectedly.",
                    true,
                )
            })?
        } else {
            false
        };
        if !cached_image_valid {
            download_wallpaper_item(&self.root, &index.items[position], self.fetcher.as_ref())
                .await?;
            index.items[position].cached = true;
            persist_cache_index(&self.root, &index)?;
        }

        Ok(index.items[position].clone())
    }
}

impl PreviewLeaseCache {
    fn remove_expired(&mut self, now_ms: u64) {
        self.entries.retain(|_, lease| lease.expires_at_ms > now_ms);
    }

    fn evict_for(&mut self, incoming_bytes: usize) {
        while self.entries.len() >= MAX_PREVIEW_LEASES
            || self.total_bytes().saturating_add(incoming_bytes) > MAX_PREVIEW_LEASE_BYTES
        {
            let candidate = self
                .entries
                .iter()
                .min_by(|(left_token, left), (right_token, right)| {
                    left.last_access
                        .cmp(&right.last_access)
                        .then_with(|| left_token.cmp(right_token))
                })
                .map(|(token, _)| token.clone());
            let Some(candidate) = candidate else {
                break;
            };
            self.entries.remove(&candidate);
        }
    }

    fn total_bytes(&self) -> usize {
        self.entries.values().fold(0usize, |total, lease| {
            total.saturating_add(lease.byte_length as usize)
        })
    }
}

fn preview_descriptor(token: &str, lease: &PreviewLease) -> BingWallpaperPreview {
    BingWallpaperPreview {
        wallpaper_id: lease.wallpaper_id.clone(),
        token: token.to_string(),
        mime_type: "image/jpeg".into(),
        byte_length: lease.byte_length,
        width: lease.width,
        height: lease.height,
        expires_at_ms: lease.expires_at_ms,
    }
}

fn preview_file_name(cache_file_name: &str) -> Result<String, BingWallpaperError> {
    if !is_safe_cache_file_name(cache_file_name) {
        return Err(error(
            "preview.path",
            "The wallpaper cache file name is invalid.",
            false,
        ));
    }
    let stem = Path::new(cache_file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            error(
                "preview.path",
                "The wallpaper cache file name is invalid.",
                false,
            )
        })?;
    Ok(format!("{stem}-preview-v{PREVIEW_VERSION}.jpg"))
}

fn generate_preview_derivative(
    source_path: &Path,
    preview_path: &Path,
) -> Result<PreviewDerivative, BingWallpaperError> {
    let image = image::open(source_path).map_err(|image_error| {
        error(
            "preview.decode",
            format!("Failed to decode cached wallpaper: {image_error}"),
            false,
        )
    })?;
    let preview = image
        .thumbnail(PREVIEW_MAX_WIDTH, PREVIEW_MAX_HEIGHT)
        .to_rgb8();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, PREVIEW_JPEG_QUALITY)
        .encode_image(&preview)
        .map_err(|encode_error| {
            error(
                "preview.encode",
                format!("Failed to encode wallpaper preview: {encode_error}"),
                true,
            )
        })?;
    if bytes.is_empty() || bytes.len() > MAX_PREVIEW_BYTES {
        return Err(error(
            "preview.too_large",
            "The generated wallpaper preview exceeds its byte limit.",
            false,
        ));
    }
    let root = preview_path.parent().ok_or_else(|| {
        error(
            "preview.path",
            "The wallpaper preview path is invalid.",
            false,
        )
    })?;
    let file_name = preview_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            error(
                "preview.path",
                "The wallpaper preview path is invalid.",
                false,
            )
        })?;
    write_plugin_file(root, file_name, &bytes, MAX_PREVIEW_BYTES)
        .map_err(BingWallpaperError::from)?;
    validate_preview_derivative(preview_path)
}

fn validate_preview_derivative(path: &Path) -> Result<PreviewDerivative, BingWallpaperError> {
    let metadata = fs::metadata(path).map_err(|read_error| {
        error(
            "preview.missing",
            format!("The cached wallpaper preview is unavailable: {read_error}"),
            true,
        )
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() as usize > MAX_PREVIEW_BYTES {
        return Err(error(
            "preview.too_large",
            "The cached wallpaper preview size is invalid.",
            false,
        ));
    }
    let reader = image::ImageReader::open(path)
        .map_err(|read_error| {
            error(
                "preview.read",
                format!("Failed to open cached wallpaper preview: {read_error}"),
                true,
            )
        })?
        .with_guessed_format()
        .map_err(|format_error| {
            error(
                "preview.format",
                format!("Failed to identify cached wallpaper preview: {format_error}"),
                false,
            )
        })?;
    if reader.format() != Some(image::ImageFormat::Jpeg) {
        return Err(error(
            "preview.format",
            "The cached wallpaper preview is not a JPEG derivative.",
            false,
        ));
    }
    let (width, height) = reader.into_dimensions().map_err(|decode_error| {
        error(
            "preview.decode",
            format!("Failed to decode cached wallpaper preview: {decode_error}"),
            false,
        )
    })?;
    if width == 0 || height == 0 || width > PREVIEW_MAX_WIDTH || height > PREVIEW_MAX_HEIGHT {
        return Err(error(
            "preview.dimensions",
            "The cached wallpaper preview dimensions are invalid.",
            false,
        ));
    }
    Ok(PreviewDerivative {
        path: path.to_path_buf(),
        byte_length: metadata.len(),
        width,
        height,
    })
}

fn random_preview_token() -> Result<String, BingWallpaperError> {
    let mut bytes = [0u8; 24];
    fill(&mut bytes).map_err(|_| {
        error(
            "preview.token",
            "A secure wallpaper preview token could not be created.",
            true,
        )
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn parse_bing_archive(json: &str) -> Result<Vec<BingWallpaperItem>, BingWallpaperError> {
    let archive: BingArchiveResponse = serde_json::from_str(json).map_err(|parse_error| {
        error(
            "bing.response_invalid",
            format!("Failed to parse Bing wallpaper response: {parse_error}"),
            true,
        )
    })?;

    let mut items = Vec::new();
    for image in archive.images {
        let Some(remote_url) =
            normalized_remote_url(image.url.as_deref(), image.urlbase.as_deref())
        else {
            continue;
        };
        let start_date = sanitize_date(&image.startdate);
        if start_date.is_empty() {
            continue;
        }
        let attribution = image.copyright.unwrap_or_default().trim().to_owned();
        let title = image.title.unwrap_or_default().trim().to_owned();
        let title = if title.is_empty() {
            attribution
                .split(" (")
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        } else {
            title
        };
        let id = image
            .hsh
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| hash_identifier(&remote_url));
        let short_id = id
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .take(16)
            .collect::<String>();
        let cache_file_name = format!(
            "{start_date}-{}.jpg",
            if short_id.is_empty() {
                hash_identifier(&remote_url)
            } else {
                short_id
            }
        );

        items.push(BingWallpaperItem {
            id,
            start_date,
            title,
            attribution,
            copyright_url: normalized_optional_bing_url(image.copyrightlink.as_deref()),
            remote_url,
            cache_file_name,
            preview_file_name: None,
            cached: false,
        });
    }

    if items.is_empty() {
        return Err(error(
            "bing.no_valid_items",
            "Bing response did not contain a valid wallpaper item.",
            true,
        ));
    }

    Ok(merge_wallpaper_items(items, Vec::new()))
}

pub fn merge_wallpaper_items(
    remote: Vec<BingWallpaperItem>,
    cached: Vec<BingWallpaperItem>,
) -> Vec<BingWallpaperItem> {
    let cached_by_id = cached
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for mut item in remote.into_iter().chain(cached) {
        if !seen.insert(item.id.clone()) {
            continue;
        }
        if let Some(previous) = cached_by_id.get(&item.id) {
            item.cached = previous.cached;
            if previous.cached {
                item.cache_file_name = previous.cache_file_name.clone();
                item.preview_file_name = previous.preview_file_name.clone();
            }
        }
        merged.push(item);
    }

    merged.sort_by(|left, right| {
        right
            .start_date
            .cmp(&left.start_date)
            .then_with(|| left.id.cmp(&right.id))
    });
    merged.truncate(MAX_HISTORY_ITEMS);
    merged
}

pub fn default_cache_root() -> PathBuf {
    crate::brand::canonical_data_root(&crate::brand::default_home())
        .join("data")
        .join("wallpaper")
}

async fn download_wallpaper_item(
    root: &Path,
    item: &BingWallpaperItem,
    fetcher: &dyn BingWallpaperFetcher,
) -> Result<(), BingWallpaperError> {
    let response = fetcher
        .fetch(
            NetworkFetchRequest {
                url: item.remote_url.clone(),
                method: Some("GET".into()),
            },
            MAX_IMAGE_BYTES,
        )
        .await
        .map_err(BingWallpaperError::from)?;
    if !(200..300).contains(&response.status) {
        return Err(error(
            "image.http_status",
            format!("Bing image returned HTTP status {}.", response.status),
            true,
        ));
    }
    if response
        .content_type
        .as_deref()
        .is_some_and(|content_type| !content_type.to_ascii_lowercase().starts_with("image/"))
    {
        return Err(error(
            "image.content_type",
            "Bing image response did not have an image content type.",
            true,
        ));
    }

    let root = root.to_path_buf();
    let cache_file_name = item.cache_file_name.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = BASE64_STANDARD
            .decode(response.body_base64)
            .map_err(|decode_error| {
                error(
                    "image.decode",
                    format!("Failed to decode Bing image: {decode_error}"),
                    true,
                )
            })?;
        image::load_from_memory(&bytes).map_err(|image_error| {
            error(
                "image.invalid",
                format!("Bing response is not a supported image: {image_error}"),
                true,
            )
        })?;
        write_plugin_file(&root, &cache_file_name, &bytes, DEFAULT_STORAGE_LIMIT_BYTES)
            .map(|_| ())
            .map_err(BingWallpaperError::from)
    })
    .await
    .map_err(|_| {
        error(
            "image.worker",
            "The wallpaper image worker stopped unexpectedly.",
            true,
        )
    })?
}

fn load_cache_index(root: &Path) -> Result<BingWallpaperCacheIndex, BingWallpaperError> {
    let path = root.join("index.json");
    if !path.exists() {
        return Ok(empty_cache_index());
    }

    let content = fs::read_to_string(&path).map_err(|read_error| {
        error(
            "cache.read",
            format!("Failed to read wallpaper cache index: {read_error}"),
            true,
        )
    })?;
    let mut index: BingWallpaperCacheIndex =
        serde_json::from_str(&content).map_err(|parse_error| {
            error(
                "cache.invalid",
                format!("Failed to parse wallpaper cache index: {parse_error}"),
                true,
            )
        })?;
    if !matches!(
        index.schema_version,
        PREVIOUS_CACHE_SCHEMA_VERSION | CACHE_SCHEMA_VERSION
    ) {
        return Err(error(
            "cache.schema",
            "Wallpaper cache schema is not supported.",
            false,
        ));
    }

    index.items.retain(|item| {
        if !is_safe_cache_file_name(&item.cache_file_name)
            || normalized_remote_url(Some(&item.remote_url), None).is_none()
        {
            return false;
        }
        true
    });
    for item in &mut index.items {
        item.cached = root.join(&item.cache_file_name).is_file();
        if item
            .preview_file_name
            .as_deref()
            .is_some_and(|name| !is_safe_cache_file_name(name) || !root.join(name).is_file())
        {
            item.preview_file_name = None;
        }
    }
    index.items = merge_wallpaper_items(Vec::new(), index.items);
    Ok(index)
}

fn persist_cache_index(
    root: &Path,
    index: &BingWallpaperCacheIndex,
) -> Result<(), BingWallpaperError> {
    fs::create_dir_all(root).map_err(|create_error| {
        error(
            "cache.create",
            format!("Failed to create wallpaper cache: {create_error}"),
            true,
        )
    })?;
    let content = serde_json::to_vec_pretty(index).map_err(|serialize_error| {
        error(
            "cache.serialize",
            format!("Failed to serialize wallpaper cache index: {serialize_error}"),
            false,
        )
    })?;
    write_plugin_file(root, "index.json", &content, MAX_METADATA_BYTES)
        .map_err(BingWallpaperError::from)?;
    Ok(())
}

fn cleanup_obsolete_files(
    root: &Path,
    previous: &[BingWallpaperItem],
    current: &[BingWallpaperItem],
) {
    let retained = current
        .iter()
        .flat_map(|item| {
            std::iter::once(item.cache_file_name.as_str()).chain(item.preview_file_name.as_deref())
        })
        .collect::<HashSet<_>>();
    for item in previous {
        for file_name in
            std::iter::once(item.cache_file_name.as_str()).chain(item.preview_file_name.as_deref())
        {
            if retained.contains(file_name) || !is_safe_cache_file_name(file_name) {
                continue;
            }
            let _ = fs::remove_file(root.join(file_name));
        }
    }
}

fn snapshot_from_index(
    index: BingWallpaperCacheIndex,
    stale: bool,
    error: Option<BingWallpaperError>,
) -> BingWallpaperSnapshot {
    BingWallpaperSnapshot {
        items: index.items,
        refreshed_at: index.refreshed_at,
        market: index.market,
        stale,
        platform: platform_capability(),
        error,
    }
}

fn empty_cache_index() -> BingWallpaperCacheIndex {
    BingWallpaperCacheIndex {
        schema_version: CACHE_SCHEMA_VERSION,
        refreshed_at: None,
        market: BING_MARKET.into(),
        items: Vec::new(),
    }
}

fn normalized_remote_url(url: Option<&str>, url_base: Option<&str>) -> Option<String> {
    if let Some(url) = url.filter(|value| !value.trim().is_empty()) {
        return normalized_bing_image_url(url);
    }

    let base = url_base?.trim();
    if base.is_empty() {
        return None;
    }
    normalized_bing_image_url(&format!("{base}_1920x1080.jpg"))
}

fn normalized_bing_image_url(value: &str) -> Option<String> {
    let normalized = normalized_optional_bing_url(Some(value))?;
    let parsed = reqwest::Url::parse(&normalized).ok()?;
    let has_image_id = parsed
        .query_pairs()
        .any(|(key, value)| key == "id" && !value.trim().is_empty());
    if parsed.path() != "/th" || !has_image_id {
        return None;
    }
    Some(normalized)
}

fn normalized_optional_bing_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let absolute = if value.starts_with('/') {
        format!("https://{BING_HOST}{value}")
    } else {
        value.to_owned()
    };
    let parsed = reqwest::Url::parse(&absolute).ok()?;
    if parsed.scheme() != "https" || parsed.host_str() != Some(BING_HOST) {
        return None;
    }
    Some(parsed.into())
}

fn is_safe_cache_file_name(value: &str) -> bool {
    let path = Path::new(value);
    path.components().count() == 1
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "jpg" | "jpeg" | "png"
                )
            })
}

fn sanitize_date(value: &str) -> String {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .take(8)
        .collect::<String>();
    if normalized.len() == 8 {
        normalized
    } else {
        String::new()
    }
}

fn hash_identifier(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn now_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn error(
    code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
) -> BingWallpaperError {
    BingWallpaperError {
        code: code.into(),
        message: message.into(),
        retryable,
    }
}

#[cfg(test)]
mod preview_cache_tests {
    use super::*;

    fn lease(id: usize, bytes: u64, access: u64) -> PreviewLease {
        PreviewLease {
            wallpaper_id: format!("wallpaper-{id}"),
            path: PathBuf::from(format!("preview-{id}.jpg")),
            byte_length: bytes,
            width: 320,
            height: 180,
            expires_at_ms: 10_000,
            last_access: access,
            reference_count: 1,
        }
    }

    #[test]
    fn preview_cache_evicts_deterministically_by_access_then_token() {
        let mut cache = PreviewLeaseCache::default();
        cache.entries.insert("b".into(), lease(1, 10, 1));
        cache.entries.insert("a".into(), lease(2, 10, 1));
        for index in 0..(MAX_PREVIEW_LEASES - 2) {
            cache.entries.insert(
                format!("recent-{index:02}"),
                lease(index + 2, 10, index as u64 + 2),
            );
        }
        cache.evict_for(1);
        assert!(!cache.entries.contains_key("a"));
        assert!(cache.entries.contains_key("b"));
    }

    #[test]
    fn preview_cache_removes_expired_entries_and_stays_within_count_bound() {
        let mut cache = PreviewLeaseCache::default();
        cache.entries.insert("expired".into(), lease(0, 10, 0));
        cache.remove_expired(10_000);
        assert!(cache.entries.is_empty());
        for index in 0..MAX_PREVIEW_LEASES {
            cache
                .entries
                .insert(format!("token-{index:02}"), lease(index, 10, index as u64));
        }
        cache.evict_for(10);
        assert!(cache.entries.len() < MAX_PREVIEW_LEASES);
    }

    #[test]
    fn preview_generation_locks_are_bounded_and_evict_only_idle_entries() {
        let state = BingWallpaperState::default();
        let held = (0..MAX_HISTORY_ITEMS)
            .map(|index| state.preview_generation_lock(&format!("held-{index:02}")))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            state.preview_generation_lock("overflow").unwrap_err().code,
            "preview.lock_limit"
        );
        drop(held);
        state.preview_generation_lock("replacement").unwrap();
        assert!(state.preview_generation_locks.lock().unwrap().len() <= MAX_HISTORY_ITEMS);
    }
}
