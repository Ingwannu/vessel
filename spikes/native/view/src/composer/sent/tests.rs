use std::str::FromStr;
use std::sync::Arc;

use vessel_proto::{AttachmentId, AttachmentKind, AttachmentRef, SessionId};

use super::{Picture, Pictures, Sent};
use crate::composer::attach::{attachment_name, mime_of};

/// The id numbered `n` (a version 4 layout, so the node's parser takes it).
fn id<T: FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012}")
        .parse()
        .expect("a well-formed id")
}

fn sent(n: u64) -> Sent {
    Sent {
        connection: crate::connection::ConnectionEpochs::default()
            .admit(
                crate::connection::MachineKey::from_book("laptop"),
                id::<vessel_proto::NodeId>(2),
            )
            .unwrap(),
        session: id::<SessionId>(1),
        attachment: id::<AttachmentId>(n),
    }
}

fn reference(kind: AttachmentKind, size: u64) -> AttachmentRef {
    AttachmentRef {
        id: id::<AttachmentId>(9),
        name: attachment_name("shot.png", "file").expect("a name the node takes"),
        kind,
        mime: mime_of("shot.png").expect("a known type"),
        size,
    }
}

fn bytes(len: usize) -> Arc<[u8]> {
    Arc::from(vec![0_u8; len])
}

#[test]
fn only_images_a_node_takes_are_pictures() {
    assert!(Pictures::shows(&reference(AttachmentKind::Image, 1024)));
    assert!(!Pictures::shows(&reference(AttachmentKind::File, 1024)));
    assert!(!Pictures::shows(&reference(AttachmentKind::Audio, 1024)));
    assert!(!Pictures::shows(&reference(
        AttachmentKind::Image,
        11 * 1024 * 1024
    )));
}

#[test]
fn an_asked_picture_is_read_once_however_often_it_is_asked() {
    let mut pictures = Pictures::default();
    assert_eq!(pictures.ask(&sent(1)), Picture::Wanted);
    assert_eq!(pictures.ask(&sent(1)), Picture::Wanted);
    assert_eq!(pictures.starts(), vec![sent(1)]);
    assert_eq!(pictures.ask(&sent(1)), Picture::Reading);
    assert!(pictures.starts().is_empty(), "already being read");
    pictures.read(&sent(1), Some(bytes(3)));
    assert_eq!(pictures.ask(&sent(1)), Picture::Ready(bytes(3)));
}

#[test]
fn two_are_read_at_once_the_most_recently_asked_first() {
    let mut pictures = Pictures::default();
    for n in 1..=4 {
        drop(pictures.ask(&sent(n)));
    }
    assert_eq!(pictures.starts(), vec![sent(4), sent(3)]);
    assert!(pictures.starts().is_empty(), "two are running");
    pictures.read(&sent(4), Some(bytes(1)));
    assert_eq!(pictures.starts(), vec![sent(2)]);
}

#[test]
fn a_failed_read_is_not_tried_again_while_kept() {
    let mut pictures = Pictures::default();
    drop(pictures.ask(&sent(1)));
    drop(pictures.starts());
    pictures.read(&sent(1), None);
    assert_eq!(pictures.ask(&sent(1)), Picture::Failed);
    assert!(pictures.starts().is_empty());
}

#[test]
fn an_image_sent_from_here_is_never_read_back() {
    let mut pictures = Pictures::default();
    pictures.seed(sent(1), bytes(5));
    assert_eq!(pictures.ask(&sent(1)), Picture::Ready(bytes(5)));
    assert!(pictures.starts().is_empty());
}

#[test]
fn a_read_for_a_picture_dropped_meanwhile_takes_nothing() {
    let mut pictures = Pictures::default();
    pictures.read(&sent(1), Some(bytes(1)));
    assert!(pictures.is_empty());
}

#[test]
fn at_most_48_are_kept_the_least_recently_asked_dropped() {
    let mut pictures = Pictures::default();
    for n in 1..=48 {
        pictures.seed(sent(n), bytes(1));
    }
    drop(pictures.ask(&sent(1)));
    pictures.seed(sent(49), bytes(1));
    assert_eq!(pictures.len(), Pictures::MOST);
    assert_eq!(
        pictures.ask(&sent(1)),
        Picture::Ready(bytes(1)),
        "asked lately"
    );
    assert_eq!(
        pictures.ask(&sent(2)),
        Picture::Wanted,
        "dropped, asked again"
    );
}

#[test]
fn at_most_64_mib_are_kept() {
    let mut pictures = Pictures::default();
    let ten = 10 * 1024 * 1024;
    for n in 1..=7 {
        pictures.seed(sent(n), bytes(ten));
    }
    assert!(pictures.bytes() <= Pictures::MOST_BYTES);
    assert_eq!(pictures.len(), 6);
    assert_eq!(pictures.ask(&sent(1)), Picture::Wanted, "the oldest went");
}

#[test]
fn a_picture_being_read_is_not_dropped() {
    let mut pictures = Pictures::default();
    drop(pictures.ask(&sent(1)));
    assert_eq!(pictures.starts(), vec![sent(1)]);
    for n in 2..=60 {
        pictures.seed(sent(n), bytes(1));
    }
    assert_eq!(pictures.ask(&sent(1)), Picture::Reading);
}

#[test]
fn a_reused_machine_label_does_not_reuse_a_previous_nodes_image_bytes() {
    let first = sent(1);
    let mut replacement = first.clone();
    replacement.connection = crate::connection::ConnectionEpochs::default()
        .admit(
            crate::connection::MachineKey::from_book("laptop"),
            id::<vessel_proto::NodeId>(3),
        )
        .unwrap();
    let mut pictures = Pictures::default();
    pictures.seed(first.clone(), bytes(3));
    assert!(matches!(pictures.ask(&first), Picture::Ready(_)));
    assert_eq!(pictures.ask(&replacement), Picture::Wanted);
    assert_ne!(first, replacement);
}
