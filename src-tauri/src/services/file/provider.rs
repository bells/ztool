use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::contracts::{
    FileConversionDirection, FileConversionError, FileConversionErrorCode, FileConversionProgress,
    FileConversionProvider as FileConversionProviderSnapshot, FileConversionProviderAvailability,
    FileConversionProviderId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPlatform {
    Macos,
    Windows,
    Unsupported,
}

impl ProviderPlatform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        return Self::Macos;
        #[cfg(target_os = "windows")]
        return Self::Windows;
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return Self::Unsupported;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConversionRequest {
    pub job_id: String,
    pub direction: FileConversionDirection,
    pub source_path: PathBuf,
    pub temp_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConversionOutput {
    pub path: PathBuf,
}

pub trait FileConversionProgressSink: Send + Sync {
    fn report(&self, progress: FileConversionProgress) -> Result<(), FileConversionError>;
}

pub trait FileConversionProvider: Send + Sync {
    fn id(&self) -> FileConversionProviderId;
    fn supported_directions(&self) -> &[FileConversionDirection];
    fn probe(&self) -> FileConversionProviderSnapshot;
    fn convert(
        &self,
        request: &ProviderConversionRequest,
        progress: &dyn FileConversionProgressSink,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<ProviderConversionOutput, FileConversionError>;
}

#[derive(Debug, Clone, Default)]
pub struct FileConversionCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl FileConversionCancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn check(&self) -> Result<(), FileConversionError> {
        if self.is_cancelled() {
            Err(provider_error(
                FileConversionErrorCode::Cancelled,
                "The conversion was cancelled.",
                true,
                None,
            ))
        } else {
            Ok(())
        }
    }
}

pub struct FileConversionProviderRegistry {
    providers: Vec<Arc<dyn FileConversionProvider>>,
}

impl FileConversionProviderRegistry {
    pub fn new(providers: Vec<Arc<dyn FileConversionProvider>>) -> Self {
        Self { providers }
    }

    pub fn select(
        &self,
        direction: FileConversionDirection,
        platform: ProviderPlatform,
    ) -> Result<Arc<dyn FileConversionProvider>, FileConversionError> {
        for provider_id in provider_priority(direction, platform) {
            let Some(provider) = self.providers.iter().find(|provider| {
                provider.id() == provider_id && provider.supported_directions().contains(&direction)
            }) else {
                continue;
            };
            let snapshot = provider.probe();
            if snapshot.id == provider_id
                && snapshot.directions.contains(&direction)
                && matches!(
                    snapshot.availability,
                    FileConversionProviderAvailability::Available
                )
            {
                return Ok(Arc::clone(provider));
            }
        }

        Err(provider_error(
            FileConversionErrorCode::EngineUnavailable,
            "No approved local provider is available for this conversion direction.",
            true,
            None,
        ))
    }

    pub fn snapshots(&self) -> Vec<FileConversionProviderSnapshot> {
        self.providers
            .iter()
            .map(|provider| provider.probe())
            .collect()
    }
}

fn provider_priority(
    direction: FileConversionDirection,
    platform: ProviderPlatform,
) -> Vec<FileConversionProviderId> {
    match (direction, platform) {
        (FileConversionDirection::DocxToPdf, ProviderPlatform::Macos) => vec![
            FileConversionProviderId::LibreOffice,
            FileConversionProviderId::MicrosoftWordMacos,
        ],
        (FileConversionDirection::DocxToPdf, ProviderPlatform::Windows) => vec![
            FileConversionProviderId::LibreOffice,
            FileConversionProviderId::MicrosoftWordWindows,
        ],
        _ => Vec::new(),
    }
}

pub fn provider_output_path(request: &ProviderConversionRequest) -> PathBuf {
    let extension = match request.direction {
        FileConversionDirection::PdfToDocx => "docx",
        FileConversionDirection::DocxToPdf => "pdf",
    };
    request
        .temp_directory
        .join(format!("provider-output.{extension}"))
}

pub fn provider_error(
    code: FileConversionErrorCode,
    message: &str,
    retryable: bool,
    provider_id: Option<FileConversionProviderId>,
) -> FileConversionError {
    FileConversionError {
        code,
        message: message.into(),
        retryable,
        provider_id,
        diagnostic: None,
    }
}

pub fn source_file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::file::artifacts::validate_provider_output;
    use crate::services::file::contracts::FileConversionStage;

    #[derive(Debug, Clone)]
    enum FakeOutcome {
        Success,
        Failure,
        InvalidOutput,
        Timeout,
    }

    struct FakeProvider {
        id: FileConversionProviderId,
        directions: Vec<FileConversionDirection>,
        available: bool,
        outcome: FakeOutcome,
        progress: Vec<FileConversionProgress>,
    }

    impl FileConversionProvider for FakeProvider {
        fn id(&self) -> FileConversionProviderId {
            self.id
        }

        fn supported_directions(&self) -> &[FileConversionDirection] {
            &self.directions
        }

        fn probe(&self) -> FileConversionProviderSnapshot {
            FileConversionProviderSnapshot {
                id: self.id,
                display_name: format!("Fake {:?}", self.id),
                version: Some("1.0.0".into()),
                directions: self.directions.clone(),
                availability: if self.available {
                    FileConversionProviderAvailability::Available
                } else {
                    FileConversionProviderAvailability::Unavailable {
                        error: provider_error(
                            FileConversionErrorCode::EngineUnavailable,
                            "The fake provider is unavailable.",
                            true,
                            Some(self.id),
                        ),
                    }
                },
            }
        }

        fn convert(
            &self,
            request: &ProviderConversionRequest,
            progress: &dyn FileConversionProgressSink,
            cancellation: &FileConversionCancellationToken,
        ) -> Result<ProviderConversionOutput, FileConversionError> {
            cancellation.check()?;
            for update in &self.progress {
                progress.report(update.clone())?;
                cancellation.check()?;
            }
            let path = provider_output_path(request);
            match self.outcome {
                FakeOutcome::Success => {
                    fs::write(&path, b"%PDF-1.7\nfixture").unwrap();
                    Ok(ProviderConversionOutput { path })
                }
                FakeOutcome::InvalidOutput => {
                    fs::write(&path, b"invalid output").unwrap();
                    Ok(ProviderConversionOutput { path })
                }
                FakeOutcome::Failure => Err(provider_error(
                    FileConversionErrorCode::ProviderFailed,
                    "The fake provider failed.",
                    true,
                    Some(self.id),
                )),
                FakeOutcome::Timeout => Err(provider_error(
                    FileConversionErrorCode::Timeout,
                    "The fake provider timed out.",
                    true,
                    Some(self.id),
                )),
            }
        }
    }

    #[derive(Default)]
    struct RecordingProgressSink(Mutex<Vec<FileConversionProgress>>);

    impl FileConversionProgressSink for RecordingProgressSink {
        fn report(&self, progress: FileConversionProgress) -> Result<(), FileConversionError> {
            self.0.lock().unwrap().push(progress);
            Ok(())
        }
    }

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(1);
            let root = std::env::temp_dir().join(format!(
                "zero-file-provider-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos(),
                NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)
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
    fn selection_is_direction_and_platform_specific_and_deterministic() {
        let word = fake(
            FileConversionProviderId::MicrosoftWordMacos,
            true,
            FakeOutcome::Success,
            vec![],
        );
        let libreoffice = fake(
            FileConversionProviderId::LibreOffice,
            true,
            FakeOutcome::Success,
            vec![],
        );
        let registry = FileConversionProviderRegistry::new(vec![word, libreoffice]);

        assert_eq!(
            registry
                .select(FileConversionDirection::DocxToPdf, ProviderPlatform::Macos)
                .unwrap()
                .id(),
            FileConversionProviderId::LibreOffice
        );
        let unavailable = registry
            .select(FileConversionDirection::PdfToDocx, ProviderPlatform::Macos)
            .err()
            .expect("PDF to DOCX must remain unavailable");
        assert_eq!(unavailable.code, FileConversionErrorCode::EngineUnavailable);
    }

    #[test]
    fn selection_falls_back_to_available_word_for_the_current_platform_policy() {
        let unavailable_libreoffice = fake(
            FileConversionProviderId::LibreOffice,
            false,
            FakeOutcome::Success,
            vec![],
        );
        let word = fake(
            FileConversionProviderId::MicrosoftWordWindows,
            true,
            FakeOutcome::Success,
            vec![],
        );
        let registry = FileConversionProviderRegistry::new(vec![unavailable_libreoffice, word]);

        assert_eq!(
            registry
                .select(
                    FileConversionDirection::DocxToPdf,
                    ProviderPlatform::Windows
                )
                .unwrap()
                .id(),
            FileConversionProviderId::MicrosoftWordWindows
        );
    }

    #[test]
    fn fake_success_reports_real_percentage_and_indeterminate_progress() {
        let root = TestRoot::new();
        let request = request(&root.0);
        let sink = RecordingProgressSink::default();
        let provider = fake(
            FileConversionProviderId::LibreOffice,
            true,
            FakeOutcome::Success,
            vec![
                FileConversionProgress::Indeterminate {
                    stage: FileConversionStage::Converting,
                },
                FileConversionProgress::Percentage {
                    stage: FileConversionStage::Converting,
                    percent: 42,
                },
            ],
        );

        let output = provider
            .convert(&request, &sink, &FileConversionCancellationToken::default())
            .unwrap();

        validate_provider_output(&output.path, FileConversionDirection::DocxToPdf).unwrap();
        assert_eq!(sink.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn fake_failure_timeout_and_invalid_output_remain_distinct() {
        for (outcome, expected) in [
            (
                FakeOutcome::Failure,
                FileConversionErrorCode::ProviderFailed,
            ),
            (FakeOutcome::Timeout, FileConversionErrorCode::Timeout),
        ] {
            let root = TestRoot::new();
            let error = fake(FileConversionProviderId::LibreOffice, true, outcome, vec![])
                .convert(
                    &request(&root.0),
                    &RecordingProgressSink::default(),
                    &FileConversionCancellationToken::default(),
                )
                .unwrap_err();
            assert_eq!(error.code, expected);
        }

        let root = TestRoot::new();
        let output = fake(
            FileConversionProviderId::LibreOffice,
            true,
            FakeOutcome::InvalidOutput,
            vec![],
        )
        .convert(
            &request(&root.0),
            &RecordingProgressSink::default(),
            &FileConversionCancellationToken::default(),
        )
        .unwrap();
        assert_eq!(
            validate_provider_output(&output.path, FileConversionDirection::DocxToPdf)
                .unwrap_err()
                .code,
            FileConversionErrorCode::InvalidProviderOutput
        );
    }

    #[test]
    fn cancellation_is_observed_before_fake_provider_work() {
        let root = TestRoot::new();
        let cancellation = FileConversionCancellationToken::default();
        cancellation.cancel();

        let error = fake(
            FileConversionProviderId::LibreOffice,
            true,
            FakeOutcome::Success,
            vec![],
        )
        .convert(
            &request(&root.0),
            &RecordingProgressSink::default(),
            &cancellation,
        )
        .unwrap_err();

        assert_eq!(error.code, FileConversionErrorCode::Cancelled);
        assert!(!provider_output_path(&request(&root.0)).exists());
    }

    fn fake(
        id: FileConversionProviderId,
        available: bool,
        outcome: FakeOutcome,
        progress: Vec<FileConversionProgress>,
    ) -> Arc<dyn FileConversionProvider> {
        Arc::new(FakeProvider {
            id,
            directions: vec![FileConversionDirection::DocxToPdf],
            available,
            outcome,
            progress,
        })
    }

    fn request(root: &Path) -> ProviderConversionRequest {
        ProviderConversionRequest {
            job_id: "test-job".into(),
            direction: FileConversionDirection::DocxToPdf,
            source_path: root.join("source.docx"),
            temp_directory: root.to_path_buf(),
        }
    }
}
