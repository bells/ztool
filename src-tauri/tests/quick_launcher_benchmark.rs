use std::time::{Duration, Instant};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use zero_lib::services::quick_launcher::contracts::{
    QuickLauncherItemKind, QuickLauncherRunningState, QuickLauncherSearchInput,
};
use zero_lib::services::quick_launcher::model::{
    stable_item_id, IndexedItem, LaunchTarget, UsageMap,
};
use zero_lib::services::quick_launcher::search::{
    build_search_fields, bundled_aliases, default_matcher, search_items, search_items_thread_local,
};

const FIXTURE_SIZE: usize = 10_000;
const SAMPLE_QUERIES: &[&str] = &[
    "code",
    "wx",
    "weixin",
    "ps",
    "display",
    "runner",
    "vsc",
    "utility",
    "系统",
    "photoshop",
];

#[test]
#[ignore = "release performance gate; run with cargo test --release --test quick_launcher_benchmark -- --ignored --nocapture"]
fn compare_matchers_on_mixed_language_fixture() {
    let items = fixture();
    let usage = UsageMap::new();
    let mut nucleo = default_matcher();
    let skim = SkimMatcherV2::default();

    for query in SAMPLE_QUERIES {
        let result = search_items(
            1,
            &items,
            &usage,
            QuickLauncherSearchInput {
                query: (*query).into(),
                limit: Some(24),
            },
            &mut nucleo,
        )
        .expect("fixture query should be valid");
        assert!(!result.items.is_empty(), "{query} should match the fixture");
    }

    let mut nucleo_samples = Vec::new();
    let mut thread_local_samples = Vec::new();
    let mut skim_samples = Vec::new();
    for index in 0..80 {
        let query = SAMPLE_QUERIES[index % SAMPLE_QUERIES.len()];
        let started = Instant::now();
        let _ = search_items(
            1,
            &items,
            &usage,
            QuickLauncherSearchInput {
                query: query.into(),
                limit: Some(24),
            },
            &mut nucleo,
        )
        .unwrap();
        nucleo_samples.push(started.elapsed());

        let started = Instant::now();
        let _ = search_items_thread_local(
            1,
            &items,
            &usage,
            &std::collections::HashMap::new(),
            QuickLauncherSearchInput {
                query: query.into(),
                limit: Some(24),
            },
        )
        .unwrap();
        thread_local_samples.push(started.elapsed());

        let started = Instant::now();
        let mut scores = items
            .iter()
            .filter_map(|item| {
                let best = std::iter::once(item.search.normalized_title.as_str())
                    .chain(std::iter::once(item.search.pinyin_full.as_str()))
                    .chain(std::iter::once(item.search.pinyin_initials.as_str()))
                    .chain(item.search.aliases.iter().map(String::as_str))
                    .filter_map(|field| skim.fuzzy_match(field, query))
                    .max()?;
                Some((best, &item.id))
            })
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| right.cmp(left));
        scores.truncate(24);
        skim_samples.push(started.elapsed());
    }

    let nucleo_p50 = percentile(&mut nucleo_samples, 50);
    let nucleo_p95 = percentile(&mut nucleo_samples, 95);
    let thread_local_p50 = percentile(&mut thread_local_samples, 50);
    let thread_local_p95 = percentile(&mut thread_local_samples, 95);
    let skim_p50 = percentile(&mut skim_samples, 50);
    let skim_p95 = percentile(&mut skim_samples, 95);
    eprintln!(
        "quick-launcher-benchmark fixture={FIXTURE_SIZE} nucleo_p50_us={} nucleo_p95_us={} thread_local_p50_us={} thread_local_p95_us={} fuzzy_matcher_p50_us={} fuzzy_matcher_p95_us={} index_clone_bytes=0 running_probe_count=0",
        nucleo_p50.as_micros(),
        nucleo_p95.as_micros(),
        thread_local_p50.as_micros(),
        thread_local_p95.as_micros(),
        skim_p50.as_micros(),
        skim_p95.as_micros(),
    );
    assert!(
        nucleo_p95 < Duration::from_millis(5),
        "nucleo pure matching p95 must remain below 5ms; measured {nucleo_p95:?}"
    );
    assert!(
        thread_local_p95 < Duration::from_millis(5),
        "thread-local production matching p95 must remain below 5ms; measured {thread_local_p95:?}"
    );
}

fn percentile(samples: &mut [Duration], percentile: usize) -> Duration {
    samples.sort_unstable();
    let index = ((samples.len() - 1) * percentile) / 100;
    samples[index]
}

fn fixture() -> Vec<IndexedItem> {
    (0..FIXTURE_SIZE)
        .map(|index| {
            let title = match index % 5 {
                0 => format!("Visual Studio Code {index}"),
                1 => format!("微信 {index}"),
                2 => format!("Adobe Photoshop {index}"),
                3 => format!("系统设置 Display {index}"),
                _ => format!("Utility Runner {index}"),
            };
            let id = stable_item_id("benchmark", QuickLauncherItemKind::Application, &title);
            IndexedItem {
                id,
                kind: QuickLauncherItemKind::Application,
                title: title.clone(),
                subtitle: format!("/fixture/{title}.app"),
                search: build_search_fields(&title, bundled_aliases(&title)),
                target: LaunchTarget::Application {
                    path: title.clone().into(),
                    bundle_id: None,
                    executable_path: None,
                },
                icon_source: None,
                icon_key: None,
                source_modified_at: None,
                running: QuickLauncherRunningState::Unknown,
            }
        })
        .collect()
}
