#[cfg(unix)]
mod process_integration {
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use zero_lib::services::file::artifacts::validate_provider_output;
    use zero_lib::services::file::contracts::{
        FileConversionDirection, FileConversionErrorCode, FileConversionProviderId,
    };
    use zero_lib::services::file::process::{
        DirectProcessRequest, DirectProcessRunner, SystemProcessRunner,
    };
    use zero_lib::services::file::provider::FileConversionCancellationToken;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-process-integration-{}-{}",
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

    fn executable(root: &TestRoot, name: &str, source: &str) -> PathBuf {
        let path = root.0.join(name);
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn request(
        executable: PathBuf,
        working_directory: PathBuf,
        arguments: Vec<OsString>,
    ) -> DirectProcessRequest {
        DirectProcessRequest {
            executable,
            arguments,
            working_directory,
            timeout: Duration::from_secs(2),
            output_limit: 8,
            provider_id: FileConversionProviderId::LibreOffice,
        }
    }

    #[test]
    fn fake_executable_preserves_arguments_working_directory_and_output_bounds() {
        let root = TestRoot::new();
        let working_directory = root.0.join("job workspace 空格");
        fs::create_dir(&working_directory).unwrap();
        let provider = executable(
            &root,
            "fake provider.sh",
            "#!/bin/sh\nprintf '%s' \"$PWD\"\nprintf '%s' \"$1\" >&2\n",
        );
        let input = OsString::from("报告 final.docx;$(touch should-not-run)");

        let output = SystemProcessRunner
            .run(
                &request(provider, working_directory.clone(), vec![input]),
                &FileConversionCancellationToken::default(),
            )
            .unwrap();

        assert!(output.success);
        let canonical_working_directory = fs::canonicalize(&working_directory).unwrap();
        assert_eq!(
            output.stdout,
            canonical_working_directory.to_string_lossy()[..8]
        );
        assert_eq!(output.stderr.as_bytes().len(), 8);
        assert!(!working_directory.join("should-not-run").exists());
    }

    #[test]
    fn fake_executable_is_cancelled_and_invalid_results_are_rejected() {
        let root = TestRoot::new();
        let provider = executable(
            &root,
            "slow-provider.sh",
            "#!/bin/sh\nwhile :; do :; done\n",
        );
        let mut timeout_request = request(provider.clone(), root.0.clone(), Vec::new());
        timeout_request.timeout = Duration::from_millis(40);
        assert_eq!(
            SystemProcessRunner
                .run(
                    &timeout_request,
                    &FileConversionCancellationToken::default(),
                )
                .unwrap_err()
                .code,
            FileConversionErrorCode::Timeout
        );

        let cancellation = FileConversionCancellationToken::default();
        let cancellation_for_thread = cancellation.clone();
        let cancel_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            cancellation_for_thread.cancel();
        });

        let error = SystemProcessRunner
            .run(
                &request(provider, root.0.clone(), Vec::new()),
                &cancellation,
            )
            .unwrap_err();
        cancel_thread.join().unwrap();
        assert_eq!(error.code, FileConversionErrorCode::Cancelled);

        let invalid_output = root.0.join("provider-output.pdf");
        fs::write(&invalid_output, b"not a PDF").unwrap();
        assert_eq!(
            validate_provider_output(&invalid_output, FileConversionDirection::DocxToPdf)
                .unwrap_err()
                .code,
            FileConversionErrorCode::InvalidProviderOutput
        );
    }
}
