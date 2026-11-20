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
    let levels: Vec<String> = (1..=20_u32)
        .map(|level| {
            let below = level.saturating_sub(1);
            format!("l{level}: &l{level} [*l{below}, *l{below}]")
        })
        .collect();
    let source = format!("---\nl0: &l0 [x, x]\n{}\n---\nBody\n", levels.join("\n"));
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

#[test]
fn a_block_nested_past_the_depth_budget_stays_in_the_body() {
    // About 10 KB that the loader would build 5,000 levels deep. The
    // reader runs on a quarter of Windows' main stack, so a descent per
    // level would overflow there.
    let depth = 5_000;
    let source = format!(
        "---\nkey: {}{}\n---\n# Body\n",
        "[".repeat(depth),
        "]".repeat(depth)
    );
    let read = {
        let source = source.clone();
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || markdown_document(&source))
            .expect("a thread")
            .join()
            .expect("read without overflowing")
    };
    assert_eq!(read, intact(&source));
}

#[test]
fn a_block_at_the_depth_budget_is_read() {
    let source = format!(
        "---\nkey: {}1{}\n---\nbody\n",
        "[".repeat(MAX_DEPTH.saturating_sub(1)),
        "]".repeat(MAX_DEPTH.saturating_sub(1))
    );
    let document = markdown_document(&source);
    assert_eq!(document.body, "body\n");
    assert_eq!(document.front_matter.len(), 1);
}

#[test]
fn aliases_that_nest_past_the_depth_budget_stay_in_the_body() {
    // The app review's case: each anchor 62 levels inside the map holds
    // the one before. Under the node budget, it is about 1,000 levels deep
    // once the loader copies the aliases.
    let yaml: String = (0..17_usize)
        .map(|level| {
            let inner = level
                .checked_sub(1)
                .map_or_else(|| "1".to_owned(), |before| format!("*a{before}"));
            format!(
                "k{level}: &a{level} {}{inner}{}\n",
                "[".repeat(62),
                "]".repeat(62)
            )
        })
        .collect();
    let source = format!("---\n{yaml}---\n# Body\n");
    let read = {
        let source = source.clone();
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || markdown_document(&source))
            .expect("a thread")
            .join()
            .expect("read without overflowing")
    };
    assert_eq!(read, intact(&source));
}

#[test]
fn a_shallow_alias_within_the_depth_budget_is_read() {
    let source = "---\nbase: &base {name: one}\ncopy: *base\n---\nbody\n";
    let document = markdown_document(source);
    assert_eq!(document.body, "body\n");
    assert_eq!(document.front_matter.len(), 2);
}
