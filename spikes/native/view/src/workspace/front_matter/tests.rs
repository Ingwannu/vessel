//! Paseo's `markdown-preview/document.test.ts` cases (the page's
//! `front-matter.test.ts`), and the core schema's scalars as the page's
//! table writes them.

use super::*;

fn row(key: &str, value: &str) -> FrontMatterRow {
    FrontMatterRow {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn intact(source: &str) -> MarkdownDocument {
    MarkdownDocument {
        front_matter: Vec::new(),
        body: source.to_owned(),
    }
}

#[test]
fn yaml_front_matter_is_separated_from_the_markdown_body() {
    let source = "---\nname: release-beta\ndescription: Cut a beta release\nallowed-tools:\n  - Bash\n  - Read\n---\n# Release beta\n\nShip it.\n";
    assert_eq!(
        markdown_document(source),
        MarkdownDocument {
            front_matter: vec![
                row("name", "release-beta"),
                row("description", "Cut a beta release"),
                row("allowed-tools", "Bash, Read"),
            ],
            body: "# Release beta\n\nShip it.\n".to_owned(),
        }
    );
}

#[test]
fn nested_metadata_is_written_without_flattening_its_meaning() {
    let source = "---\nmetadata:\n  audience: maintainers\n  stable: true\n---\nBody\n";
    assert_eq!(
        markdown_document(source),
        MarkdownDocument {
            front_matter: vec![row("metadata", "audience: maintainers\nstable: true")],
            body: "Body\n".to_owned(),
        }
    );
}

#[test]
fn invalid_front_matter_leaves_the_document_intact() {
    let source = "---\nname: [broken\n---\nBody\n";
    assert_eq!(markdown_document(source), intact(source));
}

#[test]
fn excessive_aliases_leave_the_document_intact() {
    let aliases = vec!["*item"; 101].join(", ");
    let source = format!("---\nitem: &item value\nitems: [{aliases}]\n---\nBody\n");
    assert_eq!(markdown_document(&source), intact(&source));
}

#[test]
fn nested_aliases_past_the_size_budget_leave_the_document_intact() {
    // Twenty levels, each two aliases of the last: 40 aliases, 2^20 nodes.
    let mut source = String::from("---\nl0: &l0 [x, x]\n");
    for level in 1..=20 {
        let below = level - 1;
        source.push_str(&format!("l{level}: &l{level} [*l{below}, *l{below}]\n"));
    }
    source.push_str("---\nBody\n");
    assert_eq!(markdown_document(&source), intact(&source));
}

#[test]
fn a_thematic_break_inside_the_document_is_not_front_matter() {
    let source = "Introduction\n\n---\n\nBody\n";
    assert_eq!(markdown_document(source), intact(source));
}

#[test]
fn core_schema_scalars_are_written_as_the_page_writes_them() {
    let source = "---\non: true\noff: false\nnone: null\ntilde: ~\ncount: 42\nhex: 0x2A\nratio: 1.5\nwhole: 2.0\nquoted: \"true\"\nmixed: [1, true, null, x]\n---\n";
    assert_eq!(
        markdown_document(source).front_matter,
        vec![
            row("on", "true"),
            row("off", "false"),
            row("none", "null"),
            row("tilde", "null"),
            row("count", "42"),
            row("hex", "42"),
            row("ratio", "1.5"),
            row("whole", "2"),
            row("quoted", "true"),
            row("mixed", "1, true, null, x"),
        ]
    );
}

#[test]
fn a_bom_and_crlf_lines_are_read_as_the_page_reads_them() {
    let source = "\u{feff}---\r\ntitle: Notes\r\n---\r\nBody\r\n";
    assert_eq!(
        markdown_document(source),
        MarkdownDocument {
            front_matter: vec![row("title", "Notes")],
            body: "Body\r\n".to_owned(),
        }
    );
}
