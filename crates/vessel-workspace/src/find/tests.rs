//! Ranking, highlighting, cancellation and the cache of indexes.

use std::sync::Arc;
use std::time::Instant;

use super::*;
use crate::paths::Root;
use crate::testing::Sandbox;
use tokio_util::sync::CancellationToken;

fn index(paths: &[&str]) -> NameIndex {
    NameIndex::from_paths(
        paths.iter().map(|path| path.parse().unwrap()).collect(),
        true,
    )
}

fn found(index: &NameIndex, query: &str) -> Vec<String> {
    index
        .search(query, None, &CancellationToken::new())
        .matches
        .into_iter()
        .map(|found| found.path.to_string())
        .collect()
}

#[test]
fn the_file_name_and_word_starts_rank_first() {
    let paths = index(&[
        "docs/reference/main-notes.txt",
        "src/main.rs",
        "crates/vessel-node/src/main.rs",
        "src/domain/ai/nested.rs",
        "README.md",
    ]);
    let ranked = found(&paths, "main");
    assert_eq!(ranked.first().map(String::as_str), Some("src/main.rs"));
    assert!(ranked.contains(&"crates/vessel-node/src/main.rs".to_owned()));
    assert!(!ranked.contains(&"README.md".to_owned()));
    // Across the path when the name alone does not hold it.
    assert_eq!(
        found(&paths, "vnmain").first().map(String::as_str),
        Some("crates/vessel-node/src/main.rs")
    );
    assert_eq!(found(&paths, "READ"), ["README.md"], "case does not matter");
    assert!(found(&paths, "zzz").is_empty());
}

#[test]
fn ascii_and_unicode_paths_rank_by_the_same_rules() {
    let needle: Vec<char> = "mrs".chars().collect();
    for path in ["src/main.rs", "crates/MainRunner/src/lib.rs", "a/mr_s.txt"] {
        let lower = path.to_lowercase();
        let start = lower.rfind('/').map_or(0, |slash| slash + 1);
        assert_eq!(
            score_ascii(b"mrs", lower.as_bytes(), path.as_bytes(), start),
            score(&needle, &lower, path),
            "{path}"
        );
    }
    let paths = index(&["문서/메인.md", "docs/main.md"]);
    assert_eq!(found(&paths, "메인"), ["문서/메인.md"]);
}

#[test]
fn positions_point_at_the_matched_characters() {
    let paths = index(&["src/main.rs"]);
    let result = paths.search("mrs", None, &CancellationToken::new());
    let first = &result.matches[0];
    let marked: String = first
        .positions
        .iter()
        .map(|at| {
            first
                .path
                .as_str()
                .chars()
                .nth(usize::try_from(*at).unwrap())
                .unwrap()
        })
        .collect();
    assert_eq!(marked, "mrs");
}

#[test]
fn a_search_keeps_its_limit_and_stops_when_cancelled() {
    let many: Vec<String> = (0..10_000).map(|n| format!("dir/file{n}.txt")).collect();
    let paths = NameIndex::from_paths(
        many.iter().map(|path| path.parse().unwrap()).collect(),
        true,
    );
    let result = paths.search("file", Some(7), &CancellationToken::new());
    assert_eq!(result.matches.len(), 7);
    assert_eq!(result.indexed, 10_000);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let stopped = paths.search("file", None, &cancel);
    assert!(stopped.superseded);
    assert!(stopped.matches.is_empty());
}

#[tokio::test]
async fn indexes_come_from_git_or_a_walk_and_are_kept() {
    let sandbox = Sandbox::new();
    let repo = sandbox.repo("r");
    Sandbox::write(&repo, ".gitignore", "target/\n");
    Sandbox::write(&repo, "target/huge.o", "x");
    Sandbox::write(&repo, "src/lib.rs", "x");
    let git = sandbox.git().await;
    let indexes = Arc::new(Indexes::default());
    let root = Root::open(&repo).unwrap();
    let first = indexes.get(Some(&git), &root).await;
    assert!(
        found(&first, "huge").is_empty(),
        "ignored files are not indexed"
    );
    assert_eq!(found(&first, "lib"), ["src/lib.rs"]);
    let again = indexes.get(Some(&git), &root).await;
    assert!(Arc::ptr_eq(&first, &again), "kept while fresh");

    let folder = sandbox.path().join("plain");
    Sandbox::write(&folder, "a/b/c.txt", "x");
    std::fs::create_dir_all(folder.join(".git")).unwrap();
    std::fs::write(folder.join(".git/HEAD"), "x").unwrap();
    let plain = indexes.get(None, &Root::open(&folder).unwrap()).await;
    assert_eq!(found(&plain, "c.txt"), ["a/b/c.txt"]);
    assert!(
        found(&plain, "HEAD").is_empty(),
        "a `.git` folder is not walked"
    );
}

/// Measures a search on a large repository: `VESSEL_FIND_REPO=<path>
/// cargo test -p vessel-workspace --release -- --ignored measure`.
#[tokio::test]
#[ignore = "a measurement on a repository of the machine's, not a check"]
async fn measure_a_large_repository() {
    let Some(repo) = std::env::var_os("VESSEL_FIND_REPO") else {
        return;
    };
    let sandbox = Sandbox::new();
    let git = sandbox.git().await;
    let root = Root::open(Path::new(&repo)).unwrap();
    let started = Instant::now();
    let built = NameIndex::build(Some(&git), &root).await;
    let build_time = started.elapsed();
    let mut timings = Vec::new();
    for query in ["m", "ma", "mai", "main", "mainrs", "src/main", "zzzzqqq"] {
        let started = Instant::now();
        let result = built.search(query, Some(50), &CancellationToken::new());
        timings.push((query, started.elapsed(), result.matches.len()));
    }
    let report = format!(
        "{} paths, built in {build_time:?}; searches: {timings:?}",
        built.len()
    );
    assert!(!built.is_empty(), "{report}");
    // Shown on failure only; run with a failing bound to read it.
    assert!(build_time.as_secs() < 60, "{report}");
    tracing::info!("{report}");
    std::fs::write(std::env::temp_dir().join("vessel-find-measure.txt"), report).unwrap();
}
