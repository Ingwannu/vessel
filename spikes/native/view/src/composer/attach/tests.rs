//! The rules a file is checked by before it is sent, as the page's
//! (`web/src/data/attach.test.ts`) and the node's.

use vessel_proto::{AgentAbilities, AttachmentKind, AttachmentLimits};

use super::*;

fn images_and_audio() -> AgentAbilities {
    AgentAbilities {
        image: true,
        audio: true,
        ..AgentAbilities::default()
    }
}

fn kind(name: &str) -> AttachmentKind {
    kind_of(&mime_of(name).unwrap())
}

fn named(raw: &str) -> String {
    attachment_name(raw, "file").unwrap().as_str().to_owned()
}

#[test]
fn a_files_type_follows_its_extension_as_the_clis_does() {
    assert_eq!(kind("shot.PNG"), AttachmentKind::Image);
    assert_eq!(kind("photo.jpeg"), AttachmentKind::Image);
    assert_eq!(kind("voice.m4a"), AttachmentKind::Audio);
    assert_eq!(kind("notes.md"), AttachmentKind::File);
    assert_eq!(mime_of("Makefile").unwrap().as_str(), UNKNOWN);
    assert_eq!(mime_of("archive.tar.gz").unwrap().as_str(), UNKNOWN);
    for name in [
        "a.png", "a.jpg", "a.gif", "a.webp", "a.mp3", "a.wav", "a.ogg", "a.flac", "a.m4a", "a.txt",
        "a.md", "a.csv", "a.html", "a.json", "a.pdf", "a.zip", "a.bin",
    ] {
        assert!(
            mime_of(name).is_some(),
            "{name}: every type is one the node takes"
        );
    }
}

#[test]
fn a_name_becomes_one_the_node_takes() {
    assert_eq!(named("report.pdf"), "report.pdf");
    assert_eq!(
        named("a/b\\c:d*e?f\"g<h>i|j.txt"),
        "a_b_c_d_e_f_g_h_i_j.txt"
    );
    assert_eq!(named(".env"), "_.env");
    assert_eq!(named("trailing. "), "trailing");
    assert_eq!(named("CON.txt"), "_CON.txt");
    assert_eq!(named("lpt1"), "_lpt1");
    assert_eq!(named("tab\there"), "tab_here");
    assert_eq!(named("..."), "file", "nothing left: the fallback");
    assert_eq!(named("스크린샷 2026-11-14.png"), "스크린샷 2026-11-14.png");
}

#[test]
fn a_long_name_is_cut_on_a_character_keeping_its_extension() {
    let long = format!("{}.png", "가".repeat(200));
    let name = named(&long);
    assert!(name.len() <= 255, "{} bytes", name.len());
    assert!(name.ends_with("가.png"), "{name}");
    let plain = "x".repeat(300);
    assert_eq!(named(&plain).len(), 255);
}

#[test]
fn an_image_must_start_as_its_type_does() {
    let png = mime_of("a.png").unwrap();
    assert!(looks_like(
        &png,
        &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0]
    ));
    assert!(!looks_like(&png, b"GIF89a"));
    let webp = mime_of("a.webp").unwrap();
    assert!(looks_like(&webp, b"RIFF\0\0\0\0WEBPVP8 "));
    assert!(!looks_like(&webp, b"RIFF\0\0\0\0WAVE"));
    assert!(
        looks_like(&mime_of("a.txt").unwrap(), b""),
        "any other type passes"
    );
}

#[test]
fn the_agent_must_take_the_kind() {
    let limits = AttachmentLimits::DEFAULT;
    let image = Candidate {
        kind: AttachmentKind::Image,
        size: 10,
    };
    assert_eq!(
        refusal(image, &[], None, &limits),
        Some(Refusal::Agent(AttachmentKind::Image)),
        "before the agent said what it takes"
    );
    let text_only = AgentAbilities::default();
    assert_eq!(
        refusal(image, &[], Some(&text_only), &limits),
        Some(Refusal::Agent(AttachmentKind::Image))
    );
    assert_eq!(
        refusal(image, &[], Some(&images_and_audio()), &limits),
        None
    );
    let file = Candidate {
        kind: AttachmentKind::File,
        size: 10,
    };
    assert_eq!(refusal(file, &[], None, &limits), None, "files always");
}

#[test]
fn sizes_counts_and_the_inline_total_are_the_nodes() {
    let limits = AttachmentLimits::DEFAULT;
    let abilities = images_and_audio();
    let take = |candidate, held: &[Candidate]| refusal(candidate, held, Some(&abilities), &limits);
    let image = |size| Candidate {
        kind: AttachmentKind::Image,
        size,
    };
    let file = |size| Candidate {
        kind: AttachmentKind::File,
        size,
    };
    assert_eq!(take(file(0), &[]), Some(Refusal::Empty));
    assert_eq!(
        take(image(limits.image.saturating_add(1)), &[]),
        Some(Refusal::TooLarge {
            limit: limits.image
        })
    );
    assert_eq!(take(file(limits.file), &[]), None, "at the limit");
    let full = vec![file(1); 16];
    assert_eq!(take(file(1), &full), Some(Refusal::TooMany { limit: 16 }));
    let images = vec![image(limits.image); 2];
    assert_eq!(
        take(image(limits.image), &images),
        Some(Refusal::InlineTotal {
            limit: limits.inline_per_prompt
        }),
        "three 10 MiB images are past 25 MiB"
    );
    assert_eq!(
        take(file(limits.file), &images),
        None,
        "a file is not carried inline"
    );
}
