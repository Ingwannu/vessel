//! Canonical tool bodies become bounded rendering instructions without provider
//! name parsing or I/O. Setup and sub-agent markers deliberately have no body.

use std::borrow::Cow;
use std::path::Path;

use vessel_proto::RawJson;
use vessel_proto::timeline::{DetailOffset, ToolCallDetail};

/// Paseo read details start at line one when the provider supplied no offset.
const DEFAULT_READ_OFFSET: DetailOffset = DetailOffset::new(1);

/// Unknown tools retain the input/result distinction in the person's language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSectionLabel {
    /// Arguments supplied to the tool.
    Input,
    /// Result returned by the tool.
    Output,
}

/// The screen renders these instructions through existing Markdown/output controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSection<'a> {
    /// One console block combines command and output as in Paseo.
    Terminal {
        /// Console command without trailing blank lines.
        command: &'a str,
        /// Observed output, without leading blank lines.
        output: Option<&'a str>,
    },
    /// A file's content may carry a reported first line for read numbering.
    Code {
        /// Exact file content.
        text: &'a str,
        /// Syntax selection is derived from a reported path, never client I/O.
        path: &'a Path,
        /// Reads have line numbers; whole-file write previews need none.
        first_line: Option<DetailOffset>,
    },
    /// Unified provider text takes precedence over reconstructed file contents.
    Patch {
        /// The provider's unified patch.
        text: &'a str,
        /// The affected file.
        path: &'a Path,
    },
    /// A reported replacement is diffed without inferring absent old contents.
    Edit {
        /// Absence remains unknown rather than invented original content.
        old: Option<&'a str>,
        /// Explicitly reported replacement; empty is a real deletion.
        new: &'a str,
        /// The affected file.
        path: &'a Path,
    },
    /// Search/fetch output keeps the source's code-style text surface.
    Text(Cow<'a, str>),
    /// Literal readable prose uses the UI face and shared selectable text engine.
    Prose(Cow<'a, str>),
    /// Arbitrary tool data stays opaque until the existing JSON text decoder.
    Opaque {
        /// Distinguish arguments from the result.
        label: ToolSectionLabel,
        /// Core presentation only formats it, without reading arbitrary fields.
        raw: &'a RawJson,
    },
}

/// The canonical body vocabulary is closed; a new kind must choose a renderer.
#[must_use]
pub fn tool_sections(detail: &ToolCallDetail) -> Vec<ToolSection<'_>> {
    match detail {
        ToolCallDetail::Shell(shell) => vec![ToolSection::Terminal {
            command: shell.command.trim_end_matches('\n'),
            output: shell
                .output
                .as_deref()
                .map(|text| text.trim_start_matches('\n')),
        }],
        ToolCallDetail::Read(read) => read
            .content
            .as_deref()
            .map(|text| ToolSection::Code {
                text,
                path: &read.file_path,
                first_line: Some(read.offset.unwrap_or(DEFAULT_READ_OFFSET)),
            })
            .into_iter()
            .collect(),
        ToolCallDetail::Write(write) => write
            .content
            .as_deref()
            .map(|text| ToolSection::Code {
                text,
                path: &write.file_path,
                first_line: None,
            })
            .into_iter()
            .collect(),
        ToolCallDetail::Edit(edit) => {
            if let Some(text) = &edit.unified_diff {
                vec![ToolSection::Patch {
                    text,
                    path: &edit.file_path,
                }]
            } else {
                edit.new_string
                    .as_deref()
                    .map(|new| ToolSection::Edit {
                        old: edit.old_string.as_deref(),
                        new,
                        path: &edit.file_path,
                    })
                    .into_iter()
                    .collect()
            }
        }
        ToolCallDetail::Search(search) => search_sections(search),
        ToolCallDetail::Fetch(fetch) => vec![ToolSection::Text(Cow::Owned(
            fetch
                .result
                .as_deref()
                .filter(|text| !text.is_empty())
                .map_or_else(
                    || fetch.url.to_string(),
                    |result| format!("{}\n\n{result}", fetch.url),
                ),
        ))],
        ToolCallDetail::PlainText(plain) => plain
            .text
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(|text| ToolSection::Prose(Cow::Borrowed(text)))
            .into_iter()
            .collect(),
        ToolCallDetail::Unknown(unknown) => unknown_sections(unknown),
        // Plan's separate card also owns its decision title and optional approval footer.
        ToolCallDetail::Plan(_)
        | ToolCallDetail::WorktreeSetup(_)
        | ToolCallDetail::SubAgent(_) => Vec::new(),
    }
}

/// Formatting edge only: recognize a top-level JSON string/null without reading
/// provider fields. Strings without an output are prose in Paseo's unknown body.
fn unknown_sections(unknown: &vessel_proto::timeline::UnknownToolDetail) -> Vec<ToolSection<'_>> {
    let output_absent = unknown
        .output
        .as_ref()
        .is_none_or(|raw| serde_json::from_str::<()>(raw.as_text()).is_ok());
    if output_absent
        && let Some(text) = unknown
            .input
            .as_ref()
            .and_then(|raw| serde_json::from_str::<String>(raw.as_text()).ok())
    {
        return vec![ToolSection::Prose(Cow::Owned(text))];
    }
    [
        (ToolSectionLabel::Input, unknown.input.as_ref()),
        (ToolSectionLabel::Output, unknown.output.as_ref()),
    ]
    .into_iter()
    .filter_map(|(label, raw)| {
        raw.filter(|raw| super::tool_display::opaque_present(Some(raw)))
            .map(|raw| ToolSection::Opaque { label, raw })
    })
    .collect()
}

fn search_sections(search: &vessel_proto::timeline::SearchDetail) -> Vec<ToolSection<'_>> {
    let mut sections = Vec::new();
    if let Some(text) = &search.content {
        sections.push(ToolSection::Text(Cow::Borrowed(text)));
    }
    if let Some(paths) = &search.file_paths {
        sections.push(ToolSection::Text(Cow::Owned(
            paths
                .iter()
                .map(|path| path.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n"),
        )));
    }
    if let Some(results) = &search.web_results {
        sections.push(ToolSection::Text(Cow::Owned(
            results
                .iter()
                .map(|result| format!("{}\n{}", result.title, result.url))
                .collect::<Vec<_>>()
                .join("\n\n"),
        )));
    }
    if let Some(notes) = &search.annotations {
        sections.push(ToolSection::Text(Cow::Owned(notes.join("\n"))));
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn detail(value: serde_json::Value) -> ToolCallDetail {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn reads_preserve_reported_offsets_including_zero_and_writes_stay_unnumbered() {
        let read = detail(json!({"type":"read","file_path":"/a.rs","content":"one\n","offset":0}));
        assert!(
            matches!(tool_sections(&read).as_slice(), [ToolSection::Code { first_line:Some(offset), .. }] if offset.get() == 0)
        );
        let read = detail(json!({"type":"read","file_path":"/a.rs","content":"one\n"}));
        assert!(
            matches!(tool_sections(&read).as_slice(), [ToolSection::Code { first_line:Some(offset), .. }] if offset.get() == 1)
        );
        let write = detail(json!({"type":"write","file_path":"/a.rs","content":"one\n"}));
        assert!(matches!(
            tool_sections(&write).as_slice(),
            [ToolSection::Code {
                first_line: None,
                ..
            }]
        ));
    }

    #[test]
    fn absent_replacement_never_becomes_deletion_and_reported_unified_diff_wins() {
        let missing = detail(json!({"type":"edit","file_path":"a","old_string":"old"}));
        assert!(tool_sections(&missing).is_empty());
        let deletion =
            detail(json!({"type":"edit","file_path":"a","old_string":"old","new_string":""}));
        assert!(matches!(
            tool_sections(&deletion).as_slice(),
            [ToolSection::Edit { new: "", .. }]
        ));
        let patch = detail(
            json!({"type":"edit","file_path":"a","old_string":"old","new_string":"new","unified_diff":"@@ -1 +1 @@\n-old\n+new"}),
        );
        assert!(matches!(
            tool_sections(&patch).as_slice(),
            [ToolSection::Patch { .. }]
        ));
    }

    #[test]
    fn search_results_make_bounded_section_count_instead_of_mounting_each_path() {
        let search = detail(
            json!({"type":"search","query":"x","file_paths": ["a","b","c"], "annotations":["note"]}),
        );
        let sections = tool_sections(&search);
        assert_eq!(sections.len(), 2);
        assert!(matches!(&sections[0], ToolSection::Text(text) if text == "a\nb\nc"));
    }
    #[test]
    fn plain_and_unknown_string_bodies_stay_literal_prose_without_json_headers() {
        let text = "  <b>literal</b> **reasoning**\n한글  ";
        let plain = detail(json!({"type":"plain_text","text":text}));
        assert!(
            matches!(tool_sections(&plain).as_slice(), [ToolSection::Prose(prose)] if prose == text)
        );
        let unknown = detail(json!({"type":"unknown","input":text,"output":null}));
        assert!(
            matches!(tool_sections(&unknown).as_slice(), [ToolSection::Prose(prose)] if prose == text)
        );
        let with_result = detail(json!({"type":"unknown","input":text,"output":0}));
        assert!(matches!(
            tool_sections(&with_result).as_slice(),
            [
                ToolSection::Opaque {
                    label: ToolSectionLabel::Input,
                    ..
                },
                ToolSection::Opaque {
                    label: ToolSectionLabel::Output,
                    ..
                }
            ]
        ));
        let empty = detail(json!({"type":"unknown","input":null,"output":null}));
        assert!(tool_sections(&empty).is_empty());
    }
}
