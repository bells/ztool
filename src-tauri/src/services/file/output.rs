use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::contracts::{FileConversionDirection, FileConversionError, FileConversionErrorCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputReservation {
    pub output_path: PathBuf,
    pub output_name: String,
    pub output_directory: PathBuf,
}

pub fn reserve_output_path(
    source: &Path,
    direction: FileConversionDirection,
    requested_directory: Option<&Path>,
    reserved_paths: &mut HashSet<PathBuf>,
) -> Result<OutputReservation, FileConversionError> {
    let output_directory = output_directory(source, requested_directory)?;
    validate_writable_directory(&output_directory)?;

    let source_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("document");
    let extension = target_extension(direction);

    for attempt in 0_u32.. {
        let output_name = output_name(source_stem, extension, attempt);
        let output_path = output_directory.join(&output_name);
        if output_path.exists() || reserved_paths.contains(&output_path) {
            continue;
        }

        reserved_paths.insert(output_path.clone());
        return Ok(OutputReservation {
            output_path,
            output_name,
            output_directory,
        });
    }

    unreachable!("the incrementing output-name space is not finite")
}

pub fn target_extension(direction: FileConversionDirection) -> &'static str {
    match direction {
        FileConversionDirection::PdfToDocx => "docx",
        FileConversionDirection::DocxToPdf => "pdf",
    }
}

fn output_directory(
    source: &Path,
    requested_directory: Option<&Path>,
) -> Result<PathBuf, FileConversionError> {
    let directory = requested_directory
        .map(Path::to_path_buf)
        .or_else(|| source.parent().map(Path::to_path_buf))
        .ok_or_else(|| {
            output_error(
                FileConversionErrorCode::OutputNotWritable,
                "The source file has no usable output directory.",
                false,
            )
        })?;

    directory.canonicalize().map_err(|_| {
        output_error(
            FileConversionErrorCode::OutputNotWritable,
            "The selected output directory does not exist or cannot be accessed.",
            true,
        )
    })
}

fn validate_writable_directory(directory: &Path) -> Result<(), FileConversionError> {
    if !directory.is_dir() {
        return Err(output_error(
            FileConversionErrorCode::OutputNotWritable,
            "The selected output location is not a directory.",
            false,
        ));
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let probe = directory.join(format!(
        ".zero-file-write-probe-{}-{nonce}",
        std::process::id()
    ));
    {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .map_err(|_| {
                output_error(
                    FileConversionErrorCode::OutputNotWritable,
                    "Zero cannot write to the selected output directory.",
                    true,
                )
            })?;
    }
    fs::remove_file(&probe).map_err(|_| {
        output_error(
            FileConversionErrorCode::OutputNotWritable,
            "Zero could not complete the output-directory write check.",
            true,
        )
    })?;
    Ok(())
}

fn output_name(stem: &str, extension: &str, attempt: u32) -> String {
    if attempt == 0 {
        format!("{stem}-converted.{extension}")
    } else {
        format!("{stem}-converted-{}.{extension}", attempt + 1)
    }
}

fn output_error(
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::services::file::validation::detect_direction;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-output-{}",
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
    fn detects_both_directions_case_insensitively() {
        assert_eq!(
            detect_direction(Path::new("report.PDF")).unwrap(),
            FileConversionDirection::PdfToDocx
        );
        assert_eq!(
            detect_direction(Path::new("notes.DocX")).unwrap(),
            FileConversionDirection::DocxToPdf
        );
    }

    #[test]
    fn reserves_default_and_incremented_names_without_touching_existing_outputs() {
        let root = TestRoot::new();
        let source = root.0.join("report.pdf");
        let existing = root.0.join("report-converted.docx");
        fs::write(&source, b"%PDF-1.7").unwrap();
        fs::write(&existing, b"keep me").unwrap();
        let mut reserved = HashSet::new();

        let first = reserve_output_path(
            &source,
            FileConversionDirection::PdfToDocx,
            None,
            &mut reserved,
        )
        .unwrap();
        let second = reserve_output_path(
            &source,
            FileConversionDirection::PdfToDocx,
            None,
            &mut reserved,
        )
        .unwrap();

        assert_eq!(first.output_name, "report-converted-2.docx");
        assert_eq!(second.output_name, "report-converted-3.docx");
        assert!(!first.output_path.exists());
        assert!(!second.output_path.exists());
        assert_eq!(fs::read(&existing).unwrap(), b"keep me");
    }

    #[test]
    fn canonicalizes_and_uses_an_optional_output_directory() {
        let root = TestRoot::new();
        let source = root.0.join("notes.docx");
        let output = root.0.join("exports");
        fs::write(&source, b"fixture").unwrap();
        fs::create_dir_all(&output).unwrap();

        let reservation = reserve_output_path(
            &source,
            FileConversionDirection::DocxToPdf,
            Some(&output),
            &mut HashSet::new(),
        )
        .unwrap();

        assert_eq!(reservation.output_directory, output.canonicalize().unwrap());
        assert_eq!(reservation.output_name, "notes-converted.pdf");
    }

    #[test]
    fn rejects_missing_or_non_directory_output_locations() {
        let root = TestRoot::new();
        let source = root.0.join("report.pdf");
        let file_target = root.0.join("not-a-directory");
        fs::write(&source, b"%PDF-1.7").unwrap();
        fs::write(&file_target, b"fixture").unwrap();

        for invalid in [root.0.join("missing"), file_target] {
            let error = reserve_output_path(
                &source,
                FileConversionDirection::PdfToDocx,
                Some(&invalid),
                &mut HashSet::new(),
            )
            .unwrap_err();
            assert_eq!(error.code, FileConversionErrorCode::OutputNotWritable);
        }
    }
}
