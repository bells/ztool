use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuickLauncherItemKind {
    Application,
    SystemSetting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuickLauncherIndexSource {
    Empty,
    Cache,
    Scan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuickLauncherPlatformSupport {
    Supported,
    Degraded,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuickLauncherRunningState {
    Running,
    NotRunning,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherIndexSnapshot {
    pub revision: u64,
    pub source: QuickLauncherIndexSource,
    pub refreshing: bool,
    pub item_count: usize,
    pub last_updated_at: Option<u64>,
    pub platform_support: QuickLauncherPlatformSupport,
    pub diagnostics: Vec<QuickLauncherDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickLauncherSearchInput {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherResultItem {
    pub id: String,
    pub kind: QuickLauncherItemKind,
    pub title: String,
    pub subtitle: String,
    pub running: QuickLauncherRunningState,
    pub icon_key: Option<String>,
    pub matched_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherSearchResult {
    pub revision: u64,
    pub query: String,
    pub elapsed_micros: u64,
    pub items: Vec<QuickLauncherResultItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickLauncherActivateInput {
    pub item_id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuickLauncherActivationAction {
    Focused,
    Launched,
    LaunchedFallback,
    OpenedSetting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherActivationResult {
    pub item_id: String,
    pub action: QuickLauncherActivationAction,
    pub usage_count: u64,
    pub activated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickLauncherIconInput {
    pub item_id: String,
    pub icon_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherIconResult {
    pub item_id: String,
    pub data_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuickLauncherIconBatchInput {
    pub items: Vec<QuickLauncherIconInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherIconBatchResult {
    pub results: Vec<QuickLauncherIconResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherRunningSnapshot {
    pub index_revision: u64,
    pub running_revision: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLauncherError {
    pub operation: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for QuickLauncherError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for QuickLauncherError {}

pub fn launcher_error(
    operation: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
) -> QuickLauncherError {
    QuickLauncherError {
        operation: operation.into(),
        code: code.into(),
        message: message.into(),
        retryable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_use_stable_camel_case_fields_and_values() {
        let search = QuickLauncherSearchInput {
            query: "wx".into(),
            limit: Some(12),
        };
        let activation = QuickLauncherActivationResult {
            item_id: "app:macos:1".into(),
            action: QuickLauncherActivationAction::LaunchedFallback,
            usage_count: 3,
            activated_at: 42,
        };
        let search_json = serde_json::to_value(search).unwrap();
        let activation_json = serde_json::to_value(activation).unwrap();
        assert_eq!(search_json["limit"], 12);
        assert_eq!(activation_json["itemId"], "app:macos:1");
        assert_eq!(activation_json["action"], "launchedFallback");
        assert_eq!(activation_json["usageCount"], 3);
    }

    #[test]
    fn execution_inputs_reject_arbitrary_targets_and_wrong_types() {
        assert!(
            serde_json::from_value::<QuickLauncherActivateInput>(serde_json::json!({
                "itemId": "app:macos:1",
                "revision": 1,
                "path": "/Applications/Bad.app"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<QuickLauncherIconInput>(serde_json::json!({
                "itemId": 7
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<QuickLauncherIconBatchInput>(serde_json::json!({
                "items": [],
                "concurrency": 100
            }))
            .is_err()
        );
    }

    #[test]
    fn unsupported_enum_values_fail_deserialization() {
        assert!(
            serde_json::from_value::<QuickLauncherActivationAction>(serde_json::json!(
                "pretendedFocus"
            ))
            .is_err()
        );
        assert!(
            serde_json::from_value::<QuickLauncherRunningState>(serde_json::json!("maybe"))
                .is_err()
        );
    }
}
