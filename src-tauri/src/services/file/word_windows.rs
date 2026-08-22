use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
const WORD_COM_SCRIPT: &str = r#"
param(
  [Parameter(Mandatory = $true)][string]$InputPath,
  [Parameter(Mandatory = $true)][string]$OutputPath
)
$ErrorActionPreference = 'Stop'
$word = $null
$document = $null
$exitCode = 0
try {
  $word = New-Object -ComObject Word.Application
  $word.Visible = $false
  $word.DisplayAlerts = 0
  $document = $word.Documents.Open($InputPath, $false, $true)
  $document.ExportAsFixedFormat($OutputPath, 17)
} catch [System.UnauthorizedAccessException] {
  [Console]::Error.WriteLine('ZERO_WORD_PERMISSION')
  $exitCode = 13
} catch [System.Runtime.InteropServices.COMException] {
  $hresult = ('0x{0:X8}' -f ($_.Exception.HResult -band 0xffffffffL))
  if ($hresult -eq '0x80040154') {
    [Console]::Error.WriteLine('ZERO_WORD_UNAVAILABLE')
    $exitCode = 10
  } elseif ($hresult -eq '0x80070005') {
    [Console]::Error.WriteLine('ZERO_WORD_PERMISSION')
    $exitCode = 13
  } elseif ($hresult -eq '0x80080005') {
    [Console]::Error.WriteLine('ZERO_WORD_ACTIVATION')
    $exitCode = 11
  } else {
    [Console]::Error.WriteLine('ZERO_WORD_COM_FAILURE')
    $exitCode = 12
  }
} catch {
  [Console]::Error.WriteLine('ZERO_WORD_PROVIDER_FAILURE')
  $exitCode = 12
} finally {
  if ($null -ne $document) {
    try { $document.Close(0) } catch {}
    [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($document)
  }
  if ($null -ne $word) {
    try { $word.Quit(0) } catch {}
    [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($word)
  }
  [GC]::Collect()
  [GC]::WaitForPendingFinalizers()
}
exit $exitCode
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
struct WordWindowsInstallation {
    executable_path: PathBuf,
    version: String,
}

pub struct MicrosoftWordWindowsProvider {
    installation: Mutex<Option<WordWindowsInstallation>>,
    powershell_path: Mutex<Option<PathBuf>>,
    process_runner: Arc<dyn DirectProcessRunner>,
    timeout: Duration,
    platform_supported: bool,
}

impl Default for MicrosoftWordWindowsProvider {
    fn default() -> Self {
        Self {
            installation: Mutex::new(detect_word_installation()),
            powershell_path: Mutex::new(detect_powershell()),
            process_runner: Arc::new(SystemProcessRunner),
            timeout: WORD_CONVERSION_TIMEOUT,
            platform_supported: cfg!(target_os = "windows"),
        }
    }
}

impl MicrosoftWordWindowsProvider {
    #[cfg(test)]
    fn with_process_runner(
        installation: Option<WordWindowsInstallation>,
        powershell_path: Option<PathBuf>,
        process_runner: Arc<dyn DirectProcessRunner>,
    ) -> Self {
        Self {
            installation: Mutex::new(installation),
            powershell_path: Mutex::new(powershell_path),
            process_runner,
            timeout: Duration::from_secs(1),
            platform_supported: true,
        }
    }
}

impl FileConversionProvider for MicrosoftWordWindowsProvider {
    fn id(&self) -> FileConversionProviderId {
        FileConversionProviderId::MicrosoftWordWindows
    }

    fn supported_directions(&self) -> &[FileConversionDirection] {
        &DIRECTIONS
    }

    fn probe(&self) -> FileConversionProviderSnapshot {
        let installation = self
            .installation
            .lock()
            .ok()
            .and_then(|installation| installation.clone());
        let powershell_path = self
            .powershell_path
            .lock()
            .ok()
            .and_then(|path| path.clone());
        let unavailable =
            |code, message, retryable| FileConversionProviderAvailability::Unavailable {
                error: word_error(code, message, retryable),
            };
        let (version, availability) = if !self.platform_supported {
            (
                None,
                unavailable(
                    FileConversionErrorCode::EngineUnavailable,
                    "Microsoft Word COM automation is available only on Windows.",
                    false,
                ),
            )
        } else if powershell_path.as_ref().is_none_or(|path| !path.is_file()) {
            (
                None,
                unavailable(
                    FileConversionErrorCode::ProviderActivationFailed,
                    "Windows PowerShell with STA support is unavailable.",
                    false,
                ),
            )
        } else if let Some(installation) = &installation {
            let availability = if !installation.executable_path.is_file() {
                unavailable(
                    FileConversionErrorCode::EngineUnavailable,
                    "The detected Microsoft Word executable is no longer installed.",
                    true,
                )
            } else if approved_word_windows_version(&installation.version) {
                FileConversionProviderAvailability::Available
            } else {
                unavailable(
                    FileConversionErrorCode::EngineVersionUnsupported,
                    "The installed Microsoft Word version is outside Zero's approved Office 16 range.",
                    false,
                )
            };
            (Some(installation.version.clone()), availability)
        } else {
            (
                None,
                unavailable(
                    FileConversionErrorCode::EngineUnavailable,
                    "Microsoft Word was not found in an approved Windows installation location.",
                    true,
                ),
            )
        };
        FileConversionProviderSnapshot {
            id: FileConversionProviderId::MicrosoftWordWindows,
            display_name: "Microsoft Word".into(),
            version,
            origin: FileConversionProviderOrigin::Compatibility,
            engine_version: None,
            package_version: None,
            platform_minimum: Some("Windows 10".into()),
            quality_profiles: vec![FileConversionQualityProfile::CompatibilityProvider],
            directions: DIRECTIONS.to_vec(),
            availability,
        }
    }

    fn invalidate(&self) {
        if let Ok(mut installation) = self.installation.lock() {
            *installation = detect_word_installation();
        }
        if let Ok(mut powershell_path) = self.powershell_path.lock() {
            *powershell_path = detect_powershell();
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
        let powershell_path = self
            .powershell_path
            .lock()
            .ok()
            .and_then(|path| path.clone())
            .ok_or_else(|| {
                word_error(
                    FileConversionErrorCode::ProviderActivationFailed,
                    "Windows PowerShell is no longer available.",
                    true,
                )
            })?;
        let script_path = request.temp_directory.join("zero-word-convert.ps1");
        write_private_script(&script_path)?;
        let output = request.temp_directory.join("provider-output.pdf");

        progress.report(FileConversionProgress::Indeterminate {
            stage: FileConversionStage::Converting,
        })?;
        let process_result = self.process_runner.run(
            &DirectProcessRequest {
                executable: powershell_path,
                arguments: vec![
                    OsString::from("-NoLogo"),
                    OsString::from("-NoProfile"),
                    OsString::from("-NonInteractive"),
                    OsString::from("-STA"),
                    OsString::from("-ExecutionPolicy"),
                    OsString::from("Bypass"),
                    OsString::from("-File"),
                    script_path.into_os_string(),
                    OsString::from("-InputPath"),
                    request.source_path.as_os_str().to_owned(),
                    OsString::from("-OutputPath"),
                    output.as_os_str().to_owned(),
                ],
                working_directory: request.temp_directory.clone(),
                timeout: self.timeout,
                output_limit: PROCESS_OUTPUT_LIMIT,
                provider_id: FileConversionProviderId::MicrosoftWordWindows,
            },
            cancellation,
        )?;
        cancellation.check()?;
        if !process_result.success {
            return Err(classify_com_failure(&process_result.stderr));
        }

        progress.report(FileConversionProgress::Indeterminate {
            stage: FileConversionStage::Finalizing,
        })?;
        validate_provider_output(&output, request.direction).map_err(|mut error| {
            error.provider_id = Some(FileConversionProviderId::MicrosoftWordWindows);
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

fn detect_word_installation() -> Option<WordWindowsInstallation> {
    #[cfg(target_os = "windows")]
    {
        for root in [
            std::env::var_os("ProgramFiles"),
            std::env::var_os("ProgramFiles(x86)"),
        ]
        .into_iter()
        .flatten()
        {
            let executable_path = PathBuf::from(root)
                .join("Microsoft Office")
                .join("root")
                .join("Office16")
                .join("WINWORD.EXE");
            if executable_path.is_file() {
                return Some(WordWindowsInstallation {
                    executable_path,
                    version: "16".into(),
                });
            }
        }
    }
    None
}

fn detect_powershell() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let system_root = std::env::var_os("SystemRoot")?;
        let candidate = PathBuf::from(system_root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        return candidate.is_file().then_some(candidate);
    }
    #[cfg(not(target_os = "windows"))]
    None
}

fn approved_word_windows_version(version: &str) -> bool {
    version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        == Some(16)
        && !["alpha", "beta", "preview"]
            .iter()
            .any(|marker| version.to_ascii_lowercase().contains(marker))
}

fn write_private_script(path: &Path) -> Result<(), FileConversionError> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| {
        word_error(
            FileConversionErrorCode::PermissionDenied,
            "The private Word automation script could not be created.",
            true,
        )
    })?;
    file.write_all(WORD_COM_SCRIPT.as_bytes()).map_err(|_| {
        word_error(
            FileConversionErrorCode::PermissionDenied,
            "The private Word automation script could not be written.",
            true,
        )
    })?;
    file.sync_all().map_err(|_| {
        word_error(
            FileConversionErrorCode::PermissionDenied,
            "The private Word automation script could not be synchronized.",
            true,
        )
    })
}

fn classify_com_failure(stderr: &str) -> FileConversionError {
    if stderr.contains("ZERO_WORD_UNAVAILABLE") {
        word_error(
            FileConversionErrorCode::EngineUnavailable,
            "Microsoft Word COM automation is not registered.",
            true,
        )
    } else if stderr.contains("ZERO_WORD_PERMISSION") {
        word_error(
            FileConversionErrorCode::PermissionDenied,
            "Windows denied access to Microsoft Word automation.",
            true,
        )
    } else if stderr.contains("ZERO_WORD_ACTIVATION") {
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
        Some(FileConversionProviderId::MicrosoftWordWindows),
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
    }

    impl DirectProcessRunner for RecordingRunner {
        fn run(
            &self,
            request: &DirectProcessRequest,
            _cancellation: &FileConversionCancellationToken,
        ) -> Result<DirectProcessOutput, FileConversionError> {
            self.requests.lock().unwrap().push(request.clone());
            fs::write(request.arguments.last().unwrap(), b"%PDF-1.7\nfixture").unwrap();
            Ok(DirectProcessOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            })
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
                "zero-file-word-windows-{}-{}",
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
    fn approved_range_requires_office_major_version_16() {
        assert!(approved_word_windows_version("16"));
        assert!(approved_word_windows_version("16.0.20228"));
        assert!(!approved_word_windows_version("15.0"));
        assert!(!approved_word_windows_version("17.0-preview"));
    }

    #[test]
    fn com_failures_are_classified_without_exposing_diagnostics() {
        assert_eq!(
            classify_com_failure("ZERO_WORD_UNAVAILABLE").code,
            FileConversionErrorCode::EngineUnavailable
        );
        assert_eq!(
            classify_com_failure("ZERO_WORD_PERMISSION").code,
            FileConversionErrorCode::PermissionDenied
        );
        assert_eq!(
            classify_com_failure("ZERO_WORD_ACTIVATION").code,
            FileConversionErrorCode::ProviderActivationFailed
        );
        assert!(classify_com_failure("unknown COM details")
            .diagnostic
            .is_none());
    }

    #[test]
    fn powershell_sta_adapter_uses_argument_boundaries_and_explicit_com_cleanup() {
        let root = TestRoot::new();
        let word = root.0.join("WINWORD.EXE");
        let powershell = root.0.join("powershell.exe");
        fs::write(&word, b"fixture").unwrap();
        fs::write(&powershell, b"fixture").unwrap();
        let source = root.0.join("报告 final.docx");
        fs::write(&source, b"fixture").unwrap();
        let runner = Arc::new(RecordingRunner {
            requests: Mutex::new(Vec::new()),
        });
        let provider = MicrosoftWordWindowsProvider::with_process_runner(
            Some(WordWindowsInstallation {
                executable_path: word,
                version: "16.0.20228".into(),
            }),
            Some(powershell),
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
        let arguments = &requests[0].arguments;
        assert!(arguments.contains(&OsString::from("-STA")));
        let input_index = arguments
            .iter()
            .position(|argument| argument == "-InputPath")
            .unwrap();
        assert_eq!(arguments[input_index + 1], source.as_os_str());
        let script = fs::read_to_string(root.0.join("zero-word-convert.ps1")).unwrap();
        assert!(script.contains("$document.Close(0)"));
        assert!(script.contains("$word.Quit(0)"));
        assert!(script.contains("FinalReleaseComObject"));
        assert!(script.contains("finally"));
    }
}
