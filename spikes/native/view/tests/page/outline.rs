//! The page's `data/outline.test.ts`.

use vessel_app_view::timeline::{OutlineItem, outline, preview};

use crate::support::make::{message, prompt};

#[test]
fn one_item_per_prompt_with_the_turns_last_reply() {
    assert_eq!(
        outline(&[
            prompt(1, "Fix  the\nbuild"),
            message(2, "Looking."),
            message(3, "**Done**: it builds."),
            prompt(4, "Now the tests"),
        ]),
        [
            OutlineItem {
                entry: 0,
                prompt: "Fix the build".to_owned(),
                reply: Some("Done: it builds.".to_owned())
            },
            OutlineItem {
                entry: 3,
                prompt: "Now the tests".to_owned(),
                reply: None
            },
        ]
    );
}

#[test]
fn a_long_prompt_is_cut_with_an_ellipsis() {
    let cut = preview(&"x".repeat(200), 120);
    assert_eq!(cut.chars().count(), 120);
    assert!(cut.ends_with('…'));
    assert_eq!(preview("short", 120), "short");
}

#[test]
fn an_unchanged_outline_is_the_same() {
    let entries = vec![
        prompt(1, "Fix the build"),
        message(9, "Looking."),
        message(2, "**Done**: it builds."),
    ];
    let first = outline(&entries);
    let mut longer = entries.clone();
    longer.push(message(3, "**Done**: it builds."));
    let again = outline(&longer);
    assert_eq!(first, outline(&entries));
    assert_eq!(again[0].reply.as_deref(), Some("Done: it builds."));
    assert_eq!(first, again);
    let mut more = entries.clone();
    more.push(prompt(4, "More"));
    assert_ne!(first, outline(&more));
    assert_ne!(
        first,
        outline(&[prompt(1, "Fix the build"), message(5, "No.")])
    );
}
