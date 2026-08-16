use super::contracts::{FileConversionError, FileConversionErrorCode, FileConversionProviderId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderFailureKind {
    EngineUnavailable,
    UnsupportedEngineVersion,
    AutomationPermissionDenied,
    ProviderActivationFailed,
    PasswordRequired,
    OcrRequired,
    MalformedInput,
    UnsupportedFeature,
    PermissionDenied,
    Timeout,
    Cancelled,
    OutputConflict,
    OutputNotWritable,
    ProviderExit { exit_code: Option<i32> },
    InvalidOutput,
}

pub fn classify_provider_failure(
    kind: ProviderFailureKind,
    provider_id: Option<FileConversionProviderId>,
) -> FileConversionError {
    let (code, message, retryable) = match kind {
        ProviderFailureKind::EngineUnavailable => (
            FileConversionErrorCode::EngineUnavailable,
            "No approved local provider is available for this conversion.",
            true,
        ),
        ProviderFailureKind::UnsupportedEngineVersion => (
            FileConversionErrorCode::EngineVersionUnsupported,
            "The installed provider version is outside Zero's approved range.",
            false,
        ),
        ProviderFailureKind::AutomationPermissionDenied => (
            FileConversionErrorCode::AutomationPermissionDenied,
            "The operating system denied document automation permission.",
            true,
        ),
        ProviderFailureKind::ProviderActivationFailed => (
            FileConversionErrorCode::ProviderActivationFailed,
            "The installed provider could not be activated.",
            true,
        ),
        ProviderFailureKind::PasswordRequired => (
            FileConversionErrorCode::PasswordRequired,
            "Password-protected PDFs are not supported in this release.",
            false,
        ),
        ProviderFailureKind::OcrRequired => (
            FileConversionErrorCode::OcrRequired,
            "This PDF contains scanned pages that require an OCR-capable workflow.",
            false,
        ),
        ProviderFailureKind::MalformedInput => (
            FileConversionErrorCode::InvalidInput,
            "The source document container is malformed or incomplete.",
            false,
        ),
        ProviderFailureKind::UnsupportedFeature => (
            FileConversionErrorCode::UnsupportedInput,
            "The local provider does not support a feature used by this document.",
            false,
        ),
        ProviderFailureKind::PermissionDenied => (
            FileConversionErrorCode::PermissionDenied,
            "The operating system denied access required for local conversion.",
            true,
        ),
        ProviderFailureKind::Timeout => (
            FileConversionErrorCode::Timeout,
            "The local provider exceeded the conversion deadline.",
            true,
        ),
        ProviderFailureKind::Cancelled => (
            FileConversionErrorCode::Cancelled,
            "The conversion was cancelled.",
            true,
        ),
        ProviderFailureKind::OutputConflict => (
            FileConversionErrorCode::OutputConflict,
            "The reserved output name is no longer available.",
            true,
        ),
        ProviderFailureKind::OutputNotWritable => (
            FileConversionErrorCode::OutputNotWritable,
            "Zero cannot write the converted file to the selected output directory.",
            true,
        ),
        ProviderFailureKind::ProviderExit { .. } => (
            FileConversionErrorCode::ProviderFailed,
            "The local provider exited before producing a valid converted document.",
            true,
        ),
        ProviderFailureKind::InvalidOutput => (
            FileConversionErrorCode::InvalidProviderOutput,
            "The local provider did not produce a valid converted document.",
            true,
        ),
    };
    let diagnostic = match kind {
        ProviderFailureKind::ProviderExit { exit_code } => Some(format!(
            "providerExitCode={}",
            exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
        )),
        _ => None,
    };

    FileConversionError {
        code,
        message: message.into(),
        retryable,
        provider_id,
        diagnostic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_document_and_runtime_outcomes_have_stable_error_codes() {
        for (kind, expected) in [
            (
                ProviderFailureKind::PasswordRequired,
                FileConversionErrorCode::PasswordRequired,
            ),
            (
                ProviderFailureKind::OcrRequired,
                FileConversionErrorCode::OcrRequired,
            ),
            (
                ProviderFailureKind::MalformedInput,
                FileConversionErrorCode::InvalidInput,
            ),
            (
                ProviderFailureKind::UnsupportedFeature,
                FileConversionErrorCode::UnsupportedInput,
            ),
            (
                ProviderFailureKind::PermissionDenied,
                FileConversionErrorCode::PermissionDenied,
            ),
            (
                ProviderFailureKind::Timeout,
                FileConversionErrorCode::Timeout,
            ),
            (
                ProviderFailureKind::Cancelled,
                FileConversionErrorCode::Cancelled,
            ),
            (
                ProviderFailureKind::InvalidOutput,
                FileConversionErrorCode::InvalidProviderOutput,
            ),
        ] {
            assert_eq!(classify_provider_failure(kind, None).code, expected);
        }
    }

    #[test]
    fn provider_exit_diagnostics_contain_only_a_bounded_synthetic_exit_code() {
        let error = classify_provider_failure(
            ProviderFailureKind::ProviderExit { exit_code: Some(7) },
            Some(FileConversionProviderId::LibreOffice),
        );

        assert_eq!(error.code, FileConversionErrorCode::ProviderFailed);
        assert_eq!(error.diagnostic.as_deref(), Some("providerExitCode=7"));
        assert!(!error.message.contains('/'));
        assert!(!error.message.contains('\\'));
    }

    #[test]
    fn permission_activation_and_missing_engine_remain_distinct() {
        assert_eq!(
            classify_provider_failure(ProviderFailureKind::EngineUnavailable, None).code,
            FileConversionErrorCode::EngineUnavailable
        );
        assert_eq!(
            classify_provider_failure(
                ProviderFailureKind::AutomationPermissionDenied,
                Some(FileConversionProviderId::MicrosoftWordMacos)
            )
            .code,
            FileConversionErrorCode::AutomationPermissionDenied
        );
        assert_eq!(
            classify_provider_failure(
                ProviderFailureKind::ProviderActivationFailed,
                Some(FileConversionProviderId::MicrosoftWordWindows)
            )
            .code,
            FileConversionErrorCode::ProviderActivationFailed
        );
    }
}
