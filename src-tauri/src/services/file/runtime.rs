use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Emitter, Manager};

use super::artifacts::{
    cleanup_stale_job_directories, commit_provider_output, create_job_temp_directory,
    remove_job_temp_directory,
};
use super::capabilities::{capability_snapshot, default_provider_registry};
use super::contracts::{
    FileConversionBatchResult, FileConversionCandidate, FileConversionCandidateValidation,
    FileConversionCapabilitySnapshot, FileConversionEnqueueRequest, FileConversionError,
    FileConversionErrorCode, FileConversionJobSnapshot, FileConversionJobState,
    FileConversionProgress, FileConversionResult, FileConversionStage,
};
use super::engine_bridge::FileEngineBridge;
use super::output::reserve_output_path;
use super::provider::{
    FileConversionCancellationToken, FileConversionProgressSink, FileConversionProviderRegistry,
    ProviderConversionRequest, ProviderPlatform,
};
use super::queue::{FileConversionJobDraft, FileConversionJobRecord, FileConversionQueue};
use super::validation::inspect_source;

pub const FILE_CONVERSION_JOB_UPDATED_EVENT: &str = "zero://file-conversion/job-updated";
const MAX_ENQUEUE_ITEMS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCapabilityInvalidationCause {
    EngineInstalled,
    EngineUpgraded,
    EngineRepaired,
    EngineRemoved,
    NativeProviderChanged,
    LifecycleReset,
}

struct CapabilityCacheState {
    generation: u64,
    snapshot: Option<FileConversionCapabilitySnapshot>,
    last_invalidation: Option<FileCapabilityInvalidationCause>,
}

impl Default for CapabilityCacheState {
    fn default() -> Self {
        Self {
            generation: 1,
            snapshot: None,
            last_invalidation: None,
        }
    }
}

#[derive(Default)]
struct RuntimeState {
    queue: FileConversionQueue,
    reserved_paths: HashSet<PathBuf>,
    cancellations: HashMap<String, FileConversionCancellationToken>,
    worker_running: bool,
    temp_root: Option<PathBuf>,
    next_job_number: u64,
}

pub struct FileConversionState {
    runtime: Mutex<RuntimeState>,
    enqueue_gate: Mutex<()>,
    providers: Mutex<Option<Arc<FileConversionProviderRegistry>>>,
    capability_cache: Mutex<CapabilityCacheState>,
    provider_initializations: AtomicU64,
    capability_refreshes: AtomicU64,
}

impl Default for FileConversionState {
    fn default() -> Self {
        Self {
            runtime: Mutex::new(RuntimeState::default()),
            enqueue_gate: Mutex::new(()),
            providers: Mutex::new(None),
            capability_cache: Mutex::new(CapabilityCacheState::default()),
            provider_initializations: AtomicU64::new(0),
            capability_refreshes: AtomicU64::new(0),
        }
    }
}

impl FileConversionState {
    pub fn initialize(&self, temp_root: PathBuf) -> Result<(), FileConversionError> {
        self.initialize_registry(None, Arc::new(FileEngineBridge::default()), temp_root)
    }

    pub fn initialize_with_engine(
        &self,
        app: tauri::AppHandle,
        bridge: Arc<FileEngineBridge>,
        temp_root: PathBuf,
    ) -> Result<(), FileConversionError> {
        self.initialize_registry(Some(app), bridge, temp_root)
    }

    fn initialize_registry(
        &self,
        app: Option<tauri::AppHandle>,
        bridge: Arc<FileEngineBridge>,
        temp_root: PathBuf,
    ) -> Result<(), FileConversionError> {
        let mut providers = self
            .providers
            .lock()
            .map_err(|_| runtime_error("The File provider registry is unavailable.", true))?;
        if providers.is_some() {
            return Ok(());
        }
        cleanup_stale_job_directories(&temp_root)?;
        let registry = Arc::new(default_provider_registry(app, bridge));
        self.lock_runtime()?.temp_root = Some(temp_root);
        *providers = Some(registry);
        self.provider_initializations
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn capabilities(&self) -> FileConversionCapabilitySnapshot {
        loop {
            let generation = self.capability_generation();
            if let Ok(cache) = self.capability_cache.lock() {
                if cache.generation == generation {
                    if let Some(snapshot) = &cache.snapshot {
                        return snapshot.clone();
                    }
                }
            }
            let providers = self
                .providers
                .lock()
                .ok()
                .and_then(|providers| providers.as_ref().map(Arc::clone))
                .unwrap_or_else(|| Arc::new(FileConversionProviderRegistry::new(Vec::new())));
            let snapshot = capability_snapshot(&providers, ProviderPlatform::current(), now_ms());
            let Ok(mut cache) = self.capability_cache.lock() else {
                return snapshot;
            };
            if cache.generation != generation {
                continue;
            }
            cache.snapshot = Some(snapshot.clone());
            self.capability_refreshes.fetch_add(1, Ordering::Relaxed);
            return snapshot;
        }
    }

    pub fn invalidate_capabilities(&self, cause: FileCapabilityInvalidationCause) -> u64 {
        if let Ok(providers) = self.providers.lock() {
            if let Some(providers) = providers.as_ref() {
                providers.invalidate();
            }
        }
        let mut cache = match self.capability_cache.lock() {
            Ok(cache) => cache,
            Err(poisoned) => poisoned.into_inner(),
        };
        cache.generation = cache.generation.saturating_add(1);
        cache.snapshot = None;
        cache.last_invalidation = Some(cause);
        cache.generation
    }

    pub fn capability_generation(&self) -> u64 {
        self.capability_cache
            .lock()
            .map(|cache| cache.generation)
            .unwrap_or_default()
    }

    pub fn capability_refresh_count(&self) -> u64 {
        self.capability_refreshes.load(Ordering::Relaxed)
    }

    pub fn provider_initialization_count(&self) -> u64 {
        self.provider_initializations.load(Ordering::Relaxed)
    }

    pub fn inspect_paths(&self, source_paths: Vec<String>) -> Vec<FileConversionCandidate> {
        let mut active_sources = self
            .runtime
            .lock()
            .map(|runtime| runtime.queue.active_sources())
            .unwrap_or_default();
        source_paths
            .into_iter()
            .map(|source_path| {
                let candidate = inspect_source(Path::new(&source_path), &active_sources);
                if matches!(
                    candidate.validation,
                    FileConversionCandidateValidation::Valid { .. }
                ) {
                    active_sources.insert(PathBuf::from(&candidate.source_path));
                }
                candidate
            })
            .collect()
    }

    pub fn enqueue(
        &self,
        request: FileConversionEnqueueRequest,
    ) -> Result<FileConversionBatchResult, FileConversionError> {
        if request.items.is_empty() || request.items.len() > MAX_ENQUEUE_ITEMS {
            return Err(FileConversionError {
                code: FileConversionErrorCode::InvalidInput,
                message: "A conversion batch must contain between 1 and 100 files.".into(),
                retryable: false,
                provider_id: None,
                diagnostic: None,
            });
        }
        let _enqueue = self
            .enqueue_gate
            .lock()
            .map_err(|_| runtime_error("The File enqueue coordinator is unavailable.", true))?;
        let (mut active_sources, mut reserved_paths) = {
            let runtime = self.lock_runtime()?;
            (
                runtime.queue.active_sources(),
                runtime.reserved_paths.clone(),
            )
        };
        let mut prepared = Vec::new();
        let mut rejected_candidates = Vec::new();

        for item in request.items {
            let candidate = inspect_source(Path::new(&item.source_path), &active_sources);
            let direction = match &candidate.validation {
                FileConversionCandidateValidation::Valid { direction, .. } => *direction,
                FileConversionCandidateValidation::Rejected { .. } => {
                    rejected_candidates.push(candidate);
                    continue;
                }
            };
            let canonical_source = PathBuf::from(&candidate.source_path);
            let reservation = match reserve_output_path(
                &canonical_source,
                direction,
                item.output_directory.as_deref().map(Path::new),
                &mut reserved_paths,
            ) {
                Ok(reservation) => reservation,
                Err(error) => {
                    rejected_candidates.push(rejected_from_candidate(candidate, error));
                    continue;
                }
            };
            active_sources.insert(canonical_source.clone());
            prepared.push((candidate, canonical_source, direction, reservation));
        }

        let mut runtime = self.lock_runtime()?;
        let mut jobs = Vec::new();
        for (candidate, canonical_source, direction, reservation) in prepared {
            runtime.next_job_number += 1;
            let draft = FileConversionJobDraft {
                id: format!("file-{}-{}", now_ms(), runtime.next_job_number),
                canonical_source: canonical_source.clone(),
                final_output: reservation.output_path.clone(),
                source_name: candidate.source_name.clone(),
                size_bytes: candidate.size_bytes.unwrap_or(0),
                direction,
                target_name: reservation.output_name,
            };
            match runtime.queue.enqueue(draft, now_ms()) {
                Ok(snapshot) => {
                    runtime.reserved_paths.insert(reservation.output_path);
                    jobs.push(snapshot);
                }
                Err(error) => {
                    rejected_candidates.push(rejected_from_candidate(candidate, error));
                }
            }
        }

        Ok(FileConversionBatchResult {
            jobs,
            rejected_candidates,
        })
    }

    pub fn snapshots(&self) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
        Ok(self.lock_runtime()?.queue.snapshots())
    }

    pub fn start(&self) -> Result<(Vec<FileConversionJobSnapshot>, bool), FileConversionError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.temp_root.is_none() {
            return Err(runtime_error(
                "The File conversion workspace has not been initialized.",
                true,
            ));
        }
        let updates = runtime
            .queue
            .start(now_ms())
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(snapshot) = updates.first() {
            runtime
                .cancellations
                .entry(snapshot.id.clone())
                .or_default();
        }
        let should_spawn = !updates.is_empty() && !runtime.worker_running;
        if should_spawn {
            runtime.worker_running = true;
        }
        Ok((updates, should_spawn))
    }

    pub fn cancel(
        &self,
        job_id: &str,
    ) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.queue.active_job_id() == Some(job_id) {
            runtime
                .cancellations
                .entry(job_id.to_string())
                .or_default()
                .cancel();
            return runtime
                .queue
                .record(job_id)
                .map(|record| vec![record.snapshot.clone()])
                .ok_or_else(unknown_job);
        }
        runtime.queue.cancel(job_id, now_ms())
    }

    pub fn remove(&self, job_id: &str) -> Result<FileConversionJobSnapshot, FileConversionError> {
        let mut runtime = self.lock_runtime()?;
        let final_output = runtime
            .queue
            .record(job_id)
            .map(|record| record.final_output.clone())
            .ok_or_else(unknown_job)?;
        let removed = runtime.queue.remove(job_id)?;
        runtime.reserved_paths.remove(&final_output);
        runtime.cancellations.remove(job_id);
        Ok(removed)
    }

    pub fn retry(&self, job_id: &str) -> Result<FileConversionJobSnapshot, FileConversionError> {
        self.lock_runtime()?.queue.retry(job_id, now_ms())
    }

    pub fn clear_completed(&self) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
        let mut runtime = self.lock_runtime()?;
        let completed = runtime
            .queue
            .snapshots()
            .into_iter()
            .filter(|job| matches!(job.state, FileConversionJobState::Completed { .. }))
            .collect::<Vec<_>>();
        for job in &completed {
            if let Some(record) = runtime.queue.record(&job.id) {
                let final_output = record.final_output.clone();
                runtime.reserved_paths.remove(&final_output);
            }
            runtime.cancellations.remove(&job.id);
        }
        Ok(runtime.queue.clear_completed())
    }

    pub(crate) fn completed_record(
        &self,
        job_id: &str,
    ) -> Result<FileConversionJobRecord, FileConversionError> {
        self.lock_runtime()?
            .queue
            .record(job_id)
            .cloned()
            .ok_or_else(unknown_job)
    }

    pub fn shutdown_cleanup(&self) {
        let temp_root = if let Ok(runtime) = self.runtime.lock() {
            for cancellation in runtime.cancellations.values() {
                cancellation.cancel();
            }
            runtime.temp_root.clone()
        } else {
            None
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let worker_running = self
                .runtime
                .lock()
                .map(|runtime| runtime.worker_running)
                .unwrap_or(false);
            if !worker_running {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        if let Some(temp_root) = temp_root {
            let worker_running = self
                .runtime
                .lock()
                .map(|runtime| runtime.worker_running)
                .unwrap_or(true);
            if !worker_running {
                let _ = cleanup_stale_job_directories(&temp_root);
            }
        }
    }

    fn lock_runtime(&self) -> Result<std::sync::MutexGuard<'_, RuntimeState>, FileConversionError> {
        self.runtime.lock().map_err(|_| {
            runtime_error(
                "The File conversion queue is temporarily unavailable.",
                true,
            )
        })
    }
}

pub async fn run_file_conversion_worker(app: tauri::AppHandle) {
    loop {
        let active = {
            let state = app.state::<FileConversionState>();
            let Ok(mut runtime) = state.lock_runtime() else {
                return;
            };
            let Some(job_id) = runtime.queue.active_job_id().map(str::to_string) else {
                runtime.worker_running = false;
                drop(runtime);
                let bridge = Arc::clone(
                    &app.state::<super::engine_bridge::FileEngineBridgeState>()
                        .bridge,
                );
                bridge.schedule_idle_teardown(&app);
                return;
            };
            let Some(record) = runtime.queue.record(&job_id).cloned() else {
                runtime.worker_running = false;
                return;
            };
            let cancellation = runtime.cancellations.entry(job_id).or_default().clone();
            let Some(temp_root) = runtime.temp_root.clone() else {
                runtime.worker_running = false;
                return;
            };
            (record, cancellation, temp_root)
        };
        execute_active_job(&app, active.0, active.1, active.2).await;
    }
}

async fn execute_active_job(
    app: &tauri::AppHandle,
    record: FileConversionJobRecord,
    cancellation: FileConversionCancellationToken,
    temp_root: PathBuf,
) {
    let state = app.state::<FileConversionState>();
    if let Ok(snapshot) = state.lock_runtime().and_then(|mut runtime| {
        runtime.queue.set_preparing_stage(
            &record.snapshot.id,
            FileConversionStage::WaitingForProvider,
            now_ms(),
        )
    }) {
        emit_snapshot(app, &snapshot);
    }
    if cancellation.is_cancelled() {
        finish_with_error(app, &record.snapshot.id, cancelled_error());
        return;
    }

    let providers = match state.providers.lock() {
        Ok(providers) => providers.as_ref().map(Arc::clone),
        Err(_) => {
            finish_with_error(
                app,
                &record.snapshot.id,
                runtime_error("The File provider registry is unavailable.", true),
            );
            return;
        }
    };
    let Some(providers) = providers else {
        finish_with_error(
            app,
            &record.snapshot.id,
            runtime_error("The File provider registry has not been initialized.", true),
        );
        return;
    };
    let provider = match providers.select(record.snapshot.direction, ProviderPlatform::current()) {
        Ok(provider) => provider,
        Err(error) => {
            finish_with_error(app, &record.snapshot.id, error);
            return;
        }
    };
    let job_directory = match create_job_temp_directory(&temp_root, &record.snapshot.id) {
        Ok(directory) => directory,
        Err(error) => {
            finish_with_error(app, &record.snapshot.id, error);
            return;
        }
    };
    match state.lock_runtime().and_then(|mut runtime| {
        runtime.queue.mark_running(
            &record.snapshot.id,
            provider.id(),
            FileConversionProgress::Indeterminate {
                stage: FileConversionStage::Converting,
            },
            now_ms(),
        )
    }) {
        Ok(snapshot) => emit_snapshot(app, &snapshot),
        Err(error) => {
            let _ = remove_job_temp_directory(&temp_root, &job_directory);
            finish_with_error(app, &record.snapshot.id, error);
            return;
        }
    }

    let progress_sink = RuntimeProgressSink {
        app: app.clone(),
        job_id: record.snapshot.id.clone(),
    };
    let request = ProviderConversionRequest {
        job_id: record.snapshot.id.clone(),
        direction: record.snapshot.direction,
        source_path: record.canonical_source.clone(),
        temp_directory: job_directory.clone(),
    };
    let provider_id = provider.id();
    let provider_result = tauri::async_runtime::spawn_blocking(move || {
        provider.convert(&request, &progress_sink, &cancellation)
    })
    .await
    .map_err(|_| runtime_error("The local conversion worker stopped unexpectedly.", true))
    .and_then(|result| result);

    let final_result = provider_result.and_then(|output| {
        commit_provider_output(
            &job_directory,
            &output.path,
            &record.final_output,
            record.snapshot.direction,
            &record.snapshot.id,
        )?;
        let metadata = std::fs::metadata(&record.final_output)
            .map_err(|_| runtime_error("The completed output could not be revalidated.", true))?;
        Ok(FileConversionResult {
            output_path: record.final_output.to_string_lossy().into_owned(),
            output_name: record.snapshot.target_name.clone(),
            size_bytes: metadata.len(),
            completed_at_ms: now_ms(),
            provider_id,
            provider_origin: output.provider_origin,
            engine_version: output.engine_version,
            quality_profile: output.quality_profile,
            warning_keys: output.warning_keys,
            page_count: output.page_count,
        })
    });
    let _ = remove_job_temp_directory(&temp_root, &job_directory);

    match final_result {
        Ok(result) => finish_completed(app, &record.snapshot.id, result),
        Err(error) => finish_with_error(app, &record.snapshot.id, error),
    }
}

struct RuntimeProgressSink {
    app: tauri::AppHandle,
    job_id: String,
}

impl FileConversionProgressSink for RuntimeProgressSink {
    fn report(&self, progress: FileConversionProgress) -> Result<(), FileConversionError> {
        let snapshot = self
            .app
            .state::<FileConversionState>()
            .lock_runtime()?
            .queue
            .update_progress(&self.job_id, progress, now_ms())?;
        emit_snapshot(&self.app, &snapshot);
        Ok(())
    }
}

fn finish_completed(app: &tauri::AppHandle, job_id: &str, result: FileConversionResult) {
    let state = app.state::<FileConversionState>();
    if let Ok(updates) = state.lock_runtime().and_then(|mut runtime| {
        runtime.cancellations.remove(job_id);
        let updates = runtime.queue.complete(job_id, result, now_ms())?;
        register_next_cancellation(&mut runtime, &updates);
        Ok(updates)
    }) {
        updates.iter().for_each(|update| emit_snapshot(app, update));
    }
}

fn finish_with_error(app: &tauri::AppHandle, job_id: &str, error: FileConversionError) {
    let state = app.state::<FileConversionState>();
    if let Ok(updates) = state.lock_runtime().and_then(|mut runtime| {
        runtime.cancellations.remove(job_id);
        let updates = if error.code == FileConversionErrorCode::Cancelled {
            runtime.queue.cancel(job_id, now_ms())?
        } else {
            runtime.queue.fail(job_id, error, now_ms())?
        };
        register_next_cancellation(&mut runtime, &updates);
        Ok(updates)
    }) {
        updates.iter().for_each(|update| emit_snapshot(app, update));
    }
}

fn register_next_cancellation(runtime: &mut RuntimeState, updates: &[FileConversionJobSnapshot]) {
    for update in updates {
        if matches!(update.state, FileConversionJobState::Preparing { .. }) {
            runtime.cancellations.entry(update.id.clone()).or_default();
        }
    }
}

fn emit_snapshot(app: &tauri::AppHandle, snapshot: &FileConversionJobSnapshot) {
    let _ = app.emit(FILE_CONVERSION_JOB_UPDATED_EVENT, snapshot.clone());
}

fn rejected_from_candidate(
    candidate: FileConversionCandidate,
    error: FileConversionError,
) -> FileConversionCandidate {
    FileConversionCandidate {
        validation: FileConversionCandidateValidation::Rejected { error },
        ..candidate
    }
}

fn cancelled_error() -> FileConversionError {
    FileConversionError {
        code: FileConversionErrorCode::Cancelled,
        message: "The conversion was cancelled.".into(),
        retryable: true,
        provider_id: None,
        diagnostic: None,
    }
}

fn unknown_job() -> FileConversionError {
    FileConversionError {
        code: FileConversionErrorCode::UnknownJob,
        message: "The conversion job was not found.".into(),
        retryable: false,
        provider_id: None,
        diagnostic: None,
    }
}

fn runtime_error(message: &str, retryable: bool) -> FileConversionError {
    FileConversionError {
        code: FileConversionErrorCode::Internal,
        message: message.into(),
        retryable,
        provider_id: None,
        diagnostic: None,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::file::contracts::{
        FileConversionEnqueueItem, FileConversionEnqueueRequest,
    };

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-runtime-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn mixed_enqueue_keeps_valid_jobs_and_returns_inline_rejections_without_starting() {
        let root = TestRoot::new();
        let valid = root.0.join("report.pdf");
        let unsupported = root.0.join("legacy.doc");
        fs::write(&valid, b"%PDF-1.7\nfixture").unwrap();
        fs::write(&unsupported, b"fixture").unwrap();
        let state = FileConversionState::default();
        state.initialize(root.0.join("temp")).unwrap();

        let result = state
            .enqueue(FileConversionEnqueueRequest {
                items: vec![
                    FileConversionEnqueueItem {
                        source_path: valid.to_string_lossy().into_owned(),
                        output_directory: None,
                    },
                    FileConversionEnqueueItem {
                        source_path: unsupported.to_string_lossy().into_owned(),
                        output_directory: None,
                    },
                ],
            })
            .unwrap();

        assert_eq!(result.jobs.len(), 1);
        assert_eq!(result.rejected_candidates.len(), 1);
        assert!(matches!(
            result.jobs[0].state,
            FileConversionJobState::Queued
        ));
        assert!(state
            .lock_runtime()
            .unwrap()
            .queue
            .active_job_id()
            .is_none());
    }

    #[test]
    fn concurrent_enqueues_publish_unique_jobs_without_holding_runtime_during_inspection() {
        let root = TestRoot::new();
        let sources = [
            root.0.join("first-race.pdf"),
            root.0.join("second-race.pdf"),
        ];
        for source in &sources {
            fs::write(source, b"%PDF-1.7\nfixture").unwrap();
        }
        let state = Arc::new(FileConversionState::default());
        state.initialize(root.0.join("temp")).unwrap();
        let workers = sources.map(|source| {
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                state
                    .enqueue(FileConversionEnqueueRequest {
                        items: vec![FileConversionEnqueueItem {
                            source_path: source.to_string_lossy().into_owned(),
                            output_directory: None,
                        }],
                    })
                    .unwrap()
            })
        });
        let results = workers.map(|worker| worker.join().unwrap());
        assert!(results.iter().all(|result| result.jobs.len() == 1));
        let snapshots = state.snapshots().unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_ne!(snapshots[0].id, snapshots[1].id);
    }

    #[test]
    fn explicit_start_and_job_scoped_actions_use_the_authoritative_queue() {
        let root = TestRoot::new();
        let first = root.0.join("first.pdf");
        let second = root.0.join("second.pdf");
        fs::write(&first, b"%PDF-1.7\nfixture").unwrap();
        fs::write(&second, b"%PDF-1.7\nfixture").unwrap();
        let state = FileConversionState::default();
        state.initialize(root.0.join("temp")).unwrap();
        let result = state
            .enqueue(FileConversionEnqueueRequest {
                items: [first, second]
                    .into_iter()
                    .map(|path| FileConversionEnqueueItem {
                        source_path: path.to_string_lossy().into_owned(),
                        output_directory: None,
                    })
                    .collect(),
            })
            .unwrap();
        let queued_second = result.jobs[1].id.clone();

        let (updates, should_spawn) = state.start().unwrap();
        assert!(should_spawn);
        assert_eq!(updates.len(), 1);
        assert!(matches!(
            updates[0].state,
            FileConversionJobState::Preparing { .. }
        ));

        let cancelled = state.cancel(&queued_second).unwrap();
        assert!(matches!(
            cancelled[0].state,
            FileConversionJobState::Cancelled { .. }
        ));
        state.remove(&queued_second).unwrap();
        assert_eq!(state.snapshots().unwrap().len(), 1);
    }

    #[test]
    fn a_new_runtime_has_no_persisted_job_history() {
        let state = FileConversionState::default();
        assert_eq!(state.provider_initialization_count(), 0);
        assert!(state.snapshots().unwrap().is_empty());
        assert_eq!(
            state.remove("missing").unwrap_err().code,
            FileConversionErrorCode::UnknownJob
        );
        assert_eq!(
            state.cancel("missing").unwrap_err().code,
            FileConversionErrorCode::UnknownJob
        );
    }

    #[test]
    fn provider_registry_initializes_once_for_repeated_requests() {
        let root = TestRoot::new();
        let state = FileConversionState::default();
        state.initialize(root.0.join("temp")).unwrap();
        state.initialize(root.0.join("different-temp")).unwrap();

        assert_eq!(state.provider_initialization_count(), 1);
        let first = state.capabilities();
        let second = state.capabilities();
        assert!(!first.directions.is_empty());
        assert_eq!(first, second);
        assert_eq!(state.capability_refresh_count(), 1);
    }

    #[test]
    fn every_supported_cause_advances_generation_and_rebuilds_once_on_demand() {
        let root = TestRoot::new();
        let state = FileConversionState::default();
        state.initialize(root.0.join("temp")).unwrap();
        let _ = state.capabilities();
        let causes = [
            FileCapabilityInvalidationCause::EngineInstalled,
            FileCapabilityInvalidationCause::EngineUpgraded,
            FileCapabilityInvalidationCause::EngineRepaired,
            FileCapabilityInvalidationCause::EngineRemoved,
            FileCapabilityInvalidationCause::NativeProviderChanged,
            FileCapabilityInvalidationCause::LifecycleReset,
        ];
        let mut generation = state.capability_generation();
        for (index, cause) in causes.into_iter().enumerate() {
            let next_generation = state.invalidate_capabilities(cause);
            assert_eq!(next_generation, generation + 1);
            generation = next_generation;
            let _ = state.capabilities();
            let _ = state.capabilities();
            assert_eq!(state.capability_refresh_count(), index as u64 + 2);
        }
    }

    #[test]
    fn rejects_empty_and_unbounded_batches_before_touching_the_queue() {
        let state = FileConversionState::default();
        for count in [0, MAX_ENQUEUE_ITEMS + 1] {
            let error = state
                .enqueue(FileConversionEnqueueRequest {
                    items: (0..count)
                        .map(|index| FileConversionEnqueueItem {
                            source_path: format!("/tmp/{index}.pdf"),
                            output_directory: None,
                        })
                        .collect(),
                })
                .unwrap_err();
            assert_eq!(error.code, FileConversionErrorCode::InvalidInput);
        }
        assert!(state.snapshots().unwrap().is_empty());
    }

    #[test]
    fn shutdown_cleanup_removes_only_zero_owned_stale_workspaces() {
        let root = TestRoot::new();
        let temp_root = root.0.join("temp");
        fs::create_dir_all(temp_root.join("job-stale")).unwrap();
        fs::create_dir_all(temp_root.join("keep-me")).unwrap();
        let state = FileConversionState::default();
        state.initialize(temp_root.clone()).unwrap();
        fs::create_dir_all(temp_root.join("job-current-stale")).unwrap();

        state.shutdown_cleanup();

        assert!(!temp_root.join("job-current-stale").exists());
        assert!(temp_root.join("keep-me").exists());
    }
}
