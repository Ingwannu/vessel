//! The browser link answers every command (the coordinator's ask when
//! `web.rs`'s `answer` was split by kind of request, ADR 0114's row).
//!
//! `web.rs` builds only for wasm32, where these tests do not run, and the
//! compiler already refuses a `match` that misses a command. Two things it
//! cannot see:
//! - a catch-all arm, which would take a new command silently;
//! - a command that loses its arm when the dispatch is moved around.
//!
//! So this reads the sources: every variant of `Command`, `Files` and
//! mentions' `Ask` is named in the browser's dispatch, and no arm there
//! is a bare `_` or binding.

const COMMAND: &str = include_str!("command.rs");
const FILES: &str = include_str!("attachments.rs");
const MENTIONS: &str = include_str!("mentions.rs");
const WEB: &str = include_str!("web.rs");
const REFUSE: &str = include_str!("web/refuse.rs");

/// The variants of `pub enum NAME {` in `source`: the lines indented once
/// that start with a capital (fields are indented twice; comments start
/// with `/`).
fn variants(source: &str, name: &str) -> Vec<String> {
    let head = format!("pub enum {name} {{\n");
    let (_, rest) = source.split_once(&head).expect("the enum is declared");
    let (body, _) = rest.split_once("\n}\n").expect("its end");
    let found: Vec<String> = body
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix("    ")?;
            if !line.starts_with(|c: char| c.is_ascii_uppercase()) {
                return None;
            }
            line.split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .map(str::to_owned)
        })
        .collect();
    assert!(!found.is_empty(), "{name} has variants");
    found
}

/// Whether `source` names `path::variant` as a whole word.
fn names(source: &str, path: &str, variant: &str) -> bool {
    let needle = format!("{path}::{variant}");
    source.match_indices(&needle).any(|(at, _)| {
        source
            .get(at..)
            .and_then(|from| from.strip_prefix(needle.as_str()))
            .and_then(|after| after.chars().next())
            .is_none_or(|next| !next.is_ascii_alphanumeric() && next != '_')
    })
}

#[test]
fn every_command_has_its_own_arm_in_the_browser_link() {
    for variant in variants(COMMAND, "Command") {
        assert!(names(WEB, "Command", &variant), "Command::{variant}");
    }
    for variant in variants(FILES, "Files") {
        assert!(names(WEB, "Files", &variant), "Files::{variant}");
    }
    for variant in variants(MENTIONS, "Ask") {
        assert!(
            names(REFUSE, "crate::mentions::Ask", &variant),
            "Ask::{variant}"
        );
    }
}

#[test]
fn the_browser_link_has_no_catch_all_arm() {
    for (file, source) in [("web.rs", WEB), ("web/refuse.rs", REFUSE)] {
        for line in source.lines() {
            let arm = line.trim_start();
            let Some(pattern) = arm.split(" =>").next().filter(|_| arm.contains(" =>")) else {
                continue;
            };
            let bare =
                pattern == "_" || pattern.chars().all(|c| c.is_ascii_lowercase() || c == '_');
            assert!(!bare, "{file}: a catch-all arm `{arm}`");
        }
    }
}

#[test]
fn the_reader_finds_what_it_reads() {
    let sample = concat!(
        "pub enum Thing {\n",
        "    /// One.\n",
        "    One(u8),\n",
        "    Two {\n",
        "        field: u8,\n",
        "    },\n",
        "    // A note.\n",
        "    Three,\n",
        "}\n",
    );
    assert_eq!(variants(sample, "Thing"), ["One", "Two", "Three"]);
    assert!(names("Thing::One =>", "Thing", "One"));
    assert!(!names("Thing::OneMore =>", "Thing", "One"));
}
