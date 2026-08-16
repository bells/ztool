use std::path::{Path, PathBuf};

pub const PRODUCT_NAME: &str = "Zero";
pub const PRIMARY_STATUS_ITEM_ID: &str = "zero.primary";
pub const ZERO_LAUNCH_PLUGIN_ID: &str = "zero.launch";
pub const ZERO_SNAP_PLUGIN_ID: &str = "zero.snap";
pub const ZERO_AWAKE_PLUGIN_ID: &str = "zero.awake";
pub const ZERO_PAPER_PLUGIN_ID: &str = "zero.paper";
pub const ZERO_FILE_PLUGIN_ID: &str = "zero.file";

const LEGACY_PLUGIN_IDS: [(&str, &str); 4] = [
    ("ztool.quick-launcher", ZERO_LAUNCH_PLUGIN_ID),
    ("ztool.screenshot", ZERO_SNAP_PLUGIN_ID),
    ("ztool.caffeine", ZERO_AWAKE_PLUGIN_ID),
    ("ztool.bing-wallpaper", ZERO_PAPER_PLUGIN_ID),
];

pub fn canonical_first_party_plugin_id(value: &str) -> &str {
    LEGACY_PLUGIN_IDS
        .iter()
        .find_map(|(legacy, canonical)| (*legacy == value).then_some(*canonical))
        .unwrap_or(value)
}

pub fn canonical_first_party_contribution_id(value: &str) -> String {
    for (legacy, canonical) in LEGACY_PLUGIN_IDS {
        if value == legacy {
            return canonical.to_string();
        }

        if let Some(suffix) = value
            .strip_prefix(legacy)
            .filter(|suffix| suffix.starts_with('.'))
        {
            return format!("{canonical}{suffix}");
        }
    }

    value.to_string()
}

pub fn canonical_data_root(home: &Path) -> PathBuf {
    home.join(".zero")
}

pub fn legacy_data_root(home: &Path) -> PathBuf {
    home.join(".ztool")
}

pub fn default_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_first_party_contribution_id, canonical_first_party_plugin_id,
        ZERO_AWAKE_PLUGIN_ID, ZERO_FILE_PLUGIN_ID, ZERO_LAUNCH_PLUGIN_ID, ZERO_PAPER_PLUGIN_ID,
        ZERO_SNAP_PLUGIN_ID,
    };

    #[test]
    fn maps_only_known_legacy_first_party_ids() {
        assert_eq!(ZERO_FILE_PLUGIN_ID, "zero.file");
        assert_eq!(
            canonical_first_party_plugin_id("ztool.quick-launcher"),
            ZERO_LAUNCH_PLUGIN_ID
        );
        assert_eq!(
            canonical_first_party_plugin_id("ztool.screenshot"),
            ZERO_SNAP_PLUGIN_ID
        );
        assert_eq!(
            canonical_first_party_plugin_id("ztool.caffeine"),
            ZERO_AWAKE_PLUGIN_ID
        );
        assert_eq!(
            canonical_first_party_plugin_id("ztool.bing-wallpaper"),
            ZERO_PAPER_PLUGIN_ID
        );
        assert_eq!(
            canonical_first_party_plugin_id("ztool.third-party"),
            "ztool.third-party"
        );
    }

    #[test]
    fn maps_known_contribution_prefixes_without_touching_other_plugins() {
        assert_eq!(
            canonical_first_party_contribution_id("ztool.screenshot.capture"),
            "zero.snap.capture"
        );
        assert_eq!(
            canonical_first_party_contribution_id("ztool.third-party.capture"),
            "ztool.third-party.capture"
        );
    }
}
