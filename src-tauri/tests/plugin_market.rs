use zero_lib::plugins::market::{PluginMarketCache, DEFAULT_PLUGIN_MARKET_URL};

fn valid_market_json() -> &'static str {
    r#"{
        "schemaVersion": 1,
        "updatedAt": "2026-06-21T00:00:00Z",
        "plugins": [
            {
                "name": "clipboard-helper",
                "version": "0.1.0",
                "author": "watson",
                "repository": "https://github.com/watson/clipboard-helper",
                "releaseUrl": "https://github.com/watson/clipboard-helper/releases/tag/v0.1.0",
                "downloadUrl": "https://github.com/watson/clipboard-helper/releases/download/v0.1.0/clipboard-helper.zplugin",
                "permissions": ["clipboard.read"]
            }
        ]
    }"#
}

#[test]
fn parses_and_caches_valid_market_json() {
    let mut cache = PluginMarketCache::new(DEFAULT_PLUGIN_MARKET_URL);

    let snapshot = cache
        .refresh_from_json(valid_market_json())
        .expect("market should parse");

    assert_eq!(snapshot.source_url, DEFAULT_PLUGIN_MARKET_URL);
    assert_eq!(snapshot.entries.len(), 1);
    assert!(snapshot.entries[0].download_url.ends_with(".zplugin"));
    assert_eq!(cache.cached_snapshot().expect("cached").entries.len(), 1);
}

#[test]
fn refresh_with_fetcher_uses_configured_market_url() {
    let mut cache = PluginMarketCache::new(DEFAULT_PLUGIN_MARKET_URL);

    let snapshot = cache
        .refresh_with_fetcher(|url| {
            assert_eq!(url, DEFAULT_PLUGIN_MARKET_URL);
            Ok::<_, String>(valid_market_json().to_string())
        })
        .expect("market should refresh through fetcher");

    assert_eq!(snapshot.entries[0].name, "clipboard-helper");
}

#[test]
fn invalid_market_json_keeps_previous_cache() {
    let mut cache = PluginMarketCache::new(DEFAULT_PLUGIN_MARKET_URL);
    cache
        .refresh_from_json(valid_market_json())
        .expect("initial market should parse");

    let error = cache
        .refresh_from_json("{ definitely not json")
        .expect_err("invalid market should fail");

    assert!(error.message.contains("parse"));
    assert_eq!(cache.cached_snapshot().expect("cached").entries.len(), 1);
}

#[test]
fn fetch_failure_keeps_previous_cache() {
    let mut cache = PluginMarketCache::new(DEFAULT_PLUGIN_MARKET_URL);
    cache
        .refresh_from_json(valid_market_json())
        .expect("initial market should parse");

    let error = cache
        .refresh_with_fetcher(|_| Err::<String, _>("network unavailable"))
        .expect_err("fetch failure should fail");

    assert!(error.message.contains("failed to fetch"));
    assert_eq!(cache.cached_snapshot().expect("cached").entries.len(), 1);
}

#[test]
fn invalid_market_entry_is_rejected() {
    let mut cache = PluginMarketCache::new(DEFAULT_PLUGIN_MARKET_URL);
    let error = cache
        .refresh_from_json(
            r#"{
                "schemaVersion": 1,
                "plugins": [
                    {
                        "name": "bad-plugin",
                        "version": "0.1.0",
                        "author": "watson",
                        "repository": "https://github.com/watson/bad-plugin",
                        "releaseUrl": "https://github.com/watson/bad-plugin/releases/tag/v0.1.0",
                        "downloadUrl": "https://github.com/watson/bad-plugin/releases/download/v0.1.0/bad-plugin.zip",
                        "permissions": ["clipboard.read"]
                    }
                ]
            }"#,
        )
        .expect_err("non-zplugin asset should fail");

    assert!(error.message.contains(".zplugin"));
    assert!(cache.cached_snapshot().is_none());
}
