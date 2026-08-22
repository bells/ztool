use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use pinyin::ToPinyin;

use super::contracts::{
    launcher_error, QuickLauncherError, QuickLauncherResultItem, QuickLauncherRunningState,
    QuickLauncherSearchInput, QuickLauncherSearchResult,
};
use super::model::{IndexedItem, SearchFields, UsageMap};

pub const MAX_QUERY_CHARS: usize = 128;
pub const DEFAULT_RESULT_LIMIT: usize = 24;
pub const MAX_RESULT_LIMIT: usize = 50;

thread_local! {
    static THREAD_MATCHER: RefCell<Matcher> = RefCell::new(default_matcher());
}

pub trait Romanizer {
    fn romanize(&self, value: &str) -> (String, String);
}

#[derive(Default)]
pub struct PinyinRomanizer;

impl Romanizer for PinyinRomanizer {
    fn romanize(&self, value: &str) -> (String, String) {
        let mut full = String::new();
        let mut initials = String::new();
        for character in value.chars() {
            match character.to_pinyin() {
                Some(pinyin) => {
                    full.push_str(pinyin.plain());
                    initials.push_str(pinyin.first_letter());
                }
                None if character.is_ascii_alphanumeric() => {
                    full.extend(character.to_lowercase());
                }
                None => {}
            }
        }
        (full, initials)
    }
}

pub fn normalize_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn build_search_fields(title: &str, aliases: Vec<String>) -> SearchFields {
    let romanizer = PinyinRomanizer;
    let normalized_title = normalize_text(title);
    let (pinyin_full, pinyin_initials) = romanizer.romanize(title);
    let word_initials = normalized_title
        .split(|character: char| !character.is_alphanumeric())
        .filter_map(|word| word.chars().next())
        .collect::<String>();
    let mut aliases = aliases
        .into_iter()
        .map(|alias| normalize_text(&alias))
        .filter(|alias| !alias.is_empty())
        .collect::<Vec<_>>();
    aliases.sort();
    aliases.dedup();

    SearchFields {
        normalized_title,
        pinyin_full,
        pinyin_initials,
        word_initials,
        aliases,
    }
}

pub fn bundled_aliases(title: &str) -> Vec<String> {
    let normalized = normalize_text(title);
    let aliases: &[&str] = if normalized.contains("photoshop") {
        &["ps", "adobe ps"]
    } else if normalized.contains("visual studio code") {
        &["vscode", "code"]
    } else if normalized.contains("wechat") || title.contains("微信") {
        &["wx", "weixin", "wechat"]
    } else if normalized.contains("chrome") {
        &["chrome", "gc"]
    } else {
        &[]
    };
    aliases.iter().map(|alias| (*alias).to_string()).collect()
}

pub fn search_items(
    revision: u64,
    items: &[IndexedItem],
    usage: &UsageMap,
    input: QuickLauncherSearchInput,
    matcher: &mut Matcher,
) -> Result<QuickLauncherSearchResult, QuickLauncherError> {
    search_items_with_running(revision, items, usage, &HashMap::new(), input, matcher)
}

pub fn search_items_with_running(
    revision: u64,
    items: &[IndexedItem],
    usage: &UsageMap,
    running_states: &HashMap<String, QuickLauncherRunningState>,
    input: QuickLauncherSearchInput,
    matcher: &mut Matcher,
) -> Result<QuickLauncherSearchResult, QuickLauncherError> {
    let started = std::time::Instant::now();
    if input.query.chars().count() > MAX_QUERY_CHARS {
        return Err(launcher_error(
            "launcher.search",
            "launcher.query_too_long",
            format!("Search query must be at most {MAX_QUERY_CHARS} characters."),
            false,
        ));
    }
    let limit = input.limit.unwrap_or(DEFAULT_RESULT_LIMIT);
    if limit == 0 || limit > MAX_RESULT_LIMIT {
        return Err(launcher_error(
            "launcher.search",
            "launcher.limit_invalid",
            format!("Search result limit must be between 1 and {MAX_RESULT_LIMIT}."),
            false,
        ));
    }

    let query = normalize_text(&input.query);
    let now = unix_timestamp();
    let pattern = Pattern::new(
        &query,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let mut ranked = items
        .iter()
        .filter_map(|item| {
            let running = running_states
                .get(&item.id)
                .copied()
                .unwrap_or(item.running);
            rank_item(item, running, usage, &query, &pattern, matcher, now)
        })
        .collect::<Vec<_>>();
    ranked.sort_by_key(|ranked| {
        (
            Reverse(ranked.score),
            Reverse(ranked.last_used_at),
            ranked.item.title.to_lowercase(),
            ranked.item.id.clone(),
        )
    });

    Ok(QuickLauncherSearchResult {
        revision,
        query: input.query,
        elapsed_micros: started.elapsed().as_micros().min(u64::MAX as u128) as u64,
        items: ranked
            .into_iter()
            .take(limit)
            .map(|ranked| QuickLauncherResultItem {
                id: ranked.item.id.clone(),
                kind: ranked.item.kind,
                title: ranked.item.title.clone(),
                subtitle: ranked.item.subtitle.clone(),
                running: ranked.running,
                icon_key: ranked.item.icon_key.clone(),
                matched_field: ranked.matched_field,
            })
            .collect(),
    })
}

pub fn search_items_thread_local(
    revision: u64,
    items: &[IndexedItem],
    usage: &UsageMap,
    running_states: &HashMap<String, QuickLauncherRunningState>,
    input: QuickLauncherSearchInput,
) -> Result<QuickLauncherSearchResult, QuickLauncherError> {
    THREAD_MATCHER.with(|matcher| {
        search_items_with_running(
            revision,
            items,
            usage,
            running_states,
            input,
            &mut matcher.borrow_mut(),
        )
    })
}

struct RankedItem<'a> {
    item: &'a IndexedItem,
    running: QuickLauncherRunningState,
    score: u64,
    last_used_at: u64,
    matched_field: String,
}

fn rank_item<'a>(
    item: &'a IndexedItem,
    running: QuickLauncherRunningState,
    usage: &UsageMap,
    query: &str,
    pattern: &Pattern,
    matcher: &mut Matcher,
    now: u64,
) -> Option<RankedItem<'a>> {
    let usage_entry = usage.get(&item.id);
    let frequency = usage_entry.map(|entry| entry.count).unwrap_or_default();
    let last_used_at = usage_entry
        .map(|entry| entry.last_used_at)
        .unwrap_or_default();
    let usage_bonus = frequency.saturating_add(1).ilog2().min(5) as u64 * 6;
    let recency_bonus = if last_used_at > 0 {
        let age_days = now.saturating_sub(last_used_at) / 86_400;
        20_u64.saturating_sub(age_days.min(20))
    } else {
        0
    };
    let running_bonus = matches!(
        running,
        super::contracts::QuickLauncherRunningState::Running
    ) as u64
        * 4;

    if query.is_empty() {
        let common_setting_bonus = matches!(
            item.kind,
            super::contracts::QuickLauncherItemKind::SystemSetting
        ) as u64
            * 2;
        return Some(RankedItem {
            item,
            running,
            score: usage_bonus + recency_bonus + running_bonus + common_setting_bonus,
            last_used_at,
            matched_field: "recent".into(),
        });
    }

    let fields = std::iter::once(("title", item.search.normalized_title.as_str(), 40_u64))
        .chain(std::iter::once((
            "wordInitials",
            item.search.word_initials.as_str(),
            30,
        )))
        .chain(std::iter::once((
            "pinyin",
            item.search.pinyin_full.as_str(),
            24,
        )))
        .chain(std::iter::once((
            "pinyinInitials",
            item.search.pinyin_initials.as_str(),
            26,
        )))
        .chain(
            item.search
                .aliases
                .iter()
                .map(|alias| ("alias", alias.as_str(), 34)),
        );
    let mut best: Option<(u64, String)> = None;
    let mut buffer = Vec::new();
    for (field_name, value, field_bonus) in fields {
        if value.is_empty() {
            continue;
        }
        let tier = if value == query {
            4_u64
        } else if value.starts_with(query) {
            3
        } else if value.contains(query) {
            2
        } else {
            1
        };
        let Some(fuzzy) = pattern.score(Utf32Str::new(value, &mut buffer), matcher) else {
            continue;
        };
        let fuzzy = fuzzy as u64;
        let text_score = tier * 1_000_000 + fuzzy * 100 + field_bonus;
        if match best.as_ref() {
            Some((score, _)) => text_score > *score,
            None => true,
        } {
            best = Some((text_score, field_name.into()));
        }
    }
    let (text_score, matched_field) = best?;

    Some(RankedItem {
        item,
        running,
        score: text_score + usage_bonus + recency_bonus + running_bonus,
        last_used_at,
        matched_field,
    })
}

pub fn default_matcher() -> Matcher {
    Matcher::new(Config::DEFAULT.match_paths())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::quick_launcher::contracts::{
        QuickLauncherItemKind, QuickLauncherRunningState,
    };
    use crate::services::quick_launcher::model::{stable_item_id, LaunchTarget, UsageEntry};

    fn item(title: &str, aliases: Vec<String>) -> IndexedItem {
        IndexedItem {
            id: stable_item_id("test", QuickLauncherItemKind::Application, title),
            kind: QuickLauncherItemKind::Application,
            title: title.into(),
            subtitle: format!("/Applications/{title}.app"),
            search: build_search_fields(title, aliases),
            target: LaunchTarget::Application {
                path: title.into(),
                bundle_id: None,
                executable_path: None,
            },
            icon_source: None,
            icon_key: None,
            source_modified_at: None,
            running: QuickLauncherRunningState::NotRunning,
        }
    }

    #[test]
    fn pinyin_and_aliases_are_precomputed() {
        let fields = build_search_fields("微信", bundled_aliases("微信"));
        assert_eq!(fields.pinyin_full, "weixin");
        assert_eq!(fields.pinyin_initials, "wx");
        assert!(fields.aliases.contains(&"wechat".into()));
        assert!(bundled_aliases("Adobe Photoshop").contains(&"ps".into()));
    }

    #[test]
    fn exact_relevance_beats_high_usage_weak_match() {
        let exact = item("Code", vec![]);
        let weak = item("Xcode Helper", vec![]);
        let mut usage = UsageMap::new();
        usage.insert(
            weak.id.clone(),
            UsageEntry {
                count: 50_000,
                last_used_at: unix_timestamp(),
            },
        );
        let mut matcher = default_matcher();
        let result = search_items(
            1,
            &[weak, exact.clone()],
            &usage,
            QuickLauncherSearchInput {
                query: "code".into(),
                limit: Some(10),
            },
            &mut matcher,
        )
        .unwrap();
        assert_eq!(result.items[0].id, exact.id);
    }

    #[test]
    fn pinyin_initials_and_bundled_aliases_match() {
        let wechat = item("微信", bundled_aliases("微信"));
        let photoshop = item("Adobe Photoshop", bundled_aliases("Adobe Photoshop"));
        let mut matcher = default_matcher();
        for (query, expected) in [("wx", &wechat.id), ("ps", &photoshop.id)] {
            let result = search_items(
                1,
                &[wechat.clone(), photoshop.clone()],
                &UsageMap::new(),
                QuickLauncherSearchInput {
                    query: query.into(),
                    limit: Some(10),
                },
                &mut matcher,
            )
            .unwrap();
            assert_eq!(&result.items[0].id, expected);
        }
    }

    #[test]
    fn invalid_limits_and_queries_are_rejected() {
        let mut matcher = default_matcher();
        let error = search_items(
            1,
            &[],
            &UsageMap::new(),
            QuickLauncherSearchInput {
                query: "x".repeat(MAX_QUERY_CHARS + 1),
                limit: Some(10),
            },
            &mut matcher,
        )
        .unwrap_err();
        assert_eq!(error.code, "launcher.query_too_long");

        let error = search_items(
            1,
            &[],
            &UsageMap::new(),
            QuickLauncherSearchInput {
                query: String::new(),
                limit: Some(0),
            },
            &mut matcher,
        )
        .unwrap_err();
        assert_eq!(error.code, "launcher.limit_invalid");
    }

    #[test]
    fn ranking_is_deterministic_and_empty_query_is_bounded() {
        let alpha = item("Alpha", vec![]);
        let beta = item("Beta", vec![]);
        let items = [beta, alpha];
        let mut matcher = default_matcher();
        let first = search_items(
            8,
            &items,
            &UsageMap::new(),
            QuickLauncherSearchInput {
                query: String::new(),
                limit: Some(1),
            },
            &mut matcher,
        )
        .unwrap();
        let second = search_items(
            8,
            &items,
            &UsageMap::new(),
            QuickLauncherSearchInput {
                query: String::new(),
                limit: Some(1),
            },
            &mut matcher,
        )
        .unwrap();
        assert_eq!(first.items, second.items);
        assert_eq!(first.items.len(), 1);
        assert_eq!(first.items[0].title, "Alpha");
    }

    #[test]
    fn romanizer_handles_traditional_and_polyphonic_input_deterministically() {
        let romanizer = PinyinRomanizer;
        let (traditional, initials) = romanizer.romanize("網路");
        assert!(!traditional.is_empty());
        assert_eq!(traditional.chars().count(), traditional.len());
        assert_eq!(initials.chars().count(), 2);

        let first = romanizer.romanize("重庆");
        let second = romanizer.romanize("重庆");
        assert_eq!(first, second);
    }
}
