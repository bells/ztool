use std::fs;
use std::future::poll_fn;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use image::{Rgb, RgbImage};
use serde_json::json;
use zero_lib::plugins::contracts::{
    NativeResourceError, NetworkFetchRequest, NetworkFetchResponse,
};
use zero_lib::services::bing_wallpaper::WallpaperSetter;
use zero_lib::services::bing_wallpaper::{
    merge_wallpaper_items, parse_bing_archive, BingFetchFuture, BingWallpaperFetcher,
    BingWallpaperItem, BingWallpaperSnapshot, BingWallpaperState, BING_ARCHIVE_URL,
};

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "zero-bing-wallpaper-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct RecordingSetter {
    paths: Mutex<Vec<PathBuf>>,
    error: Option<NativeResourceError>,
}

struct FixtureFetcher {
    metadata_body: String,
    metadata_status: u16,
    image_body_base64: String,
    failed_image_fragment: Option<String>,
    yield_metadata_once: bool,
    metadata_calls: AtomicUsize,
    image_calls: AtomicUsize,
}

impl FixtureFetcher {
    fn successful(metadata_body: String) -> Self {
        let image = image::DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 2, Rgb([41, 116, 168])));
        let mut encoded = Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Jpeg)
            .expect("encode jpeg");
        Self {
            metadata_body,
            metadata_status: 200,
            image_body_base64: BASE64_STANDARD.encode(encoded.into_inner()),
            failed_image_fragment: None,
            yield_metadata_once: false,
            metadata_calls: AtomicUsize::new(0),
            image_calls: AtomicUsize::new(0),
        }
    }
}

impl BingWallpaperFetcher for FixtureFetcher {
    fn fetch(&self, request: NetworkFetchRequest, _max_bytes: usize) -> BingFetchFuture<'_> {
        let is_metadata = request.url == BING_ARCHIVE_URL;
        if is_metadata {
            self.metadata_calls.fetch_add(1, Ordering::SeqCst);
        } else {
            self.image_calls.fetch_add(1, Ordering::SeqCst);
        }
        let should_yield = is_metadata && self.yield_metadata_once;
        let response = if is_metadata {
            NetworkFetchResponse {
                status: self.metadata_status,
                content_type: Some("application/json".into()),
                body_base64: BASE64_STANDARD.encode(self.metadata_body.as_bytes()),
            }
        } else if self
            .failed_image_fragment
            .as_deref()
            .is_some_and(|fragment| request.url.contains(fragment))
        {
            NetworkFetchResponse {
                status: 503,
                content_type: Some("image/jpeg".into()),
                body_base64: String::new(),
            }
        } else {
            NetworkFetchResponse {
                status: 200,
                content_type: Some("image/jpeg".into()),
                body_base64: self.image_body_base64.clone(),
            }
        };

        Box::pin(async move {
            if should_yield {
                let mut yielded = false;
                poll_fn(move |context| {
                    if yielded {
                        Poll::Ready(())
                    } else {
                        yielded = true;
                        context.waker().wake_by_ref();
                        Poll::Pending
                    }
                })
                .await;
            }
            Ok(response)
        })
    }
}

impl WallpaperSetter for RecordingSetter {
    fn set_from_path(&self, path: &Path) -> Result<(), NativeResourceError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        self.paths
            .lock()
            .expect("paths lock")
            .push(path.to_path_buf());
        Ok(())
    }
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name),
    )
    .expect("fixture")
}

fn item(id: &str, start_date: &str, cached: bool) -> BingWallpaperItem {
    BingWallpaperItem {
        id: id.into(),
        start_date: start_date.into(),
        title: format!("Title {id}"),
        attribution: format!("Attribution {id}"),
        copyright_url: None,
        remote_url: format!("https://www.bing.com/th?id=OHR.{id}"),
        cache_file_name: format!("{start_date}-{id}.jpg"),
        preview_file_name: None,
        cached,
    }
}

fn write_cached_state(root: &Path, wallpaper: &BingWallpaperItem) {
    let image_path = root.join(&wallpaper.cache_file_name);
    RgbImage::from_pixel(2, 2, Rgb([24, 92, 146]))
        .save(&image_path)
        .expect("test jpeg");
    let index = json!({
        "schemaVersion": 1,
        "refreshedAt": "2026-07-15T00:00:00Z",
        "market": "zh-CN",
        "items": [{
            "id": wallpaper.id,
            "startDate": wallpaper.start_date,
            "title": wallpaper.title,
            "attribution": wallpaper.attribution,
            "remoteUrl": wallpaper.remote_url,
            "cacheFileName": wallpaper.cache_file_name,
            "cached": true
        }]
    });
    fs::write(
        root.join("index.json"),
        serde_json::to_vec_pretty(&index).expect("serialize index"),
    )
    .expect("write index");
}

fn write_cache_index(root: &Path, wallpapers: &[BingWallpaperItem]) {
    let items = wallpapers
        .iter()
        .map(|wallpaper| {
            json!({
                "id": wallpaper.id,
                "startDate": wallpaper.start_date,
                "title": wallpaper.title,
                "attribution": wallpaper.attribution,
                "remoteUrl": wallpaper.remote_url,
                "cacheFileName": wallpaper.cache_file_name,
                "cached": wallpaper.cached
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        root.join("index.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "refreshedAt": "old-refresh",
            "market": "zh-CN",
            "items": items
        }))
        .expect("serialize cache index"),
    )
    .expect("write cache index");
}

#[test]
fn parses_success_fixture_and_normalizes_bing_fields() {
    let items = parse_bing_archive(&fixture("bing_wallpaper_success.json"))
        .expect("success response should parse");

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "quiet-bay-hash");
    assert_eq!(items[0].start_date, "20260715");
    assert!(items[0]
        .remote_url
        .starts_with("https://www.bing.com/th?id="));
    assert_eq!(
        items[0].copyright_url.as_deref(),
        Some("https://www.bing.com/search?q=Quiet+Bay&form=hpcapt&mkt=zh-cn")
    );
    assert_eq!(items[1].cache_file_name, "20260714-forestlighthash.jpg");
}

#[test]
fn parser_handles_missing_fields_and_partial_results_conservatively() {
    let missing = parse_bing_archive(&fixture("bing_wallpaper_missing_fields.json"))
        .expect("required fields are enough");
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].title, "");
    assert_eq!(missing[0].attribution, "");
    assert_eq!(missing[0].copyright_url, None);
    assert!(!missing[0].id.is_empty());

    let partial = parse_bing_archive(&fixture("bing_wallpaper_partial_result.json"))
        .expect("valid entries should survive invalid siblings");
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].id, "valid-partial-hash");
    assert_eq!(partial[0].title, "Fallback title");
    assert_eq!(
        partial[0].attribution,
        "Fallback title (© Example Photographer)"
    );
}

#[test]
fn parser_rejects_invalid_urls_and_malformed_responses() {
    let invalid = parse_bing_archive(&fixture("bing_wallpaper_invalid_url.json"))
        .expect_err("untrusted image URLs must be rejected");
    assert_eq!(invalid.code, "bing.no_valid_items");
    assert!(invalid.retryable);

    let malformed = parse_bing_archive(&fixture("bing_wallpaper_malformed_response.json"))
        .expect_err("malformed response must be rejected");
    assert_eq!(malformed.code, "bing.response_invalid");
    assert!(malformed.retryable);
}

#[test]
fn merge_is_newest_first_deduplicated_and_bounded_to_ten() {
    let cached_duplicate = item("duplicate", "20260714", true);
    let mut remote = vec![item("duplicate", "20260715", false)];
    for day in 1..=11 {
        remote.push(item(
            &format!("item-{day}"),
            &format!("202606{day:02}"),
            false,
        ));
    }

    let merged = merge_wallpaper_items(remote, vec![cached_duplicate.clone()]);

    assert_eq!(merged.len(), 10);
    assert_eq!(merged[0].id, "duplicate");
    assert!(merged[0].cached);
    assert_eq!(merged[0].cache_file_name, cached_duplicate.cache_file_name);
    assert_eq!(
        merged
            .iter()
            .filter(|wallpaper| wallpaper.id == "duplicate")
            .count(),
        1
    );
    assert!(merged
        .windows(2)
        .all(|pair| pair[0].start_date >= pair[1].start_date));
}

#[test]
fn snapshot_skips_unsafe_cache_entries_and_reports_corrupt_indexes() {
    let root = TestDir::new("cache-recovery");
    fs::write(
        root.path().join("index.json"),
        serde_json::to_vec(&json!({
            "schemaVersion": 1,
            "market": "zh-CN",
            "items": [
                {
                    "id": "unsafe",
                    "startDate": "20260715",
                    "title": "Unsafe",
                    "attribution": "Unsafe",
                    "remoteUrl": "https://www.bing.com/th?id=OHR.Unsafe",
                    "cacheFileName": "../outside.jpg",
                    "cached": true
                },
                {
                    "id": "foreign",
                    "startDate": "20260714",
                    "title": "Foreign",
                    "attribution": "Foreign",
                    "remoteUrl": "https://example.com/th?id=OHR.Foreign",
                    "cacheFileName": "20260714-foreign.jpg",
                    "cached": true
                }
            ]
        }))
        .expect("serialize cache"),
    )
    .expect("write cache");

    let state = BingWallpaperState::new(root.path());
    let safe_snapshot = state.snapshot();
    assert!(safe_snapshot.items.is_empty());

    fs::write(root.path().join("index.json"), "{not-json").expect("corrupt cache");
    let corrupt_snapshot = state.snapshot();
    assert!(corrupt_snapshot.items.is_empty());
    assert!(corrupt_snapshot.stale);
    assert_eq!(
        corrupt_snapshot
            .error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("cache.invalid")
    );
}

#[test]
fn refresh_downloads_partial_history_and_reopens_cache_first() {
    let root = TestDir::new("refresh-cache");
    let fetcher = Arc::new(FixtureFetcher::successful(fixture(
        "bing_wallpaper_success.json",
    )));
    let state = BingWallpaperState::with_fetcher(root.path(), fetcher.clone());

    let refreshed = tauri::async_runtime::block_on(state.refresh());

    assert_eq!(refreshed.items.len(), 2);
    assert!(refreshed.items.iter().all(|wallpaper| wallpaper.cached));
    assert!(!refreshed.stale);
    assert_eq!(fetcher.metadata_calls.load(Ordering::SeqCst), 1);
    assert_eq!(fetcher.image_calls.load(Ordering::SeqCst), 2);
    assert!(root.path().join("index.json").is_file());
    assert!(fs::read_dir(root.path())
        .expect("cache directory")
        .all(|entry| !entry
            .expect("cache entry")
            .file_name()
            .to_string_lossy()
            .ends_with(".part")));

    let reopened = BingWallpaperState::new(root.path()).snapshot();
    assert_eq!(reopened.items.len(), 2);
    assert!(reopened.items.iter().all(|wallpaper| wallpaper.cached));
}

#[test]
fn refresh_preserves_cache_on_http_failure_and_isolates_image_failures() {
    let root = TestDir::new("refresh-failures");
    let initial_fetcher = Arc::new(FixtureFetcher::successful(fixture(
        "bing_wallpaper_success.json",
    )));
    let initial = BingWallpaperState::with_fetcher(root.path(), initial_fetcher);
    let first = tauri::async_runtime::block_on(initial.refresh());
    assert_eq!(first.items.len(), 2);

    let mut offline_fetcher = FixtureFetcher::successful(fixture("bing_wallpaper_success.json"));
    offline_fetcher.metadata_status = 503;
    let offline = BingWallpaperState::with_fetcher(root.path(), Arc::new(offline_fetcher));
    let stale = tauri::async_runtime::block_on(offline.refresh());
    assert!(stale.stale);
    assert_eq!(stale.items.len(), 2);
    assert_eq!(
        stale.error.as_ref().map(|error| error.code.as_str()),
        Some("bing.http_status")
    );

    let partial_root = TestDir::new("partial-image");
    let mut partial_fetcher = FixtureFetcher::successful(fixture("bing_wallpaper_success.json"));
    partial_fetcher.failed_image_fragment = Some("ForestLight".into());
    let partial = BingWallpaperState::with_fetcher(partial_root.path(), Arc::new(partial_fetcher));
    let snapshot = tauri::async_runtime::block_on(partial.refresh());
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items.iter().filter(|item| item.cached).count(), 1);
    assert_eq!(
        snapshot.error.as_ref().map(|error| error.code.as_str()),
        Some("image.http_status")
    );
}

#[test]
fn concurrent_preview_requests_share_a_bounded_derivative_and_reference_counted_token() {
    let root = TestDir::new("preview-single-flight");
    let wallpaper = item("preview", "20260715", true);
    write_cached_state(root.path(), &wallpaper);
    let state = BingWallpaperState::new(root.path());

    let (first, second) = tauri::async_runtime::block_on(async {
        futures_util::future::join(state.preview("preview"), state.preview("preview")).await
    });
    let first = first.expect("first preview");
    let second = second.expect("second preview");
    assert_eq!(first, second);
    assert_eq!(first.mime_type, "image/jpeg");
    assert!(first.byte_length > 0 && first.byte_length <= 2 * 1024 * 1024);
    assert!(first.width <= 960 && first.height <= 600);

    let bytes = state
        .read_preview_bytes(&first.token)
        .expect("leased preview bytes");
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Jpeg
    );
    assert_eq!(bytes.len() as u64, first.byte_length);

    state.release_preview(&first.token);
    assert!(state.read_preview_bytes(&first.token).is_ok());
    state.release_preview(&second.token);
    assert_eq!(
        state.read_preview_bytes(&first.token).unwrap_err().code,
        "preview.token"
    );

    let serialized = serde_json::to_value(first).expect("serialize preview descriptor");
    assert!(serialized.get("dataUrl").is_none());
    assert_eq!(serialized["mimeType"], "image/jpeg");
}

#[test]
fn corrupt_preview_derivative_is_rebuilt_without_replacing_full_resolution_cache() {
    let root = TestDir::new("preview-rebuild");
    let wallpaper = item("rebuild", "20260715", true);
    write_cached_state(root.path(), &wallpaper);
    let state = BingWallpaperState::new(root.path());
    let first = tauri::async_runtime::block_on(state.preview("rebuild")).unwrap();
    let snapshot = state.snapshot();
    let preview_name = snapshot.items[0]
        .preview_file_name
        .as_deref()
        .expect("preview file name");
    let full_image_before = fs::read(root.path().join(&wallpaper.cache_file_name)).unwrap();
    fs::write(root.path().join(preview_name), b"corrupt preview").unwrap();
    state.release_preview(&first.token);

    let rebuilt = tauri::async_runtime::block_on(state.preview("rebuild")).unwrap();
    let bytes = state.read_preview_bytes(&rebuilt.token).unwrap();
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Jpeg
    );
    assert_eq!(
        fs::read(root.path().join(&wallpaper.cache_file_name)).unwrap(),
        full_image_before
    );
}

#[test]
fn simultaneous_refreshes_share_the_first_completed_result() {
    let root = TestDir::new("coalesced-refresh");
    let mut fixture_fetcher =
        FixtureFetcher::successful(fixture("bing_wallpaper_missing_fields.json"));
    fixture_fetcher.yield_metadata_once = true;
    let fetcher = Arc::new(fixture_fetcher);
    let state = BingWallpaperState::with_fetcher(root.path(), fetcher.clone());

    let (first, second) = tauri::async_runtime::block_on(async {
        futures_util::future::join(state.refresh(), state.refresh()).await
    });

    assert_eq!(first.items, second.items);
    assert_eq!(fetcher.metadata_calls.load(Ordering::SeqCst), 1);
    assert_eq!(fetcher.image_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn simultaneous_failed_refreshes_share_the_same_stale_error() {
    let root = TestDir::new("coalesced-refresh-error");
    let mut fixture_fetcher =
        FixtureFetcher::successful(fixture("bing_wallpaper_missing_fields.json"));
    fixture_fetcher.metadata_status = 503;
    fixture_fetcher.yield_metadata_once = true;
    let fetcher = Arc::new(fixture_fetcher);
    let state = BingWallpaperState::with_fetcher(root.path(), fetcher.clone());

    let (first, second) = tauri::async_runtime::block_on(async {
        futures_util::future::join(state.refresh(), state.refresh()).await
    });

    assert!(first.stale);
    assert_eq!(first, second);
    assert_eq!(fetcher.metadata_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn refresh_retention_removes_only_obsolete_index_owned_files() {
    let root = TestDir::new("retention");
    let mut cached = Vec::new();
    for day in 1..=10 {
        let wallpaper = item(&format!("old-{day}"), &format!("202606{day:02}"), true);
        RgbImage::from_pixel(2, 2, Rgb([day as u8, 40, 80]))
            .save(root.path().join(&wallpaper.cache_file_name))
            .expect("old cache image");
        cached.push(wallpaper);
    }
    write_cache_index(root.path(), &cached);
    fs::write(root.path().join("keep-me.txt"), "user file").expect("unknown file");
    let fetcher = Arc::new(FixtureFetcher::successful(fixture(
        "bing_wallpaper_success.json",
    )));
    let state = BingWallpaperState::with_fetcher(root.path(), fetcher);

    let refreshed = tauri::async_runtime::block_on(state.refresh());

    assert_eq!(refreshed.items.len(), 10);
    assert!(root.path().join("keep-me.txt").is_file());
    let retained_files = cached
        .iter()
        .filter(|wallpaper| root.path().join(&wallpaper.cache_file_name).is_file())
        .count();
    assert_eq!(retained_files, 8);
}

#[test]
fn wallpaper_contract_serializes_stable_camel_case_and_rejects_bad_shapes() {
    let snapshot = BingWallpaperSnapshot {
        items: vec![item("stable", "20260715", true)],
        refreshed_at: None,
        market: "zh-CN".into(),
        stale: false,
        platform: zero_lib::services::bing_wallpaper::platform_capability(),
        error: None,
    };
    let serialized = serde_json::to_value(snapshot).expect("serialize snapshot");

    assert_eq!(serialized["items"][0]["startDate"], "20260715");
    assert_eq!(
        serialized["items"][0]["cacheFileName"],
        "20260715-stable.jpg"
    );
    assert_eq!(serialized["items"][0]["cached"], true);
    assert!(serialized.get("refreshedAt").is_none());
    assert!(serialized.get("error").is_none());
    assert!(serde_json::from_value::<BingWallpaperSnapshot>(json!({
        "items": [{ "id": 42 }],
        "market": "zh-CN",
        "stale": false,
        "platform": { "platform": "macos", "supported": true }
    }))
    .is_err());
}

#[test]
fn cached_wallpaper_apply_and_download_actions_are_validated() {
    let root = TestDir::new("actions");
    let downloads = TestDir::new("downloads");
    let wallpaper = item("action", "20260715", true);
    write_cached_state(root.path(), &wallpaper);
    let state = BingWallpaperState::new(root.path());
    let setter = RecordingSetter::default();

    let applied = tauri::async_runtime::block_on(state.apply("action", &setter))
        .expect("cached wallpaper should apply");
    assert_eq!(applied.wallpaper_id, "action");
    assert_eq!(setter.paths.lock().expect("paths lock").len(), 1);

    fs::write(
        downloads.path().join("20260715-bing-wallpaper.jpg"),
        "existing",
    )
    .expect("collision seed");
    let saved = tauri::async_runtime::block_on(state.save_to_downloads("action", downloads.path()))
        .expect("cached wallpaper should save");
    assert!(saved.path.ends_with("20260715-bing-wallpaper-2.jpg"));
    assert!(Path::new(&saved.path).is_file());
}

#[test]
fn wallpaper_backend_failures_and_unknown_ids_do_not_report_success() {
    let root = TestDir::new("action-errors");
    let wallpaper = item("failure", "20260715", true);
    write_cached_state(root.path(), &wallpaper);
    let state = BingWallpaperState::new(root.path());
    let setter = RecordingSetter {
        paths: Mutex::new(Vec::new()),
        error: Some(NativeResourceError {
            operation: "system.setWallpaper".into(),
            code: "wallpaper.backend".into(),
            message: "backend rejected request".into(),
            retryable: true,
        }),
    };

    let backend_error = tauri::async_runtime::block_on(state.apply("failure", &setter))
        .expect_err("backend failure should propagate");
    assert_eq!(backend_error.code, "wallpaper.backend");
    assert!(backend_error.retryable);

    let missing = tauri::async_runtime::block_on(state.apply("missing", &setter))
        .expect_err("unknown ID should fail");
    assert_eq!(missing.code, "wallpaper.not_found");
    assert!(!missing.retryable);
}

#[test]
fn apply_downloads_a_missing_cached_image_before_invoking_backend() {
    let root = TestDir::new("apply-download");
    let wallpaper = item("on-demand", "20260715", false);
    write_cache_index(root.path(), std::slice::from_ref(&wallpaper));
    let fetcher = Arc::new(FixtureFetcher::successful(fixture(
        "bing_wallpaper_missing_fields.json",
    )));
    let state = BingWallpaperState::with_fetcher(root.path(), fetcher.clone());
    let setter = RecordingSetter::default();

    let result = tauri::async_runtime::block_on(state.apply("on-demand", &setter))
        .expect("missing image should download before apply");

    assert_eq!(result.wallpaper_id, "on-demand");
    assert_eq!(fetcher.image_calls.load(Ordering::SeqCst), 1);
    assert!(root.path().join(&wallpaper.cache_file_name).is_file());
    assert_eq!(setter.paths.lock().expect("paths lock").len(), 1);
}

#[test]
fn save_reports_an_unavailable_download_destination() {
    let root = TestDir::new("download-unavailable-cache");
    let blocked_parent = TestDir::new("download-unavailable-target");
    let wallpaper = item("blocked", "20260715", true);
    write_cached_state(root.path(), &wallpaper);
    let blocked_path = blocked_parent.path().join("not-a-directory");
    fs::write(&blocked_path, "file").expect("blocking file");
    let state = BingWallpaperState::new(root.path());

    let error = tauri::async_runtime::block_on(state.save_to_downloads("blocked", &blocked_path))
        .expect_err("file destination cannot be a Downloads directory");

    assert_eq!(error.code, "downloads.unavailable");
    assert!(error.retryable);
}
