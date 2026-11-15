//! One composer's files on their way: checked as they come, uploaded two
//! at a time when Send is pressed, the prompt held until all are in (the
//! page's `web/src/data/tray.test.ts`).

use std::path::PathBuf;

use vessel_proto::{AgentAbilities, AttachmentId, AttachmentLimits, AttachmentRef, UploadId};

use super::*;

fn file(name: &str) -> Origin {
    Origin::File(PathBuf::from("/tmp").join(name))
}

fn target(image: bool) -> Target {
    Target {
        abilities: Some(AgentAbilities {
            image,
            ..AgentAbilities::default()
        }),
        limits: AttachmentLimits::DEFAULT,
    }
}

fn reference(item: &Item) -> AttachmentRef {
    AttachmentRef {
        id: AttachmentId::random(),
        name: item.name.clone(),
        kind: item.kind,
        mime: item.mime.clone(),
        size: item.size,
    }
}

fn states(tray: &Tray) -> Vec<ItemState> {
    tray.items().iter().map(|item| item.state.clone()).collect()
}

#[test]
fn files_wait_unchecked_until_there_is_a_session_then_are_checked() {
    let mut tray = Tray::default();
    let shot = tray.add("shot.png", 100, file("shot.png")).unwrap();
    let notes = tray.add("notes.md", 100, file("notes.md")).unwrap();
    assert_eq!(states(&tray), [ItemState::Held, ItemState::Held]);
    assert_eq!(tray.readiness(), Readiness::Uploading, "not ready to send");
    tray.target(Some(target(false)));
    assert_eq!(
        states(&tray),
        [
            ItemState::Refused(Refusal::Agent(AttachmentKind::Image)),
            ItemState::Attached
        ]
    );
    assert_eq!(tray.readiness(), Readiness::Blocked(shot));
    // The agent says it takes images after all: looked at again.
    tray.target(Some(target(true)));
    assert_eq!(states(&tray), [ItemState::Attached, ItemState::Attached]);
    assert!(tray.remove(notes));
    assert_eq!(tray.items().len(), 1);
}

#[test]
fn a_pasted_image_without_a_name_is_an_image() {
    let mut tray = Tray::default();
    let _key = tray
        .add("", 10, Origin::Pasted(Arc::from(vec![0_u8; 10])))
        .unwrap();
    let item = &tray.items()[0];
    assert_eq!(item.name.as_str(), "image.png");
    assert_eq!(item.kind, AttachmentKind::Image);
}

#[test]
fn send_uploads_two_at_a_time_and_the_prompt_waits_for_all() {
    let mut tray = Tray::default();
    tray.target(Some(target(true)));
    for name in ["a.png", "b.png", "c.png"] {
        let _key = tray.add(name, 100, file(name));
    }
    assert!(tray.starts().is_empty(), "nothing before Send");
    tray.send();
    let first = tray.starts();
    assert_eq!(first.len(), AT_ONCE);
    assert!(tray.starts().is_empty(), "no third while two run");
    let (done, _) = &first[0];
    tray.progress(done.key, 50);
    assert_eq!(tray.items()[0].state, ItemState::Uploading(50));
    tray.uploaded(done.key, reference(done));
    let second = tray.starts();
    assert_eq!(second.len(), 1, "the third takes the freed place");
    assert_eq!(tray.readiness(), Readiness::Uploading);
    for (item, _) in first.iter().skip(1).chain(&second) {
        tray.uploaded(item.key, reference(item));
    }
    let Readiness::Ready(references) = tray.readiness() else {
        panic!("all in");
    };
    let names: Vec<&str> = references.iter().map(|file| file.name.as_str()).collect();
    assert_eq!(names, ["a.png", "b.png", "c.png"], "in the tray's order");
    tray.clear();
    assert!(tray.is_empty());
}

#[test]
fn a_failed_upload_blocks_send_until_retried_from_what_the_node_has() {
    let mut tray = Tray::default();
    tray.target(Some(target(true)));
    let key = tray.add("big.zip", 3 << 20, file("big.zip")).unwrap();
    tray.send();
    let _started = tray.starts();
    let begun = Begun {
        upload: UploadId::random(),
        chunk: 1 << 20,
        received: 2 << 20,
    };
    tray.failed(
        key,
        Problem::Node("connection lost".to_owned()),
        Again::Resume(begun),
    );
    assert_eq!(tray.readiness(), Readiness::Blocked(key));
    assert!(tray.retry(key));
    let again = tray.starts();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].1, Some(begun), "goes on from the node's 2 MiB");
    assert_eq!(tray.items()[0].state, ItemState::Uploading(2 << 20));
}

#[test]
fn a_file_that_will_never_go_cannot_be_retried() {
    let mut tray = Tray::default();
    tray.target(Some(target(true)));
    let key = tray.add("fake.png", 10, file("fake.png")).unwrap();
    tray.send();
    let _started = tray.starts();
    tray.failed(key, Problem::NotItsType, Again::Never);
    assert!(!tray.retry(key));
    assert_eq!(tray.readiness(), Readiness::Blocked(key));
}

#[test]
fn the_count_and_inline_total_are_checked_against_the_files_already_kept() {
    let mut tray = Tray::default();
    tray.target(Some(target(true)));
    let limits = AttachmentLimits::DEFAULT;
    for name in ["a.png", "b.png", "c.png"] {
        let _key = tray.add(name, limits.image, file(name));
    }
    assert_eq!(
        states(&tray)[2],
        ItemState::Refused(Refusal::InlineTotal {
            limit: limits.inline_per_prompt
        })
    );
    // Removing one makes room: the refused one is looked at again.
    let first = tray.items()[0].key;
    assert!(tray.remove(first));
    assert_eq!(tray.items()[1].state, ItemState::Attached);
}
