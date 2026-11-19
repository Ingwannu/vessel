//! The page's `markdown-paths.test.ts` cases.

use super::*;

fn rel(path: &str) -> RelPath {
    RelPath::from_str(path).expect("a valid relative path")
}

fn file(path: &str) -> Target {
    Target::File(rel(path))
}

#[test]
fn a_relative_target_resolves_from_the_markdown_files_folder() {
    assert_eq!(
        target("./images/a.png", &rel("docs/guide.md"), None),
        file("docs/images/a.png")
    );
    assert_eq!(
        target("../logo.svg", &rel("docs/guide.md"), None),
        file("logo.svg")
    );
    assert_eq!(
        target("notes.md#install", &rel("README.md"), None),
        file("notes.md")
    );
}

#[test]
fn an_encoded_name_is_decoded_and_a_query_ignored() {
    assert_eq!(
        target("my%20shot.png?raw=1", &rel("a/b.md"), None),
        file("a/my shot.png")
    );
    // A malformed escape is kept as written, as Paseo keeps it.
    assert_eq!(target("bad%zz.png", &rel("a.md"), None), file("bad%zz.png"));
}

#[test]
fn an_absolute_target_inside_the_root_is_kept_by_the_roots_folder() {
    let from = rel("README.md");
    assert_eq!(
        target("/work/app/assets/a.png", &from, Some("/work/app")),
        file("assets/a.png")
    );
    assert_eq!(
        target("file:///work/app/a.png", &from, Some("/work/app/")),
        file("a.png")
    );
    assert_eq!(
        target("/work/other/a.png", &from, Some("/work/app")),
        Target::None
    );
    assert_eq!(target("/work/app/a.png", &from, None), Target::None);
    assert_eq!(
        target("C:\\work\\app\\a.png", &from, Some("C:\\work\\app")),
        file("a.png")
    );
}

#[test]
fn a_target_leaving_the_root_is_refused() {
    assert_eq!(target("../../x.png", &rel("docs/a.md"), None), Target::None);
    assert_eq!(target("~/x.png", &rel("a.md"), None), Target::None);
}

#[test]
fn web_and_data_addresses_come_back_and_nothing_else() {
    let from = rel("a.md");
    assert_eq!(
        target("https://example.com/a.png", &from, None),
        Target::Web("https://example.com/a.png".to_owned())
    );
    assert_eq!(
        target("data:image/png;base64,AA==", &from, None),
        Target::Data("data:image/png;base64,AA==".to_owned())
    );
    for source in ["#top", "", "mailto:a@example.com", "blob:x"] {
        assert_eq!(target(source, &from, None), Target::None, "{source}");
    }
}
