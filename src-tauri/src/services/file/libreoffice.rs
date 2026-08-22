use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::artifacts::validate_provider_output;
use super::contracts::{
    FileConversionDirection, FileConversionError, FileConversionErrorCode, FileConversionProgress,
    FileConversionProvider as FileConversionProviderSnapshot, FileConversionProviderAvailability,
    FileConversionProviderId, FileConversionProviderOrigin, FileConversionQualityProfile,
    FileConversionStage,
};
use super::discovery::LibreOfficeDiscovery;
use super::errors::{classify_provider_failure, ProviderFailureKind};
use super::process::{DirectProcessRequest, DirectProcessRunner, SystemProcessRunner};
use super::provider::{
    provider_error, FileConversionCancellationToken, FileConversionProgressSink,
    FileConversionProvider, ProviderConversionOutput, ProviderConversionRequest,
};

const DIRECTIONS: [FileConversionDirection; 1] = [FileConversionDirection::DocxToPdf];
const DEFAULT_CONVERSION_TIMEOUT: Duration = Duration::from_secs(180);
const PROCESS_OUTPUT_LIMIT: usize = 64 * 1024;

pub struct LibreOfficeProvider {
    discovery: Arc<LibreOfficeDiscovery>,
    process_runner: Arc<dyn DirectProcessRunner>,
    timeout: Duration,
}

impl LibreOfficeProvider {
    pub fn new(discovery: Arc<LibreOfficeDiscovery>) -> Self {
        Self {
            discovery,
            process_runner: Arc::new(SystemProcessRunner),
            timeout: DEFAULT_CONVERSION_TIMEOUT,
        }
    }

    pub fn with_process_runner(
        discovery: Arc<LibreOfficeDiscovery>,
        process_runner: Arc<dyn DirectProcessRunner>,
        timeout: Duration,
    ) -> Self {
        Self {
            discovery,
            process_runner,
            timeout,
        }
    }

    fn executable(&self) -> Result<PathBuf, FileConversionError> {
        let snapshot = self.discovery.probe();
        if !matches!(
            snapshot.availability,
            FileConversionProviderAvailability::Available
        ) {
            if let FileConversionProviderAvailability::Unavailable { error } = snapshot.availability
            {
                return Err(error);
            }
        }
        self.discovery.resolved_executable().ok_or_else(|| {
            provider_error(
                FileConversionErrorCode::EngineUnavailable,
                "LibreOffice is no longer available at its detected location.",
                true,
                Some(FileConversionProviderId::LibreOffice),
            )
        })
    }
}

impl FileConversionProvider for LibreOfficeProvider {
    fn id(&self) -> FileConversionProviderId {
        FileConversionProviderId::LibreOffice
    }

    fn supported_directions(&self) -> &[FileConversionDirection] {
        &DIRECTIONS
    }

    fn probe(&self) -> FileConversionProviderSnapshot {
        self.discovery.probe()
    }

    fn invalidate(&self) {
        self.discovery.invalidate();
    }

    fn convert(
        &self,
        request: &ProviderConversionRequest,
        progress: &dyn FileConversionProgressSink,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<ProviderConversionOutput, FileConversionError> {
        if request.direction != FileConversionDirection::DocxToPdf {
            return Err(provider_error(
                FileConversionErrorCode::UnsupportedInput,
                "LibreOffice is approved only for DOCX-to-PDF conversion.",
                false,
                Some(FileConversionProviderId::LibreOffice),
            ));
        }
        cancellation.check()?;
        let executable = self.executable()?;
        let profile =
            create_private_directory(&request.temp_directory.join("libreoffice-profile"))?;
        let working_directory =
            create_private_directory(&request.temp_directory.join("libreoffice-work"))?;
        let output_directory =
            create_private_directory(&request.temp_directory.join("libreoffice-output"))?;
        let source_stem = request
            .source_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                provider_error(
                    FileConversionErrorCode::InvalidInput,
                    "The DOCX source name cannot be converted safely.",
                    false,
                    Some(FileConversionProviderId::LibreOffice),
                )
            })?;
        let raw_provider_output = output_directory.join(format!("{source_stem}.pdf"));
        let normalized_output = request.temp_directory.join("provider-output.pdf");
        let profile_argument = format!("-env:UserInstallation={}", path_to_file_url(&profile)?);

        progress.report(FileConversionProgress::Indeterminate {
            stage: FileConversionStage::Converting,
        })?;
        let process_output = self.process_runner.run(
            &DirectProcessRequest {
                executable,
                arguments: vec![
                    OsString::from("--headless"),
                    OsString::from("--nologo"),
                    OsString::from("--nodefault"),
                    OsString::from("--nofirststartwizard"),
                    OsString::from(profile_argument),
                    OsString::from("--convert-to"),
                    OsString::from("pdf"),
                    OsString::from("--outdir"),
                    output_directory.as_os_str().to_owned(),
                    request.source_path.as_os_str().to_owned(),
                ],
                working_directory,
                timeout: self.timeout,
                output_limit: PROCESS_OUTPUT_LIMIT,
                provider_id: FileConversionProviderId::LibreOffice,
            },
            cancellation,
        )?;
        cancellation.check()?;
        if !process_output.success {
            return Err(classify_provider_failure(
                ProviderFailureKind::ProviderExit {
                    exit_code: process_output.exit_code,
                },
                Some(FileConversionProviderId::LibreOffice),
            ));
        }

        progress.report(FileConversionProgress::Indeterminate {
            stage: FileConversionStage::Finalizing,
        })?;
        validate_provider_output(&raw_provider_output, request.direction)?;
        fs::rename(&raw_provider_output, &normalized_output).map_err(|_| {
            provider_error(
                FileConversionErrorCode::InvalidProviderOutput,
                "LibreOffice output could not be normalized in the private workspace.",
                true,
                Some(FileConversionProviderId::LibreOffice),
            )
        })?;
        validate_provider_output(&normalized_output, request.direction)?;
        Ok(ProviderConversionOutput {
            path: normalized_output,
            provider_origin: FileConversionProviderOrigin::Compatibility,
            engine_version: self.probe().version,
            quality_profile: FileConversionQualityProfile::CompatibilityProvider,
            warning_keys: vec!["file.quality.compatibilityProvider".into()],
            page_count: None,
        })
    }
}

fn create_private_directory(path: &Path) -> Result<PathBuf, FileConversionError> {
    fs::create_dir(path).map_err(|_| {
        provider_error(
            FileConversionErrorCode::ProviderFailed,
            "LibreOffice's private working directory could not be created.",
            true,
            Some(FileConversionProviderId::LibreOffice),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| {
            provider_error(
                FileConversionErrorCode::PermissionDenied,
                "LibreOffice's private working directory could not be restricted.",
                true,
                Some(FileConversionProviderId::LibreOffice),
            )
        })?;
    }
    Ok(path.to_path_buf())
}

fn path_to_file_url(path: &Path) -> Result<String, FileConversionError> {
    let canonical = path.canonicalize().map_err(|_| {
        provider_error(
            FileConversionErrorCode::ProviderFailed,
            "LibreOffice's private profile path could not be resolved.",
            true,
            Some(FileConversionProviderId::LibreOffice),
        )
    })?;
    let normalized = canonical.to_string_lossy().replace('\\', "/");
    let mut encoded = String::with_capacity(normalized.len());
    for byte in normalized.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/' | b':') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    if encoded.starts_with('/') {
        Ok(format!("file://{encoded}"))
    } else {
        Ok(format!("file:///{encoded}"))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::file::discovery::{
        ExecutableResolver, ExecutableVersionProbe, VersionProbeFailure,
    };

    struct FixedResolver(PathBuf);

    impl ExecutableResolver for FixedResolver {
        fn resolve(&self) -> Option<PathBuf> {
            Some(self.0.clone())
        }
    }

    struct FixedVersionProbe;

    impl ExecutableVersionProbe for FixedVersionProbe {
        fn version_output(&self, _executable: &Path) -> Result<String, VersionProbeFailure> {
            Ok("LibreOffice 26.2.5.2 00(Build:2)".into())
        }
    }

    #[derive(Default)]
    struct RecordingRunner(Mutex<Vec<DirectProcessRequest>>);

    impl DirectProcessRunner for RecordingRunner {
        fn run(
            &self,
            request: &DirectProcessRequest,
            _cancellation: &FileConversionCancellationToken,
        ) -> Result<super::super::process::DirectProcessOutput, FileConversionError> {
            self.0.lock().unwrap().push(request.clone());
            let outdir_index = request
                .arguments
                .iter()
                .position(|argument| argument == "--outdir")
                .unwrap();
            let outdir = PathBuf::from(&request.arguments[outdir_index + 1]);
            let source = PathBuf::from(request.arguments.last().unwrap());
            fs::write(
                outdir.join(format!(
                    "{}.pdf",
                    source.file_stem().unwrap().to_string_lossy()
                )),
                b"%PDF-1.7\nfixture",
            )
            .unwrap();
            Ok(super::super::process::DirectProcessOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    #[derive(Default)]
    struct RecordingProgress(Mutex<Vec<FileConversionProgress>>);

    impl FileConversionProgressSink for RecordingProgress {
        fn report(&self, progress: FileConversionProgress) -> Result<(), FileConversionError> {
            self.0.lock().unwrap().push(progress);
            Ok(())
        }
    }

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-libreoffice-{}-{}",
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
    fn builds_headless_argument_array_with_private_profile_and_validates_output() {
        let root = TestRoot::new();
        let executable = root.0.join("LibreOffice 空格").join("soffice");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"fixture").unwrap();
        let temp_directory = root.0.join("job");
        fs::create_dir(&temp_directory).unwrap();
        let source_path = root.0.join("报告 final.docx");
        fs::write(&source_path, b"fixture").unwrap();
        let discovery = Arc::new(LibreOfficeDiscovery::new(
            Box::new(FixedResolver(executable.clone())),
            Box::new(FixedVersionProbe),
        ));
        let runner = Arc::new(RecordingRunner::default());
        let provider = LibreOfficeProvider::with_process_runner(
            discovery,
            runner.clone(),
            Duration::from_secs(1),
        );
        let progress = RecordingProgress::default();

        let output = provider
            .convert(
                &ProviderConversionRequest {
                    job_id: "job-1".into(),
                    direction: FileConversionDirection::DocxToPdf,
                    source_path: source_path.clone(),
                    temp_directory: temp_directory.clone(),
                },
                &progress,
                &FileConversionCancellationToken::default(),
            )
            .unwrap();

        assert_eq!(output.path, temp_directory.join("provider-output.pdf"));
        assert!(output.path.is_file());
        let requests = runner.0.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].executable, executable);
        assert_eq!(
            requests[0].arguments.last(),
            Some(&source_path.into_os_string())
        );
        assert!(requests[0].arguments.iter().any(|argument| {
            argument
                .to_string_lossy()
                .starts_with("-env:UserInstallation=file://")
        }));
        assert_eq!(progress.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn file_url_percent_encodes_spaces_and_non_ascii_bytes() {
        let root = TestRoot::new();
        let path = root.0.join("profile 空格");
        fs::create_dir(&path).unwrap();

        let url = path_to_file_url(&path).unwrap();

        assert!(url.starts_with("file:///"));
        assert!(url.contains("%20"));
        assert!(url.contains("%E7%A9%BA%E6%A0%BC"));
    }
}
