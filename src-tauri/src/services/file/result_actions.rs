use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::artifacts::validate_provider_output;
use super::contracts::{FileConversionError, FileConversionErrorCode, FileConversionJobState};
use super::queue::FileConversionJobRecord;
use super::runtime::FileConversionState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletedOutputAction {
    Open,
    Reveal,
}

impl FileConversionState {
    pub fn run_completed_output_action(
        &self,
        job_id: &str,
        action: CompletedOutputAction,
    ) -> Result<(), FileConversionError> {
        let record = self.completed_record(job_id)?;
        let output = validate_completed_record(&record)?;
        run_platform_output_action(&output, action)
    }
}

fn validate_completed_record(
    record: &FileConversionJobRecord,
) -> Result<PathBuf, FileConversionError> {
    let FileConversionJobState::Completed { result } = &record.snapshot.state else {
        return Err(result_error(
            FileConversionErrorCode::InvalidJobState,
            "Only a completed conversion result can be opened or revealed.",
            false,
        ));
    };
    let output = PathBuf::from(&result.output_path);
    if output != record.final_output
        || result.output_name != record.snapshot.target_name
        || output.file_name().and_then(|name| name.to_str()) != Some(result.output_name.as_str())
    {
        return Err(output_missing());
    }
    let metadata = std::fs::symlink_metadata(&output).map_err(|_| output_missing())?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(output_missing());
    }
    validate_provider_output(&output, record.snapshot.direction).map_err(|_| output_missing())?;
    Ok(output)
}

fn run_platform_output_action(
    output: &Path,
    action: CompletedOutputAction,
) -> Result<(), FileConversionError> {
    #[cfg(target_os = "macos")]
    let (executable, arguments): (&str, Vec<OsString>) = match action {
        CompletedOutputAction::Open => ("/usr/bin/open", vec![output.as_os_str().to_owned()]),
        CompletedOutputAction::Reveal => (
            "/usr/bin/open",
            vec![OsString::from("-R"), output.as_os_str().to_owned()],
        ),
    };
    #[cfg(target_os = "windows")]
    let (executable, arguments): (&str, Vec<OsString>) = match action {
        CompletedOutputAction::Open => ("explorer.exe", vec![output.as_os_str().to_owned()]),
        CompletedOutputAction::Reveal => {
            let mut selection = OsString::from("/select,");
            selection.push(output.as_os_str());
            ("explorer.exe", vec![selection])
        }
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (output, action);
        return Err(result_error(
            FileConversionErrorCode::UnsupportedInput,
            "Completed output actions are available only on macOS and Windows.",
            false,
        ));
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|_| {
            result_error(
                FileConversionErrorCode::ProviderFailed,
                "The operating system could not open the completed output.",
                true,
            )
        })
}

fn output_missing() -> FileConversionError {
    result_error(
        FileConversionErrorCode::OutputMissing,
        "The completed output was moved, deleted, or replaced.",
        false,
    )
}

fn result_error(
    code: FileConversionErrorCode,
    message: &str,
    retryable: bool,
) -> FileConversionError {
    FileConversionError {
        code,
        message: message.into(),
        retryable,
        provider_id: None,
        diagnostic: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::file::contracts::{
        FileConversionDirection, FileConversionJobSnapshot, FileConversionResult,
    };

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-result-action-{}-{}",
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
    fn revalidates_a_completed_job_record_without_accepting_a_frontend_path() {
        let root = TestRoot::new();
        let output = root.0.join("报告 converted.pdf");
        fs::write(&output, b"%PDF-1.7\nfixture").unwrap();
        let record = completed_record(output.clone());

        assert_eq!(validate_completed_record(&record).unwrap(), output);
    }

    #[test]
    fn rejects_non_completed_missing_replaced_and_mismatched_results() {
        let root = TestRoot::new();
        let output = root.0.join("converted.pdf");
        fs::write(&output, b"%PDF-1.7\nfixture").unwrap();
        let mut queued = completed_record(output.clone());
        queued.snapshot.state = FileConversionJobState::Queued;
        assert_eq!(
            validate_completed_record(&queued).unwrap_err().code,
            FileConversionErrorCode::InvalidJobState
        );

        let missing = completed_record(root.0.join("missing.pdf"));
        assert_eq!(
            validate_completed_record(&missing).unwrap_err().code,
            FileConversionErrorCode::OutputMissing
        );

        fs::write(&output, b"replaced with invalid bytes").unwrap();
        assert_eq!(
            validate_completed_record(&completed_record(output.clone()))
                .unwrap_err()
                .code,
            FileConversionErrorCode::OutputMissing
        );

        fs::write(&output, b"%PDF-1.7\nfixture").unwrap();
        let mut mismatched = completed_record(output);
        if let FileConversionJobState::Completed { result } = &mut mismatched.snapshot.state {
            result.output_path = root.0.join("other.pdf").to_string_lossy().into_owned();
        }
        assert_eq!(
            validate_completed_record(&mismatched).unwrap_err().code,
            FileConversionErrorCode::OutputMissing
        );
    }

    fn completed_record(output: PathBuf) -> FileConversionJobRecord {
        FileConversionJobRecord {
            canonical_source: PathBuf::from("/private/source.docx"),
            final_output: output.clone(),
            snapshot: FileConversionJobSnapshot {
                id: "job-1".into(),
                source_path: "/private/source.docx".into(),
                source_name: "source.docx".into(),
                size_bytes: 42,
                direction: FileConversionDirection::DocxToPdf,
                target_name: "报告 converted.pdf".into(),
                provider_id: None,
                created_at_ms: 1,
                updated_at_ms: 2,
                state: FileConversionJobState::Completed {
                    result: FileConversionResult {
                        output_path: output.to_string_lossy().into_owned(),
                        output_name: "报告 converted.pdf".into(),
                        size_bytes: 42,
                        completed_at_ms: 2,
                        provider_id: crate::services::file::contracts::FileConversionProviderId::LibreOffice,
                        provider_origin: crate::services::file::contracts::FileConversionProviderOrigin::Compatibility,
                        engine_version: Some("1.0.0".into()),
                        quality_profile: crate::services::file::contracts::FileConversionQualityProfile::CompatibilityProvider,
                        warning_keys: Vec::new(),
                        page_count: None,
                    },
                },
            },
        }
    }
}
