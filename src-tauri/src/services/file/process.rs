use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::contracts::{FileConversionError, FileConversionErrorCode, FileConversionProviderId};
use super::provider::{provider_error, FileConversionCancellationToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectProcessRequest {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub working_directory: PathBuf,
    pub timeout: Duration,
    pub output_limit: usize,
    pub provider_id: FileConversionProviderId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectProcessOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait DirectProcessRunner: Send + Sync {
    fn run(
        &self,
        request: &DirectProcessRequest,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<DirectProcessOutput, FileConversionError>;
}

#[derive(Debug, Default)]
pub struct SystemProcessRunner;

impl DirectProcessRunner for SystemProcessRunner {
    fn run(
        &self,
        request: &DirectProcessRequest,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<DirectProcessOutput, FileConversionError> {
        cancellation.check()?;
        let mut command = Command::new(&request.executable);
        command
            .args(&request.arguments)
            .current_dir(&request.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn().map_err(|error| {
            let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
                FileConversionErrorCode::PermissionDenied
            } else {
                FileConversionErrorCode::ProviderFailed
            };
            provider_error(
                code,
                "The local provider process could not be started.",
                true,
                Some(request.provider_id),
            )
        })?;

        let stdout_reader = child
            .stdout
            .take()
            .map(|stdout| spawn_bounded_reader(stdout, request.output_limit));
        let stderr_reader = child
            .stderr
            .take()
            .map(|stderr| spawn_bounded_reader(stderr, request.output_limit));
        let deadline = Instant::now() + request.timeout;

        let status = loop {
            if cancellation.is_cancelled() {
                terminate_process_tree(&mut child);
                join_reader(stdout_reader);
                join_reader(stderr_reader);
                return Err(provider_error(
                    FileConversionErrorCode::Cancelled,
                    "The conversion was cancelled.",
                    true,
                    Some(request.provider_id),
                ));
            }
            if Instant::now() >= deadline {
                terminate_process_tree(&mut child);
                join_reader(stdout_reader);
                join_reader(stderr_reader);
                return Err(provider_error(
                    FileConversionErrorCode::Timeout,
                    "The local provider exceeded the conversion deadline.",
                    true,
                    Some(request.provider_id),
                ));
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(_) => {
                    terminate_process_tree(&mut child);
                    join_reader(stdout_reader);
                    join_reader(stderr_reader);
                    return Err(provider_error(
                        FileConversionErrorCode::ProviderFailed,
                        "The local provider process could not be monitored.",
                        true,
                        Some(request.provider_id),
                    ));
                }
            }
        };

        Ok(DirectProcessOutput {
            success: status.success(),
            exit_code: status.code(),
            stdout: join_reader(stdout_reader),
            stderr: join_reader(stderr_reader),
        })
    }
}

#[cfg(unix)]
fn terminate_process_tree(child: &mut std::process::Child) {
    let process_group = -(child.id() as i32);
    unsafe {
        libc::kill(process_group, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    unsafe {
        libc::kill(process_group, libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(target_os = "windows")]
fn terminate_process_tree(child: &mut std::process::Child) {
    let _ = Command::new("taskkill.exe")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(any(unix, target_os = "windows")))]
fn terminate_process_tree(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn spawn_bounded_reader(
    mut reader: impl Read + Send + 'static,
    output_limit: usize,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut captured = Vec::with_capacity(output_limit.min(8 * 1024));
        let mut buffer = [0_u8; 8 * 1024];
        while let Ok(read) = reader.read(&mut buffer) {
            if read == 0 {
                break;
            }
            let remaining = output_limit.saturating_sub(captured.len());
            captured.extend_from_slice(&buffer[..read.min(remaining)]);
        }
        String::from_utf8_lossy(&captured).into_owned()
    })
}

fn join_reader(reader: Option<thread::JoinHandle<String>>) -> String {
    reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-process-{}-{}",
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

    #[cfg(unix)]
    #[test]
    fn direct_runner_preserves_argument_boundaries_and_bounds_output() {
        use std::os::unix::fs::PermissionsExt;

        let root = TestRoot::new();
        let executable = root.0.join("fake provider.sh");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' \"$1\"\nprintf '0123456789' >&2\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let request = DirectProcessRequest {
            executable,
            arguments: vec![OsString::from("value with 空格;$(ignored)")],
            working_directory: root.0.clone(),
            timeout: Duration::from_secs(1),
            output_limit: 6,
            provider_id: FileConversionProviderId::LibreOffice,
        };

        let output = SystemProcessRunner
            .run(&request, &FileConversionCancellationToken::default())
            .unwrap();

        assert!(output.success);
        assert_eq!(output.stdout, "value ");
        assert_eq!(output.stderr, "012345");
        assert!(!root.0.join("ignored").exists());
    }

    #[cfg(unix)]
    #[test]
    fn direct_runner_enforces_timeout_and_cancellation() {
        use std::os::unix::fs::PermissionsExt;

        let root = TestRoot::new();
        let executable = root.0.join("slow-provider.sh");
        fs::write(&executable, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let mut request = DirectProcessRequest {
            executable,
            arguments: Vec::new(),
            working_directory: root.0.clone(),
            timeout: Duration::from_millis(40),
            output_limit: 64,
            provider_id: FileConversionProviderId::LibreOffice,
        };

        assert_eq!(
            SystemProcessRunner
                .run(&request, &FileConversionCancellationToken::default())
                .unwrap_err()
                .code,
            FileConversionErrorCode::Timeout
        );

        request.timeout = Duration::from_secs(1);
        let cancellation = FileConversionCancellationToken::default();
        cancellation.cancel();
        assert_eq!(
            SystemProcessRunner
                .run(&request, &cancellation)
                .unwrap_err()
                .code,
            FileConversionErrorCode::Cancelled
        );
    }

    #[cfg(unix)]
    #[test]
    fn timeout_terminates_descendants_in_the_provider_process_group() {
        use std::os::unix::fs::PermissionsExt;

        let root = TestRoot::new();
        let escaped_marker = root.0.join("descendant-escaped");
        let executable = root.0.join("provider-tree.sh");
        fs::write(
            &executable,
            format!(
                "#!/bin/sh\n(sleep 0.3; touch '{}') &\nwhile :; do :; done\n",
                escaped_marker.to_string_lossy()
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let request = DirectProcessRequest {
            executable,
            arguments: Vec::new(),
            working_directory: root.0.clone(),
            timeout: Duration::from_millis(40),
            output_limit: 64,
            provider_id: FileConversionProviderId::LibreOffice,
        };

        assert_eq!(
            SystemProcessRunner
                .run(&request, &FileConversionCancellationToken::default())
                .unwrap_err()
                .code,
            FileConversionErrorCode::Timeout
        );
        thread::sleep(Duration::from_millis(400));
        assert!(!escaped_marker.exists());
    }
}
