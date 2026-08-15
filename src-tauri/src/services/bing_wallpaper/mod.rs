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
use base64::Engine;
use futures_util::stream::{self, StreamExt};
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
const CACHE_SCHEMA_VERSION: u16 = 1;
const MAX_HISTORY_ITEMS: usize = 10;
const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;

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
    pub data_url: String,
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
    fetcher: Arc<dyn BingWallpaperFetcher>,
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
        let path = validate_plugin_image(&self.root, &item.cache_file_name)
            .map_err(BingWallpaperError::from)?;
        let bytes = fs::read(&path).map_err(|read_error| {
            error(
                "preview.read",
                format!("Failed to read cached wallpaper: {read_error}"),
                true,
            )
        })?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(error(
                "preview.too_large",
                "Cached wallpaper exceeds the preview size limit.",
                false,
            ));
        }
        let format = image::guess_format(&bytes).map_err(|format_error| {
            error(
                "preview.format",
                format!("Failed to detect wallpaper format: {format_error}"),
                false,
            )
        })?;
        let mime = match format {
            image::ImageFormat::Png => "image/png",
            _ => "image/jpeg",
        };

        Ok(BingWallpaperPreview {
            wallpaper_id: item.id,
            data_url: format!("data:{mime};base64,{}", BASE64_STANDARD.encode(bytes)),
        })
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
        self.apply(wallpaper_id, &SystemWallpaperSetter).await
    }

    pub async fn save_to_downloads(
        &self,
        wallpaper_id: &str,
        downloads: &Path,
    ) -> Result<BingWallpaperActionResult, BingWallpaperError> {
        let item = self.ensure_cached(wallpaper_id).await?;
        let source = validate_plugin_image(&self.root, &item.cache_file_name)
            .map_err(BingWallpaperError::from)?;
        fs::create_dir_all(downloads).map_err(|create_error| {
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

        if !index.items[position].cached
            || validate_plugin_image(&self.root, &index.items[position].cache_file_name).is_err()
        {
            download_wallpaper_item(&self.root, &index.items[position], self.fetcher.as_ref())
                .await?;
            index.items[position].cached = true;
            persist_cache_index(&self.root, &index)?;
        }

        Ok(index.items[position].clone())
    }
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
    write_plugin_file(
        root,
        &item.cache_file_name,
        &bytes,
        DEFAULT_STORAGE_LIMIT_BYTES,
    )
    .map_err(BingWallpaperError::from)?;
    Ok(())
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
    if index.schema_version != CACHE_SCHEMA_VERSION {
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
        .map(|item| item.cache_file_name.as_str())
        .collect::<HashSet<_>>();
    for item in previous {
        if retained.contains(item.cache_file_name.as_str())
            || !is_safe_cache_file_name(&item.cache_file_name)
        {
            continue;
        }
        let _ = fs::remove_file(root.join(&item.cache_file_name));
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
