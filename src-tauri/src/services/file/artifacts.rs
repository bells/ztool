use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::contracts::{FileConversionDirection, FileConversionError, FileConversionErrorCode};
use super::validation::{validate_docx_container, validate_pdf_container};

const JOB_DIRECTORY_PREFIX: &str = "job-";

pub fn create_job_temp_directory(
    temp_root: &Path,
    job_id: &str,
) -> Result<PathBuf, FileConversionError> {
    if !valid_job_id(job_id) {
        return Err(artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion job identifier is invalid.",
            false,
        ));
    }

    create_owner_only_directory_tree(temp_root)?;
    let job_directory = temp_root.join(format!("{JOB_DIRECTORY_PREFIX}{job_id}"));
    fs::create_dir(&job_directory).map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The private conversion workspace could not be created.",
            true,
        )
    })?;
    set_owner_only_directory_permissions(&job_directory)?;
    Ok(job_directory)
}

pub fn remove_job_temp_directory(
    temp_root: &Path,
    job_directory: &Path,
) -> Result<(), FileConversionError> {
    let canonical_root = temp_root.canonicalize().map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace root is unavailable.",
            true,
        )
    })?;
    let canonical_job = job_directory.canonicalize().map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace is unavailable.",
            true,
        )
    })?;
    if canonical_job.parent() != Some(canonical_root.as_path())
        || !canonical_job
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(JOB_DIRECTORY_PREFIX))
    {
        return Err(artifact_error(
            FileConversionErrorCode::Internal,
            "Refusing to clean a directory outside the conversion workspace.",
            false,
        ));
    }

    fs::remove_dir_all(canonical_job).map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace could not be cleaned.",
            true,
        )
    })
}

pub fn cleanup_stale_job_directories(temp_root: &Path) -> Result<usize, FileConversionError> {
    if !temp_root.exists() {
        return Ok(0);
    }
    let canonical_root = temp_root.canonicalize().map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace root is unavailable.",
            true,
        )
    })?;
    let mut removed = 0;

    for entry in fs::read_dir(&canonical_root).map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace root cannot be inspected.",
            true,
        )
    })? {
        let entry = entry.map_err(|_| {
            artifact_error(
                FileConversionErrorCode::Internal,
                "A stale conversion workspace cannot be inspected.",
                true,
            )
        })?;
        let file_name = entry.file_name();
        let owned_job = file_name
            .to_str()
            .is_some_and(|name| name.starts_with(JOB_DIRECTORY_PREFIX));
        if owned_job && entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            fs::remove_dir_all(entry.path()).map_err(|_| {
                artifact_error(
                    FileConversionErrorCode::Internal,
                    "A stale conversion workspace could not be cleaned.",
                    true,
                )
            })?;
            removed += 1;
        }
    }

    Ok(removed)
}

pub fn validate_provider_output(
    output: &Path,
    direction: FileConversionDirection,
) -> Result<(), FileConversionError> {
    let metadata = fs::metadata(output).map_err(|_| invalid_provider_output())?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(invalid_provider_output());
    }

    let result = match direction {
        FileConversionDirection::PdfToDocx => validate_docx_container(output),
        FileConversionDirection::DocxToPdf => validate_pdf_container(output),
    };
    result.map_err(|_| invalid_provider_output())
}

pub fn commit_provider_output(
    job_directory: &Path,
    provider_output: &Path,
    final_output: &Path,
    direction: FileConversionDirection,
    job_id: &str,
) -> Result<(), FileConversionError> {
    ensure_provider_output_is_owned(job_directory, provider_output)?;
    validate_provider_output(provider_output, direction)?;
    if !valid_job_id(job_id) {
        return Err(artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion job identifier is invalid.",
            false,
        ));
    }
    if final_output.exists() {
        return Err(output_conflict());
    }

    let final_directory = final_output.parent().ok_or_else(|| {
        artifact_error(
            FileConversionErrorCode::OutputNotWritable,
            "The final output has no destination directory.",
            false,
        )
    })?;
    let output_name = final_output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("converted-output");
    let staging = final_directory.join(format!(".{output_name}.zero-{job_id}.partial"));

    let commit_result = (|| {
        let mut source = BufReader::new(File::open(provider_output).map_err(|_| {
            artifact_error(
                FileConversionErrorCode::InvalidProviderOutput,
                "The provider output is no longer readable.",
                true,
            )
        })?);
        let staging_file = open_owner_only_staging_file(&staging)?;
        let mut target = BufWriter::new(staging_file);
        io::copy(&mut source, &mut target).map_err(|_| {
            artifact_error(
                FileConversionErrorCode::OutputNotWritable,
                "The converted file could not be staged in the output directory.",
                true,
            )
        })?;
        target.flush().map_err(|_| {
            artifact_error(
                FileConversionErrorCode::OutputNotWritable,
                "The staged converted file could not be flushed.",
                true,
            )
        })?;
        target.get_ref().sync_all().map_err(|_| {
            artifact_error(
                FileConversionErrorCode::OutputNotWritable,
                "The staged converted file could not be synchronized.",
                true,
            )
        })?;
        drop(target);

        validate_provider_output(&staging, direction)?;
        fs::hard_link(&staging, final_output).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists || final_output.exists() {
                output_conflict()
            } else {
                artifact_error(
                    FileConversionErrorCode::OutputNotWritable,
                    "The converted file could not be committed to the output directory.",
                    true,
                )
            }
        })?;
        Ok(())
    })();

    let _ = fs::remove_file(&staging);
    commit_result
}

fn ensure_provider_output_is_owned(
    job_directory: &Path,
    provider_output: &Path,
) -> Result<(), FileConversionError> {
    let canonical_job = job_directory
        .canonicalize()
        .map_err(|_| invalid_provider_output())?;
    let canonical_output = provider_output
        .canonicalize()
        .map_err(|_| invalid_provider_output())?;
    if canonical_output.parent() != Some(canonical_job.as_path()) {
        return Err(artifact_error(
            FileConversionErrorCode::InvalidProviderOutput,
            "The provider output was created outside its private workspace.",
            false,
        ));
    }
    Ok(())
}

fn valid_job_id(job_id: &str) -> bool {
    !job_id.is_empty()
        && job_id.len() <= 128
        && job_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn create_owner_only_directory_tree(path: &Path) -> Result<(), FileConversionError> {
    fs::create_dir_all(path).map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace root could not be created.",
            true,
        )
    })?;
    set_owner_only_directory_permissions(path)
}

#[cfg(unix)]
fn set_owner_only_directory_permissions(path: &Path) -> Result<(), FileConversionError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| {
        artifact_error(
            FileConversionErrorCode::Internal,
            "The conversion workspace permissions could not be restricted.",
            true,
        )
    })
}

#[cfg(not(unix))]
fn set_owner_only_directory_permissions(_path: &Path) -> Result<(), FileConversionError> {
    Ok(())
}

#[cfg(unix)]
fn open_owner_only_staging_file(path: &Path) -> Result<File, FileConversionError> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| staging_open_error(path, error))
}

#[cfg(not(unix))]
fn open_owner_only_staging_file(path: &Path) -> Result<File, FileConversionError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| staging_open_error(path, error))
}

fn staging_open_error(path: &Path, error: io::Error) -> FileConversionError {
    if error.kind() == io::ErrorKind::AlreadyExists || path.exists() {
        output_conflict()
    } else {
        artifact_error(
            FileConversionErrorCode::OutputNotWritable,
            "The converted file could not be staged in the output directory.",
            true,
        )
    }
}

fn invalid_provider_output() -> FileConversionError {
    artifact_error(
        FileConversionErrorCode::InvalidProviderOutput,
        "The local provider did not produce a valid converted document.",
        true,
    )
}

fn output_conflict() -> FileConversionError {
    artifact_error(
        FileConversionErrorCode::OutputConflict,
        "The reserved output name is no longer available.",
        true,
    )
}

fn artifact_error(
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
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);
            let root = std::env::temp_dir().join(format!(
                "zero-file-artifacts-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
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
    fn creates_and_removes_an_owned_job_directory() {
        let root = TestRoot::new();
        let temp_root = root.0.join("file-conversion");
        let job = create_job_temp_directory(&temp_root, "job_123").unwrap();

        assert_eq!(job.parent(), Some(temp_root.as_path()));
        assert!(job.is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&job).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }

        remove_job_temp_directory(&temp_root, &job).unwrap();
        assert!(!job.exists());
    }

    #[test]
    fn stale_cleanup_removes_only_owned_job_directories() {
        let root = TestRoot::new();
        let temp_root = root.0.join("file-conversion");
        fs::create_dir_all(temp_root.join("job-stale-one")).unwrap();
        fs::create_dir_all(temp_root.join("job-stale-two")).unwrap();
        fs::create_dir_all(temp_root.join("keep-directory")).unwrap();
        fs::write(temp_root.join("job-not-a-directory"), b"keep").unwrap();

        assert_eq!(cleanup_stale_job_directories(&temp_root).unwrap(), 2);
        assert!(temp_root.join("keep-directory").exists());
        assert!(temp_root.join("job-not-a-directory").exists());
    }

    #[test]
    fn validates_and_commits_provider_output_without_overwrite() {
        let root = TestRoot::new();
        let temp_root = root.0.join("file-conversion");
        let job = create_job_temp_directory(&temp_root, "commit-test").unwrap();
        let provider_output = job.join("provider.pdf");
        fs::write(&provider_output, b"%PDF-1.7\nfixture").unwrap();
        let final_output = root.0.join("notes-converted.pdf");

        commit_provider_output(
            &job,
            &provider_output,
            &final_output,
            FileConversionDirection::DocxToPdf,
            "commit-test",
        )
        .unwrap();

        assert_eq!(fs::read(&final_output).unwrap(), b"%PDF-1.7\nfixture");
        assert_eq!(fs::read(&provider_output).unwrap(), b"%PDF-1.7\nfixture");
    }

    #[test]
    fn rejects_invalid_provider_output_and_preserves_existing_files() {
        let root = TestRoot::new();
        let temp_root = root.0.join("file-conversion");
        let job = create_job_temp_directory(&temp_root, "failure-test").unwrap();
        let provider_output = job.join("provider.docx");
        fs::write(&provider_output, b"not a DOCX").unwrap();
        let final_output = root.0.join("report-converted.docx");
        fs::write(&final_output, b"existing output").unwrap();

        let error = commit_provider_output(
            &job,
            &provider_output,
            &final_output,
            FileConversionDirection::PdfToDocx,
            "failure-test",
        )
        .unwrap_err();

        assert_eq!(error.code, FileConversionErrorCode::InvalidProviderOutput);
        assert_eq!(fs::read(&final_output).unwrap(), b"existing output");
        assert_eq!(fs::read(&provider_output).unwrap(), b"not a DOCX");
    }

    #[test]
    fn detects_a_late_output_collision_and_removes_staging_files() {
        let root = TestRoot::new();
        let temp_root = root.0.join("file-conversion");
        let job = create_job_temp_directory(&temp_root, "collision-test").unwrap();
        let provider_output = job.join("provider.pdf");
        fs::write(&provider_output, b"%PDF-1.7\nfixture").unwrap();
        let final_output = root.0.join("notes-converted.pdf");
        fs::write(&final_output, b"existing output").unwrap();

        let error = commit_provider_output(
            &job,
            &provider_output,
            &final_output,
            FileConversionDirection::DocxToPdf,
            "collision-test",
        )
        .unwrap_err();

        assert_eq!(error.code, FileConversionErrorCode::OutputConflict);
        assert_eq!(fs::read(&final_output).unwrap(), b"existing output");
        assert!(!root
            .0
            .join(".notes-converted.pdf.zero-collision-test.partial")
            .exists());
    }

    #[test]
    fn accepts_a_valid_docx_provider_result() {
        let root = TestRoot::new();
        let output = root.0.join("converted.docx");
        let file = File::create(&output).unwrap();
        let mut archive = ZipWriter::new(file);
        for entry in ["[Content_Types].xml", "word/document.xml"] {
            archive
                .start_file(entry, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(b"fixture").unwrap();
        }
        archive.finish().unwrap();

        validate_provider_output(&output, FileConversionDirection::PdfToDocx).unwrap();
    }
}
