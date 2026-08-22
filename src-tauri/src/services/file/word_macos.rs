use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::artifacts::validate_provider_output;
use super::contracts::{
    FileConversionDirection, FileConversionError, FileConversionErrorCode, FileConversionProgress,
    FileConversionProvider as FileConversionProviderSnapshot, FileConversionProviderAvailability,
    FileConversionProviderId, FileConversionProviderOrigin, FileConversionQualityProfile,
    FileConversionStage,
};
use super::process::{DirectProcessRequest, DirectProcessRunner, SystemProcessRunner};
use super::provider::{
    provider_error, FileConversionCancellationToken, FileConversionProgressSink,
    FileConversionProvider, ProviderConversionOutput, ProviderConversionRequest,
};

const DIRECTIONS: [FileConversionDirection; 1] = [FileConversionDirection::DocxToPdf];
const WORD_CONVERSION_TIMEOUT: Duration = Duration::from_secs(180);
const PROCESS_OUTPUT_LIMIT: usize = 32 * 1024;
const OSASCRIPT_PATH: &str = "/usr/bin/osascript";
const WORD_JXA_SCRIPT: &str = r#"
function run(argv) {
  const inputPath = argv[0];
  const outputPath = argv[1];
  const word = Application("Microsoft Word");
  let document = null;
  try {
    document = word.open(inputPath);
    document.saveAs({ fileName: outputPath, fileFormat: "format PDF" });
  } finally {
    if (document !== null) {
      document.close({ saving: "no" });
    }
  }
}
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
struct WordMacosInstallation {
    application_path: PathBuf,
    version: String,
}

pub struct MicrosoftWordMacosProvider {
    installation: Option<WordMacosInstallation>,
    osascript_path: PathBuf,
    process_runner: Arc<dyn DirectProcessRunner>,
    timeout: Duration,
}

impl Default for MicrosoftWordMacosProvider {
    fn default() -> Self {
        Self {
            installation: detect_word_installation(),
            osascript_path: PathBuf::from(OSASCRIPT_PATH),
            process_runner: Arc::new(SystemProcessRunner),
            timeout: WORD_CONVERSION_TIMEOUT,
        }
    }
}

impl MicrosoftWordMacosProvider {
    #[cfg(test)]
    fn with_process_runner(
        installation: Option<WordMacosInstallation>,
        osascript_path: PathBuf,
        process_runner: Arc<dyn DirectProcessRunner>,
    ) -> Self {
        Self {
            installation,
            osascript_path,
            process_runner,
            timeout: Duration::from_secs(1),
        }
    }
}

impl FileConversionProvider for MicrosoftWordMacosProvider {
    fn id(&self) -> FileConversionProviderId {
        FileConversionProviderId::MicrosoftWordMacos
    }

    fn supported_directions(&self) -> &[FileConversionDirection] {
        &DIRECTIONS
    }

    fn probe(&self) -> FileConversionProviderSnapshot {
        let unavailable =
            |code, message, retryable| FileConversionProviderAvailability::Unavailable {
                error: provider_error(
                    code,
                    message,
                    retryable,
                    Some(FileConversionProviderId::MicrosoftWordMacos),
                ),
            };
        let (version, availability) = if !cfg!(target_os = "macos") {
            (
                None,
                unavailable(
                    FileConversionErrorCode::EngineUnavailable,
                    "Microsoft Word automation is available only on macOS.",
                    false,
                ),
            )
        } else if !self.osascript_path.is_file() {
            (
                None,
                unavailable(
                    FileConversionErrorCode::ProviderActivationFailed,
                    "The macOS Automation runtime is unavailable.",
                    false,
                ),
            )
        } else if let Some(installation) = &self.installation {
            if !installation.application_path.is_dir() {
                return FileConversionProviderSnapshot {
                    id: FileConversionProviderId::MicrosoftWordMacos,
                    display_name: "Microsoft Word".into(),
                    version: Some(installation.version.clone()),
                    origin: FileConversionProviderOrigin::Compatibility,
                    engine_version: None,
                    package_version: None,
                    platform_minimum: Some("macOS 11".into()),
                    quality_profiles: vec![FileConversionQualityProfile::CompatibilityProvider],
                    directions: DIRECTIONS.to_vec(),
                    availability: unavailable(
                        FileConversionErrorCode::EngineUnavailable,
                        "The detected Microsoft Word application is no longer installed.",
                        true,
                    ),
                };
            }
            let availability = if approved_word_macos_version(&installation.version) {
                FileConversionProviderAvailability::Available
            } else {
                unavailable(
                    FileConversionErrorCode::EngineVersionUnsupported,
                    "The installed Microsoft Word version is outside Zero's approved range.",
                    false,
                )
            };
            (Some(installation.version.clone()), availability)
        } else {
            (
                None,
                unavailable(
                    FileConversionErrorCode::EngineUnavailable,
                    "Microsoft Word was not found in an approved macOS application location.",
                    true,
                ),
            )
        };

        FileConversionProviderSnapshot {
            id: FileConversionProviderId::MicrosoftWordMacos,
            display_name: "Microsoft Word".into(),
            version,
            origin: FileConversionProviderOrigin::Compatibility,
            engine_version: None,
            package_version: None,
            platform_minimum: Some("macOS 11".into()),
            quality_profiles: vec![FileConversionQualityProfile::CompatibilityProvider],
            directions: DIRECTIONS.to_vec(),
            availability,
        }
    }

    fn convert(
        &self,
        request: &ProviderConversionRequest,
        progress: &dyn FileConversionProgressSink,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<ProviderConversionOutput, FileConversionError> {
        if request.direction != FileConversionDirection::DocxToPdf {
            return Err(word_error(
                FileConversionErrorCode::UnsupportedInput,
                "Microsoft Word is approved only for DOCX-to-PDF conversion.",
                false,
            ));
        }
        match self.probe().availability {
            FileConversionProviderAvailability::Available => {}
            FileConversionProviderAvailability::Unavailable { error } => return Err(error),
        }
        cancellation.check()?;
        let output = request.temp_directory.join("provider-output.pdf");
        progress.report(FileConversionProgress::Indeterminate {
            stage: FileConversionStage::Converting,
        })?;

        let process_result = self.process_runner.run(
            &DirectProcessRequest {
                executable: self.osascript_path.clone(),
                arguments: vec![
                    OsString::from("-l"),
                    OsString::from("JavaScript"),
                    OsString::from("-e"),
                    OsString::from(WORD_JXA_SCRIPT),
                    request.source_path.as_os_str().to_owned(),
                    output.as_os_str().to_owned(),
                ],
                working_directory: request.temp_directory.clone(),
                timeout: self.timeout,
                output_limit: PROCESS_OUTPUT_LIMIT,
                provider_id: FileConversionProviderId::MicrosoftWordMacos,
            },
            cancellation,
        )?;
        cancellation.check()?;
        if !process_result.success {
            return Err(classify_automation_failure(&process_result.stderr));
        }

        progress.report(FileConversionProgress::Indeterminate {
            stage: FileConversionStage::Finalizing,
        })?;
        validate_provider_output(&output, request.direction).map_err(|mut error| {
            error.provider_id = Some(FileConversionProviderId::MicrosoftWordMacos);
            error
        })?;
        Ok(ProviderConversionOutput {
            path: output,
            provider_origin: FileConversionProviderOrigin::Compatibility,
            engine_version: self.probe().version,
            quality_profile: FileConversionQualityProfile::CompatibilityProvider,
            warning_keys: vec!["file.quality.compatibilityProvider".into()],
            page_count: None,
        })
    }
}

fn detect_word_installation() -> Option<WordMacosInstallation> {
    #[cfg(target_os = "macos")]
    {
        let mut candidates = vec![PathBuf::from("/Applications/Microsoft Word.app")];
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(PathBuf::from(home).join("Applications/Microsoft Word.app"));
        }
        for application_path in candidates {
            let executable = application_path.join("Contents/MacOS/Microsoft Word");
            let info_plist = application_path.join("Contents/Info.plist");
            if !executable.is_file() || !info_plist.is_file() {
                continue;
            }
            let Ok(value) = plist::Value::from_file(&info_plist) else {
                continue;
            };
            let Some(version) = value
                .as_dictionary()
                .and_then(|dictionary| dictionary.get("CFBundleShortVersionString"))
                .and_then(plist::Value::as_string)
            else {
                continue;
            };
            return Some(WordMacosInstallation {
                application_path,
                version: version.to_string(),
            });
        }
    }
    None
}

fn approved_word_macos_version(version: &str) -> bool {
    let mut components = version.split('.').take(2).map(str::parse::<u32>);
    matches!(
        (components.next(), components.next()),
        (Some(Ok(16)), Some(Ok(minor))) if minor >= 80
    ) && !["alpha", "beta", "rc"]
        .iter()
        .any(|marker| version.to_ascii_lowercase().contains(marker))
}

fn classify_automation_failure(stderr: &str) -> FileConversionError {
    let normalized = stderr.to_ascii_lowercase();
    if normalized.contains("-1743") || normalized.contains("not authorized") {
        word_error(
            FileConversionErrorCode::AutomationPermissionDenied,
            "macOS denied Automation access to Microsoft Word.",
            true,
        )
    } else if normalized.contains("-600")
        || normalized.contains("application isn’t running")
        || normalized.contains("application isn't running")
        || normalized.contains("activation")
    {
        word_error(
            FileConversionErrorCode::ProviderActivationFailed,
            "Microsoft Word could not be activated for local conversion.",
            true,
        )
    } else {
        word_error(
            FileConversionErrorCode::ProviderFailed,
            "Microsoft Word could not convert this DOCX document.",
            true,
        )
    }
}

fn word_error(
    code: FileConversionErrorCode,
    message: &str,
    retryable: bool,
) -> FileConversionError {
    provider_error(
        code,
        message,
        retryable,
        Some(FileConversionProviderId::MicrosoftWordMacos),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::file::process::DirectProcessOutput;

    struct RecordingRunner {
        requests: Mutex<Vec<DirectProcessRequest>>,
        result: Mutex<DirectProcessOutput>,
    }

    impl DirectProcessRunner for RecordingRunner {
        fn run(
            &self,
            request: &DirectProcessRequest,
            _cancellation: &FileConversionCancellationToken,
        ) -> Result<DirectProcessOutput, FileConversionError> {
            self.requests.lock().unwrap().push(request.clone());
            if self.result.lock().unwrap().success {
                fs::write(request.arguments.last().unwrap(), b"%PDF-1.7\nfixture").unwrap();
            }
            Ok(self.result.lock().unwrap().clone())
        }
    }

    struct NoopProgress;

    impl FileConversionProgressSink for NoopProgress {
        fn report(&self, _progress: FileConversionProgress) -> Result<(), FileConversionError> {
            Ok(())
        }
    }

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-word-macos-{}-{}",
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
    fn approved_range_is_explicit_and_prereleases_are_rejected() {
        assert!(approved_word_macos_version("16.80"));
        assert!(approved_word_macos_version("16.112"));
        assert!(!approved_word_macos_version("16.79"));
        assert!(!approved_word_macos_version("17.0"));
        assert!(!approved_word_macos_version("16.112-beta"));
    }

    #[test]
    fn automation_failures_are_not_misreported_as_missing_engines() {
        assert_eq!(
            classify_automation_failure("execution error: Not authorized -1743").code,
            FileConversionErrorCode::AutomationPermissionDenied
        );
        assert_eq!(
            classify_automation_failure("Application isn't running (-600)").code,
            FileConversionErrorCode::ProviderActivationFailed
        );
        assert_eq!(
            classify_automation_failure("document export failed").code,
            FileConversionErrorCode::ProviderFailed
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn uses_jxa_argument_boundaries_and_closes_only_the_zero_opened_document() {
        let root = TestRoot::new();
        let osascript = root.0.join("osascript");
        fs::write(&osascript, b"fixture").unwrap();
        let application_path = root.0.join("Microsoft Word.app");
        fs::create_dir(&application_path).unwrap();
        let source = root.0.join("报告 final.docx");
        fs::write(&source, b"fixture").unwrap();
        let runner = Arc::new(RecordingRunner {
            requests: Mutex::new(Vec::new()),
            result: Mutex::new(DirectProcessOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }),
        });
        let provider = MicrosoftWordMacosProvider::with_process_runner(
            Some(WordMacosInstallation {
                application_path,
                version: "16.112".into(),
            }),
            osascript,
            runner.clone(),
        );

        let output = provider
            .convert(
                &ProviderConversionRequest {
                    job_id: "word-job".into(),
                    direction: FileConversionDirection::DocxToPdf,
                    source_path: source.clone(),
                    temp_directory: root.0.clone(),
                },
                &NoopProgress,
                &FileConversionCancellationToken::default(),
            )
            .unwrap();

        assert!(output.path.is_file());
        let requests = runner.requests.lock().unwrap();
        assert_eq!(requests[0].arguments[0], "-l");
        assert_eq!(requests[0].arguments[1], "JavaScript");
        assert_eq!(requests[0].arguments[4], source.as_os_str());
        let script = requests[0].arguments[3].to_string_lossy();
        assert!(script.contains("document.close"));
        assert!(!script.contains("word.quit"));
    }
}
