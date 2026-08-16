#[cfg(unix)]
mod libreoffice_integration {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use zero_lib::services::file::contracts::{
        FileConversionDirection, FileConversionError, FileConversionProgress,
    };
    use zero_lib::services::file::discovery::{
        ExecutableResolver, ExecutableVersionProbe, LibreOfficeDiscovery, VersionProbeFailure,
    };
    use zero_lib::services::file::libreoffice::LibreOfficeProvider;
    use zero_lib::services::file::provider::{
        FileConversionCancellationToken, FileConversionProgressSink, FileConversionProvider,
        ProviderConversionRequest,
    };

    struct FixedResolver(PathBuf);

    impl ExecutableResolver for FixedResolver {
        fn resolve(&self) -> Option<PathBuf> {
            Some(self.0.clone())
        }
    }

    struct StableVersion;

    impl ExecutableVersionProbe for StableVersion {
        fn version_output(&self, _executable: &Path) -> Result<String, VersionProbeFailure> {
            Ok("LibreOffice 26.2.5.2 00(Build:2)".into())
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
            let path = std::env::temp_dir().join(format!(
                "zero-file-libreoffice-integration-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn fake_soffice_converts_a_spaced_non_ascii_path_through_real_process_execution() {
        use std::os::unix::fs::PermissionsExt;

        let root = TestRoot::new();
        let provider_directory = root.0.join("Fake LibreOffice 空格");
        let executable = provider_directory.join("soffice");
        fs::create_dir_all(&provider_directory).unwrap();
        fs::write(
            &executable,
            r#"#!/bin/sh
printf '%s\n' "$@" > "$PWD/arguments.txt"
outdir=''
source=''
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--outdir' ]; then
    shift
    outdir=$1
  elif [ "${1#-}" = "$1" ]; then
    source=$1
  fi
  shift
done
name=${source##*/}
stem=${name%.*}
printf '%%PDF-1.7\nfixture\n' > "$outdir/$stem.pdf"
"#,
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let source = root.0.join("报告 final.docx");
        fs::write(&source, b"fixture source").unwrap();
        let job_directory = root.0.join("job-1");
        fs::create_dir(&job_directory).unwrap();
        let discovery = Arc::new(LibreOfficeDiscovery::new(
            Box::new(FixedResolver(executable)),
            Box::new(StableVersion),
        ));
        let provider = LibreOfficeProvider::new(discovery);

        let output = provider
            .convert(
                &ProviderConversionRequest {
                    job_id: "job-1".into(),
                    direction: FileConversionDirection::DocxToPdf,
                    source_path: source.clone(),
                    temp_directory: job_directory.clone(),
                },
                &NoopProgress,
                &FileConversionCancellationToken::default(),
            )
            .unwrap();

        assert_eq!(output.path, job_directory.join("provider-output.pdf"));
        assert!(output.path.is_file());
        let arguments =
            fs::read_to_string(job_directory.join("libreoffice-work").join("arguments.txt"))
                .unwrap();
        assert!(arguments.contains("--headless"));
        assert!(arguments.contains("--convert-to\npdf"));
        assert!(arguments.contains(source.to_string_lossy().as_ref()));
        assert!(arguments.contains("-env:UserInstallation=file://"));
    }
}
