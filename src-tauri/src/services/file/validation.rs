use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::contracts::{
    FileConversionCandidate, FileConversionCandidateValidation, FileConversionDirection,
    FileConversionError, FileConversionErrorCode,
};
use super::errors::{classify_provider_failure, ProviderFailureKind};

const PDF_HEADER_LENGTH: usize = 8;

pub fn inspect_source(source: &Path, active_sources: &HashSet<PathBuf>) -> FileConversionCandidate {
    let source_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| source.to_string_lossy().into_owned());
    let fallback_path = source.to_string_lossy().into_owned();

    if is_office_lock_file(source) {
        return rejected_candidate(
            fallback_path,
            source_name,
            None,
            FileConversionErrorCode::InvalidInput,
            "Temporary Microsoft Office lock files cannot be converted.",
        );
    }

    let direction = match detect_direction(source) {
        Ok(direction) => direction,
        Err(error) => {
            return FileConversionCandidate {
                source_path: fallback_path,
                source_name,
                size_bytes: None,
                validation: FileConversionCandidateValidation::Rejected { error },
            };
        }
    };

    let canonical = match source.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            return rejected_candidate(
                fallback_path,
                source_name,
                None,
                io_error_code(&error),
                "The source file is missing or cannot be read.",
            );
        }
    };
    let canonical_path = canonical.to_string_lossy().into_owned();

    let metadata = match fs::metadata(&canonical) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return rejected_candidate(
                canonical_path,
                source_name,
                None,
                FileConversionErrorCode::InvalidInput,
                "The selected source is not a regular file.",
            );
        }
        Err(error) => {
            return rejected_candidate(
                canonical_path,
                source_name,
                None,
                io_error_code(&error),
                "The source file metadata cannot be read.",
            );
        }
    };

    if active_sources.contains(&canonical) {
        return rejected_candidate(
            canonical_path,
            source_name,
            Some(metadata.len()),
            FileConversionErrorCode::DuplicateSource,
            "This source file is already queued or running.",
        );
    }

    let validation_error = match direction {
        FileConversionDirection::PdfToDocx => validate_pdf_container(&canonical),
        FileConversionDirection::DocxToPdf => validate_docx_container(&canonical),
    };
    if let Err(error) = validation_error {
        return FileConversionCandidate {
            source_path: canonical_path,
            source_name,
            size_bytes: Some(metadata.len()),
            validation: FileConversionCandidateValidation::Rejected { error },
        };
    }

    FileConversionCandidate {
        source_path: canonical_path,
        source_name,
        size_bytes: Some(metadata.len()),
        validation: FileConversionCandidateValidation::Valid {
            direction,
            proposed_output_name: default_output_name(source, direction),
        },
    }
}

pub fn detect_direction(source: &Path) -> Result<FileConversionDirection, FileConversionError> {
    match source
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("pdf") => Ok(FileConversionDirection::PdfToDocx),
        Some("docx") => Ok(FileConversionDirection::DocxToPdf),
        _ => Err(file_error(
            FileConversionErrorCode::UnsupportedFormat,
            "Only PDF and DOCX source files are supported.",
            false,
        )),
    }
}

pub(crate) fn validate_pdf_container(path: &Path) -> Result<(), FileConversionError> {
    let mut file = File::open(path).map_err(|error| {
        file_error(
            io_error_code(&error),
            "The PDF file cannot be opened for reading.",
            true,
        )
    })?;
    let mut header = [0_u8; PDF_HEADER_LENGTH];
    let read = file.read(&mut header).map_err(|error| {
        file_error(
            io_error_code(&error),
            "The PDF header cannot be read.",
            true,
        )
    })?;
    let valid = read >= 8
        && &header[..5] == b"%PDF-"
        && header[5].is_ascii_digit()
        && header[6] == b'.'
        && header[7].is_ascii_digit();
    if valid {
        reject_known_encrypted_pdf(&mut file)
    } else {
        Err(file_error(
            FileConversionErrorCode::InvalidInput,
            "The file does not have a valid PDF header.",
            false,
        ))
    }
}

fn reject_known_encrypted_pdf(file: &mut File) -> Result<(), FileConversionError> {
    const WINDOW: u64 = 1024 * 1024;
    let length = file
        .metadata()
        .map_err(|_| classify_provider_failure(ProviderFailureKind::MalformedInput, None))?
        .len();
    let mut inspected = Vec::new();
    file.seek(SeekFrom::Start(0))
        .map_err(|_| classify_provider_failure(ProviderFailureKind::MalformedInput, None))?;
    file.by_ref()
        .take(WINDOW)
        .read_to_end(&mut inspected)
        .map_err(|_| classify_provider_failure(ProviderFailureKind::MalformedInput, None))?;
    if length > WINDOW {
        file.seek(SeekFrom::Start(length.saturating_sub(WINDOW)))
            .map_err(|_| classify_provider_failure(ProviderFailureKind::MalformedInput, None))?;
        file.by_ref()
            .take(WINDOW)
            .read_to_end(&mut inspected)
            .map_err(|_| classify_provider_failure(ProviderFailureKind::MalformedInput, None))?;
    }
    if inspected
        .windows(b"/Encrypt".len())
        .any(|window| window == b"/Encrypt")
    {
        Err(classify_provider_failure(
            ProviderFailureKind::PasswordRequired,
            None,
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_docx_container(path: &Path) -> Result<(), FileConversionError> {
    let file = File::open(path).map_err(|error| {
        file_error(
            io_error_code(&error),
            "The DOCX file cannot be opened for reading.",
            true,
        )
    })?;
    let mut archive = ZipArchive::new(file).map_err(|_| {
        file_error(
            FileConversionErrorCode::InvalidInput,
            "The DOCX file is not a readable ZIP package.",
            false,
        )
    })?;

    for required_entry in ["[Content_Types].xml", "word/document.xml"] {
        if archive.by_name(required_entry).is_err() {
            return Err(file_error(
                FileConversionErrorCode::InvalidInput,
                "The DOCX package is missing required document entries.",
                false,
            ));
        }
    }
    Ok(())
}

fn is_office_lock_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("~$") && name.to_ascii_lowercase().ends_with(".docx"))
}

fn default_output_name(source: &Path, direction: FileConversionDirection) -> String {
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("document");
    let extension = match direction {
        FileConversionDirection::PdfToDocx => "docx",
        FileConversionDirection::DocxToPdf => "pdf",
    };
    format!("{stem}-converted.{extension}")
}

fn rejected_candidate(
    source_path: String,
    source_name: String,
    size_bytes: Option<u64>,
    code: FileConversionErrorCode,
    message: &str,
) -> FileConversionCandidate {
    FileConversionCandidate {
        source_path,
        source_name,
        size_bytes,
        validation: FileConversionCandidateValidation::Rejected {
            error: file_error(code, message, false),
        },
    }
}

fn io_error_code(error: &std::io::Error) -> FileConversionErrorCode {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        FileConversionErrorCode::PermissionDenied
    } else {
        FileConversionErrorCode::InvalidInput
    }
}

fn file_error(
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "zero-file-validation-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
            fs::create_dir_all(&root).expect("test root");
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn accepts_pdf_header_and_returns_canonical_candidate() {
        let root = TestRoot::new();
        let source = root.0.join("报告.PDF");
        fs::write(&source, b"%PDF-1.7\nfixture").unwrap();

        let candidate = inspect_source(&source, &HashSet::new());

        assert_eq!(
            candidate.source_path,
            source.canonicalize().unwrap().to_string_lossy()
        );
        assert!(matches!(
            candidate.validation,
            FileConversionCandidateValidation::Valid {
                direction: FileConversionDirection::PdfToDocx,
                ref proposed_output_name,
            } if proposed_output_name == "报告-converted.docx"
        ));
    }

    #[test]
    fn accepts_docx_only_with_required_zip_entries() {
        let root = TestRoot::new();
        let source = root.0.join("notes.docx");
        write_docx(&source, &["[Content_Types].xml", "word/document.xml"]);

        let candidate = inspect_source(&source, &HashSet::new());

        assert!(matches!(
            candidate.validation,
            FileConversionCandidateValidation::Valid {
                direction: FileConversionDirection::DocxToPdf,
                ..
            }
        ));
    }

    #[test]
    fn rejects_missing_paths_directories_and_unsupported_extensions() {
        let root = TestRoot::new();
        let missing = inspect_source(&root.0.join("missing.pdf"), &HashSet::new());
        fs::create_dir_all(root.0.join("folder.docx")).unwrap();
        let directory = inspect_source(&root.0.join("folder.docx"), &HashSet::new());
        let unsupported_path = root.0.join("legacy.doc");
        fs::write(&unsupported_path, b"fixture").unwrap();
        let unsupported = inspect_source(&unsupported_path, &HashSet::new());

        assert_rejected_with(missing, FileConversionErrorCode::InvalidInput);
        assert_rejected_with(directory, FileConversionErrorCode::InvalidInput);
        assert_rejected_with(unsupported, FileConversionErrorCode::UnsupportedFormat);
    }

    #[test]
    fn rejects_invalid_pdf_and_malformed_or_incomplete_docx() {
        let root = TestRoot::new();
        let invalid_pdf_path = root.0.join("renamed.pdf");
        fs::write(&invalid_pdf_path, b"not a pdf").unwrap();
        let malformed_docx_path = root.0.join("malformed.docx");
        fs::write(&malformed_docx_path, b"not a zip").unwrap();
        let incomplete_docx_path = root.0.join("incomplete.docx");
        write_docx(&incomplete_docx_path, &["[Content_Types].xml"]);

        assert_rejected_with(
            inspect_source(&invalid_pdf_path, &HashSet::new()),
            FileConversionErrorCode::InvalidInput,
        );
        assert_rejected_with(
            inspect_source(&malformed_docx_path, &HashSet::new()),
            FileConversionErrorCode::InvalidInput,
        );
        assert_rejected_with(
            inspect_source(&incomplete_docx_path, &HashSet::new()),
            FileConversionErrorCode::InvalidInput,
        );
    }

    #[test]
    fn rejects_a_pdf_with_an_encryption_dictionary_as_password_required() {
        let root = TestRoot::new();
        let encrypted_pdf = root.0.join("protected.pdf");
        fs::write(
            &encrypted_pdf,
            b"%PDF-1.7\n1 0 obj<</Encrypt 2 0 R>>endobj\n%%EOF",
        )
        .unwrap();

        assert_rejected_with(
            inspect_source(&encrypted_pdf, &HashSet::new()),
            FileConversionErrorCode::PasswordRequired,
        );
    }

    #[test]
    fn rejects_office_lock_files_before_container_inspection() {
        let root = TestRoot::new();
        let lock = root.0.join("~$notes.docx");
        fs::write(&lock, b"lock owner").unwrap();

        assert_rejected_with(
            inspect_source(&lock, &HashSet::new()),
            FileConversionErrorCode::InvalidInput,
        );
    }

    #[test]
    fn rejects_an_active_source_using_its_canonical_path() {
        let root = TestRoot::new();
        let source = root.0.join("duplicate.pdf");
        fs::write(&source, b"%PDF-1.7\nfixture").unwrap();
        let active = HashSet::from([source.canonicalize().unwrap()]);

        assert_rejected_with(
            inspect_source(&source, &active),
            FileConversionErrorCode::DuplicateSource,
        );
    }

    fn write_docx(path: &Path, entries: &[&str]) {
        let file = File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        for entry in entries {
            zip.start_file(*entry, SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"fixture").unwrap();
        }
        zip.finish().unwrap();
    }

    fn assert_rejected_with(candidate: FileConversionCandidate, code: FileConversionErrorCode) {
        assert!(matches!(
            candidate.validation,
            FileConversionCandidateValidation::Rejected {
                error: FileConversionError { code: actual, .. }
            } if actual == code
        ));
    }
}
