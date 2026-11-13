//! `web/src/data/archive.test.ts`, "searchArchive"; and the archive's
//! pages (`features/board/archive.tsx`: 30 at a time).

use vessel_proto::SessionStatus;

use super::fixture::{fleet, in_folder, machine, session, short};
use crate::board::{ARCHIVE_PAGE, ArchivePage, Row, search_archive};
use crate::{Card, Fleet};

fn state(title: Option<&str>) -> Fleet {
    let exited = SessionStatus::Exited { code: Some(0) };
    let mut docs = in_folder(
        session("9aa000", "2026-11-07T10:00:00Z", exited.clone()),
        "/w/docs",
    );
    docs.title = title.map(|title| title.parse().unwrap());
    fleet(vec![
        machine(
            "gpu-box",
            true,
            &[in_folder(
                session("3f2a99", "2026-11-07T10:00:00Z", exited),
                "/w/Vessel",
            )],
        ),
        machine("laptop", true, &[docs]),
    ])
}

fn rows(fleet: &Fleet) -> Vec<Row<'_>> {
    fleet
        .machines
        .iter()
        .flat_map(|machine| &machine.sessions)
        .map(|card: &Card| Row {
            card,
            depth: 0,
            parent: None,
        })
        .collect()
}

#[test]
fn keeps_everything_for_an_empty_query() {
    let fleet = state(None);
    assert_eq!(search_archive(&rows(&fleet), "  ").len(), 2);
}

#[test]
fn matches_an_ids_start_a_folder_or_a_nodes_name_in_any_case() {
    let fleet = state(None);
    let all = rows(&fleet);
    let ids = |query: &str| -> Vec<String> {
        search_archive(&all, query)
            .iter()
            .map(|row| short(row.card, 6))
            .collect()
    };
    assert_eq!(ids("3f2a"), ["3f2a99"]);
    assert!(ids("2a99").is_empty());
    assert_eq!(ids("vessel"), ["3f2a99"]);
    assert_eq!(ids("GPU-BOX"), ["3f2a99"]);
    assert_eq!(ids("laptop"), ["9aa000"]);
}

#[test]
fn matches_a_piece_of_a_title() {
    let fleet = state(Some("Release notes"));
    let found: Vec<String> = search_archive(&rows(&fleet), "release")
        .iter()
        .map(|row| short(row.card, 6))
        .collect();
    assert_eq!(found, ["9aa000"]);
}

#[test]
fn shows_thirty_at_a_time() {
    let page = ArchivePage::default();
    assert_eq!(page.visible(12), 12);
    assert_eq!(page.more(12), None);
    assert_eq!(page.visible(75), ARCHIVE_PAGE);
    assert_eq!(page.more(75), Some(30));
    let page = page.next();
    assert_eq!(page.visible(75), 60);
    assert_eq!(page.more(75), Some(15));
    assert_eq!(page.next().more(75), None);
}
