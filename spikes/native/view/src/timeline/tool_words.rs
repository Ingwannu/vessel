//! Canonical category labels use cached Fluent values; provider prose stays intact.

use super::ToolLabelKind;
use crate::localization::TimelineTool;

impl<'a> TimelineTool<'a> {
    /// Closed enum matching keeps the only category-to-translation mapping total.
    #[must_use]
    pub fn label(self, kind: ToolLabelKind) -> &'a str {
        match kind {
            ToolLabelKind::Shell => self.shell(),
            ToolLabelKind::Read => self.read(),
            ToolLabelKind::Edit => self.edit(),
            ToolLabelKind::Write => self.write(),
            ToolLabelKind::Search => self.search(),
            ToolLabelKind::Fetch => self.fetch(),
            ToolLabelKind::Setup => self.setup(),
            ToolLabelKind::Task => self.task(),
            ToolLabelKind::Plan => self.plan(),
            ToolLabelKind::Thinking => self.thinking(),
            ToolLabelKind::Tool => self.tool(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};

    #[test]
    fn canonical_categories_keep_complete_english_and_korean_labels() {
        let en = Translator::get(ENGLISH).unwrap().timeline_tool();
        let ko = Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap()
        .timeline_tool();
        for (kind, english, korean) in [
            (ToolLabelKind::Shell, "Shell", "셸"),
            (ToolLabelKind::Read, "Read", "읽기"),
            (ToolLabelKind::Edit, "Edit", "편집"),
            (ToolLabelKind::Write, "Write", "쓰기"),
            (ToolLabelKind::Search, "Search", "검색"),
            (ToolLabelKind::Fetch, "Fetch", "가져오기"),
            (ToolLabelKind::Setup, "Worktree setup", "워크트리 준비"),
            (ToolLabelKind::Task, "Task", "작업"),
            (ToolLabelKind::Plan, "Plan", "계획"),
            (ToolLabelKind::Thinking, "Thinking", "생각 중"),
            (ToolLabelKind::Tool, "Tool", "도구"),
        ] {
            assert_eq!(en.label(kind), english);
            assert_eq!(ko.label(kind), korean);
        }
    }
}
