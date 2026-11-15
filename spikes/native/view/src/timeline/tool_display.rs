//! Tool labels, summaries, icons and detail availability follow canonical meaning.
//! Provider-specific JSON fields and names never select a renderer here.

use std::borrow::Cow;
use std::path::Path;

use vessel_proto::ToolKind;
use vessel_proto::timeline::{AgentMessageEvent, ToolCallDetail, ToolIcon};

use super::tool_sections::opaque_present;

/// Closed captions are translated; provider prose retains its original spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolLabel<'a> {
    /// A canonical category in the person's language.
    Kind(ToolLabelKind),
    /// A delivery event is independent of the receiving tool's lifecycle status.
    AgentMessage(AgentMessageEvent),
    /// A provider-supplied category or humanized ordinary tool name.
    Provided(Cow<'a, str>),
}

/// The display vocabulary never authorizes an operation or chooses a provider tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLabelKind {
    /// A terminal command.
    Shell,
    /// A file read.
    Read,
    /// An existing-file replacement.
    Edit,
    /// A whole-file write.
    Write,
    /// A query.
    Search,
    /// A fetched resource.
    Fetch,
    /// Workspace preparation.
    Setup,
    /// A child launch.
    Task,
    /// Proposed Markdown.
    Plan,
    /// Internal reasoning.
    Thinking,
    /// No supplied name.
    Tool,
}

/// Marker-only calls never acquire a loading-detail affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailContents {
    /// Some meaningful body or target exists.
    Present,
    /// No details yet; a running call may show loading feedback.
    Empty,
    /// Child/workspace activity lives on its own surface.
    Marker,
}

/// Every screen uses one interpretation of a canonical tool body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDisplay<'a> {
    /// The primary caption.
    pub label: ToolLabel<'a>,
    /// Quiet contextual prose or a path, separate from the caption.
    pub summary: Option<Cow<'a, str>>,
    /// Shared semantic icon vocabulary.
    pub icon: ToolIcon,
    /// Whether disclosure can expose meaningful content.
    pub contents: DetailContents,
}

/// Canonical meaning wins over a provider's title and broad legacy tool kind.
#[must_use]
pub fn tool_display<'a>(
    detail: &'a ToolCallDetail,
    title: &'a str,
    kind: ToolKind,
) -> ToolDisplay<'a> {
    let (label, summary, icon) = caption(detail, title, kind);
    ToolDisplay {
        label,
        summary: summary.filter(|text| !text.trim().is_empty()),
        icon,
        contents: contents(detail),
    }
}

fn caption<'a>(
    detail: &'a ToolCallDetail,
    title: &'a str,
    kind: ToolKind,
) -> (ToolLabel<'a>, Option<Cow<'a, str>>, ToolIcon) {
    match detail {
        ToolCallDetail::Shell(shell) => (
            ToolLabel::Kind(ToolLabelKind::Shell),
            Some(Cow::Borrowed(shell.command.as_str())),
            ToolIcon::SquareTerminal,
        ),
        ToolCallDetail::Read(read) => file(ToolLabelKind::Read, &read.file_path, ToolIcon::Eye),
        ToolCallDetail::Edit(edit) => file(ToolLabelKind::Edit, &edit.file_path, ToolIcon::Pencil),
        ToolCallDetail::Write(write) => {
            file(ToolLabelKind::Write, &write.file_path, ToolIcon::Pencil)
        }
        ToolCallDetail::Search(search) => (
            ToolLabel::Kind(ToolLabelKind::Search),
            Some(Cow::Borrowed(search.query.as_str())),
            ToolIcon::Search,
        ),
        ToolCallDetail::Fetch(fetch) => (
            ToolLabel::Kind(ToolLabelKind::Fetch),
            Some(Cow::Owned(fetch.url.to_string())),
            ToolIcon::Search,
        ),
        ToolCallDetail::WorktreeSetup(setup) => (
            ToolLabel::Kind(ToolLabelKind::Setup),
            Some(Cow::Borrowed(setup.branch_name.as_str())),
            ToolIcon::SquareTerminal,
        ),
        ToolCallDetail::SubAgent(agent) => (
            agent
                .sub_agent_type
                .as_ref()
                .map_or(ToolLabel::Kind(ToolLabelKind::Task), |name| {
                    ToolLabel::Provided(Cow::Borrowed(name.as_str()))
                }),
            agent.description.as_deref().map(Cow::Borrowed),
            ToolIcon::Bot,
        ),
        ToolCallDetail::PlainText(plain) => (
            humanized(title),
            plain
                .label
                .as_ref()
                .map(|label| Cow::Borrowed(label.as_str())),
            plain.icon.unwrap_or(ToolIcon::Wrench),
        ),
        ToolCallDetail::Plan(_) => (ToolLabel::Kind(ToolLabelKind::Plan), None, ToolIcon::Brain),
        ToolCallDetail::Unknown(_) if kind == ToolKind::Think => (
            ToolLabel::Kind(ToolLabelKind::Thinking),
            None,
            ToolIcon::Brain,
        ),
        ToolCallDetail::Unknown(_) => (humanized(title), None, ToolIcon::Wrench),
    }
}

fn file(
    kind: ToolLabelKind,
    path: &Path,
    icon: ToolIcon,
) -> (ToolLabel<'_>, Option<Cow<'_, str>>, ToolIcon) {
    (ToolLabel::Kind(kind), Some(path.to_string_lossy()), icon)
}

fn humanized(name: &str) -> ToolLabel<'_> {
    let name = name.trim();
    if name.is_empty() {
        return ToolLabel::Kind(ToolLabelKind::Tool);
    }
    if name.contains([':', '.', '/']) || name.contains("__") {
        return ToolLabel::Provided(Cow::Borrowed(name));
    }
    let text = name
        .split(['_', '-'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let mut letters = text.chars();
    ToolLabel::Provided(Cow::Owned(
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        }),
    ))
}

fn contents(detail: &ToolCallDetail) -> DetailContents {
    let present = match detail {
        ToolCallDetail::Shell(_) | ToolCallDetail::Fetch(_) => true,
        ToolCallDetail::Read(read) => {
            !read.file_path.as_os_str().is_empty() || text_present(read.content.as_deref())
        }
        ToolCallDetail::Write(write) => {
            !write.file_path.as_os_str().is_empty() || text_present(write.content.as_deref())
        }
        ToolCallDetail::Edit(edit) => {
            !edit.file_path.as_os_str().is_empty()
                || edit.old_string.is_some()
                || edit.new_string.is_some()
                || text_present(edit.unified_diff.as_deref())
        }
        ToolCallDetail::Search(search) => {
            !search.query.is_empty()
                || text_present(search.content.as_deref())
                || search
                    .file_paths
                    .as_ref()
                    .is_some_and(|paths| !paths.is_empty())
                || search
                    .web_results
                    .as_ref()
                    .is_some_and(|results| !results.is_empty())
                || search
                    .annotations
                    .as_ref()
                    .is_some_and(|notes| !notes.is_empty())
        }
        ToolCallDetail::PlainText(plain) => {
            plain.label.is_some() || text_present(plain.text.as_deref())
        }
        ToolCallDetail::Plan(plan) => !plan.text.trim().is_empty(),
        ToolCallDetail::Unknown(unknown) => {
            opaque_present(unknown.input.as_ref()) || opaque_present(unknown.output.as_ref())
        }
        ToolCallDetail::WorktreeSetup(_) | ToolCallDetail::SubAgent(_) => {
            return DetailContents::Marker;
        }
    };
    if present {
        DetailContents::Present
    } else {
        DetailContents::Empty
    }
}

fn text_present(text: Option<&str>) -> bool {
    text.is_some_and(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_kind_wins_over_a_conflicting_name_and_legacy_kind() {
        for (detail, label, icon) in [
            (
                json!({"type":"shell","command":"pwd"}),
                ToolLabelKind::Shell,
                ToolIcon::SquareTerminal,
            ),
            (
                json!({"type":"read","file_path":"/a"}),
                ToolLabelKind::Read,
                ToolIcon::Eye,
            ),
            (
                json!({"type":"edit","file_path":"/a"}),
                ToolLabelKind::Edit,
                ToolIcon::Pencil,
            ),
            (
                json!({"type":"write","file_path":"/a"}),
                ToolLabelKind::Write,
                ToolIcon::Pencil,
            ),
            (
                json!({"type":"search","query":"q"}),
                ToolLabelKind::Search,
                ToolIcon::Search,
            ),
            (
                json!({"type":"fetch","url":"https://example.com"}),
                ToolLabelKind::Fetch,
                ToolIcon::Search,
            ),
            (
                json!({"type":"plan","text":"# Plan"}),
                ToolLabelKind::Plan,
                ToolIcon::Brain,
            ),
        ] {
            let detail = serde_json::from_value(detail).unwrap();
            let display = tool_display(&detail, "misleading_name", ToolKind::Other);
            assert_eq!(display.label, ToolLabel::Kind(label));
            assert_eq!(display.icon, icon);
        }
    }

    #[test]
    fn markers_do_not_offer_empty_details_and_plain_text_keeps_its_semantic_icon() {
        let marker =
            serde_json::from_value(json!({"type":"sub_agent","description":"Review"})).unwrap();
        let display = tool_display(&marker, "spawn", ToolKind::Other);
        assert_eq!(display.contents, DetailContents::Marker);
        assert_eq!(display.label, ToolLabel::Kind(ToolLabelKind::Task));
        assert_eq!(display.summary.as_deref(), Some("Review"));
        let plain = serde_json::from_value(
            json!({"type":"plain_text","label":"Transcribed","text":"Hello","icon":"mic_vocal"}),
        )
        .unwrap();
        assert_eq!(
            tool_display(&plain, "transcribe", ToolKind::Other).icon,
            ToolIcon::MicVocal
        );
    }

    #[test]
    fn humanization_preserves_qualified_tool_identity_and_empty_unknown_has_no_body() {
        let detail =
            serde_json::from_value(json!({"type":"unknown","input":{},"output":null})).unwrap();
        for name in [
            "mcp__server__tool",
            "agent.list",
            "host/tool",
            "namespace:tool",
        ] {
            assert_eq!(
                tool_display(&detail, name, ToolKind::Other).label,
                ToolLabel::Provided(Cow::Borrowed(name))
            );
        }
        let display = tool_display(&detail, "read_file", ToolKind::Other);
        assert_eq!(
            display.label,
            ToolLabel::Provided(Cow::Borrowed("Read file"))
        );
        assert_eq!(display.contents, DetailContents::Empty);
    }
}
