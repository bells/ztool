use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileConversionDirection {
    PdfToDocx,
    DocxToPdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileConversionProviderId {
    ZeroFilePdfToDocx,
    ZeroFileDocxToPdfMacos,
    LibreOffice,
    MicrosoftWordMacos,
    MicrosoftWordWindows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileConversionProviderOrigin {
    BuiltIn,
    Compatibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileConversionQualityProfile {
    EditableReconstruction,
    LayoutPreserving,
    WebRenderedPdf,
    CompatibilityProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileConversionErrorCode {
    InvalidInput,
    UnsupportedFormat,
    DuplicateSource,
    EngineUnavailable,
    EngineVersionUnsupported,
    AutomationPermissionDenied,
    ProviderActivationFailed,
    PasswordRequired,
    OcrRequired,
    UnsupportedInput,
    PermissionDenied,
    Timeout,
    Cancelled,
    OutputConflict,
    OutputNotWritable,
    ProviderFailed,
    InvalidProviderOutput,
    OutputMissing,
    UnknownJob,
    InvalidJobState,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionError {
    pub code: FileConversionErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<FileConversionProviderId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileConversionProviderAvailability {
    Available,
    Unavailable { error: FileConversionError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionProvider {
    pub id: FileConversionProviderId,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub origin: FileConversionProviderOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_minimum: Option<String>,
    pub quality_profiles: Vec<FileConversionQualityProfile>,
    pub directions: Vec<FileConversionDirection>,
    pub availability: FileConversionProviderAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionDirectionCapability {
    pub direction: FileConversionDirection,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_provider_id: Option<FileConversionProviderId>,
    pub providers: Vec<FileConversionProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailability: Option<FileConversionError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionCapabilitySnapshot {
    pub directions: Vec<FileConversionDirectionCapability>,
    pub refreshed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FileConversionCandidateValidation {
    Valid {
        direction: FileConversionDirection,
        proposed_output_name: String,
    },
    Rejected {
        error: FileConversionError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionCandidate {
    pub source_path: String,
    pub source_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    pub validation: FileConversionCandidateValidation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileConversionEnqueueItem {
    pub source_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileConversionEnqueueRequest {
    pub items: Vec<FileConversionEnqueueItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileConversionInspectRequest {
    pub source_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileConversionJobRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileConversionStage {
    Validating,
    WaitingForProvider,
    Converting,
    Finalizing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileConversionProgress {
    Indeterminate {
        stage: FileConversionStage,
    },
    Percentage {
        stage: FileConversionStage,
        percent: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionResult {
    pub output_path: String,
    pub output_name: String,
    pub size_bytes: u64,
    pub completed_at_ms: u64,
    pub provider_id: FileConversionProviderId,
    pub provider_origin: FileConversionProviderOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_version: Option<String>,
    pub quality_profile: FileConversionQualityProfile,
    pub warning_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FileConversionJobState {
    Queued,
    Preparing { stage: FileConversionStage },
    Running { progress: FileConversionProgress },
    Completed { result: FileConversionResult },
    Failed { error: FileConversionError },
    Cancelled { error: FileConversionError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionJobSnapshot {
    pub id: String,
    pub source_path: String,
    pub source_name: String,
    pub size_bytes: u64,
    pub direction: FileConversionDirection,
    pub target_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<FileConversionProviderId>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub state: FileConversionJobState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileConversionBatchResult {
    pub jobs: Vec<FileConversionJobSnapshot>,
    pub rejected_candidates: Vec<FileConversionCandidate>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn serializes_running_job_state_as_a_camel_case_discriminated_union() {
        let state = FileConversionJobState::Running {
            progress: FileConversionProgress::Percentage {
                stage: FileConversionStage::Converting,
                percent: 42,
            },
        };

        assert_eq!(
            serde_json::to_value(state).unwrap(),
            json!({
                "status": "running",
                "progress": {
                    "kind": "percentage",
                    "stage": "converting",
                    "percent": 42
                }
            })
        );
    }

    #[test]
    fn serializes_structured_errors_without_absent_optional_diagnostics() {
        let error = FileConversionError {
            code: FileConversionErrorCode::EngineVersionUnsupported,
            message: "The detected provider version is not supported.".into(),
            retryable: false,
            provider_id: Some(FileConversionProviderId::LibreOffice),
            diagnostic: None,
        };

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({
                "code": "engineVersionUnsupported",
                "message": "The detected provider version is not supported.",
                "retryable": false,
                "providerId": "libreOffice"
            })
        );
    }

    #[test]
    fn deserializes_explicit_enqueue_items() {
        let request: FileConversionEnqueueRequest = serde_json::from_value(json!({
            "items": [
                {
                    "sourcePath": "/tmp/report.pdf",
                    "outputDirectory": "/tmp/converted"
                }
            ]
        }))
        .unwrap();

        assert_eq!(request.items.len(), 1);
        assert_eq!(request.items[0].source_path, "/tmp/report.pdf");
        assert_eq!(
            request.items[0].output_directory.as_deref(),
            Some("/tmp/converted")
        );
    }

    #[test]
    fn rejects_missing_fields_unknown_result_paths_and_invalid_enums() {
        assert!(
            serde_json::from_value::<FileConversionEnqueueRequest>(json!({
                "items": [{ "outputDirectory": "/tmp" }]
            }))
            .is_err()
        );
        assert!(serde_json::from_value::<FileConversionJobRequest>(json!({
            "jobId": "job-1",
            "resultPath": "/tmp/arbitrary.pdf"
        }))
        .is_err());
        assert!(serde_json::from_value::<FileConversionDirection>(json!("pdfToWord")).is_err());
    }
}
