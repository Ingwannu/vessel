//! Search/recency policy for owner-projected workspace, agent and file rows.
//! Identities and navigation targets stay generic so callers use W11's canonical
//! registry addresses rather than deriving a workspace from a session or path.

use crate::matcher::{Query, Score};
use chrono::{DateTime, Utc};
use vessel_proto::forge::PullNumber;

/// Pinned result bands follow contributions, in this fixed category order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Canonical registry workspaces; archived entries are excluded by the owner.
    Workspaces,
    /// Agent identities, with hidden cwd supplied by the owner.
    Agents,
    /// Current workspace file search, retaining the node's result order.
    Files,
}
impl Category {
    /// The reference category order, independent of source registration order.
    pub const ALL: [Self; 3] = [Self::Workspaces, Self::Agents, Self::Files];
}

/// A workspace may match a change-request number even without a text match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A captured canonical workspace projection.
    Workspace {
        /// Current change request, when known.
        change_request: Option<PullNumber>,
    },
    /// A captured agent projection.
    Agent,
    /// An already searched file result from the file owner.
    File,
}
impl Kind {
    /// Which pinned category owns this typed row.
    #[must_use]
    pub const fn category(self) -> Category {
        match self {
            Self::Workspace { .. } => Category::Workspaces,
            Self::Agent => Category::Agents,
            Self::File => Category::Files,
        }
    }
}

/// Translated data and a typed owner target, with no I/O or navigation here.
#[derive(Debug, Clone)]
pub struct Row<I, T> {
    /// Stable owner-supplied identity, not a translated title or list offset.
    pub id: I,
    /// Feature-validated navigation target, rechecked again at dispatch.
    pub target: T,
    /// Category and optional change-request metadata.
    pub kind: Kind,
    /// All visible title/subtitle fields, scored independently per token.
    pub visible: Vec<String>,
    /// Hidden cwd/search aliases; visible text always wins.
    pub hidden: Vec<String>,
    /// Last activity; unknown dates sort after every known timestamp.
    pub activity_at: Option<DateTime<Utc>>,
}

/// A files scope suppresses both contributions and other built-in categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The normal root list.
    All,
    /// The active workspace's file search only.
    Files,
}

/// Empty queries show five recent workspaces and five recent agents.
pub const RECENT_LIMIT: usize = 5;

struct Scored<'a, I, T> {
    row: &'a Row<I, T>,
    score: Option<Score>,
    request_hit: bool,
}

/// Project one fixed category with Paseo's relevance/recency policy. Files have
/// already been filtered by the owner's scoped search and keep its ordering.
#[must_use]
pub fn project<'a, I, T>(
    rows: &'a [Row<I, T>],
    text: &str,
    category: Category,
    filter: Scope,
) -> Vec<&'a Row<I, T>> {
    if filter == Scope::Files && category != Category::Files {
        return Vec::new();
    }
    if category == Category::Files {
        return rows.iter().filter(|row| row.kind == Kind::File).collect();
    }
    let query = Query::new(text);
    let request = change_request_query(text);
    let mut scored: Vec<_> = rows
        .iter()
        .filter(|row| row.kind.category() == category)
        .filter_map(|row| {
            let request_hit = match row.kind {
                Kind::Workspace { change_request } => {
                    request.is_some() && request == change_request
                }
                Kind::Agent | Kind::File => false,
            };
            let visible: Vec<_> = row.visible.iter().map(String::as_str).collect();
            let score = query.score(&visible, Some(&row.hidden.join(" ")));
            (score.is_some() || request_hit).then_some(Scored {
                row,
                score,
                request_hit,
            })
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .request_hit
            .cmp(&left.request_hit)
            .then_with(|| match (left.score, right.score) {
                (Some(left), Some(right)) => left.built_in_cmp(right),
                _ => std::cmp::Ordering::Equal,
            })
            .then_with(|| right.row.activity_at.cmp(&left.row.activity_at))
    });
    if query.is_empty() {
        scored.truncate(RECENT_LIMIT);
    }
    scored.into_iter().map(|entry| entry.row).collect()
}

/// Parse only a complete PR/MR-shaped query. Text that merely contains digits
/// falls through to the shared matcher, and the existing wire number type bounds
/// the value identically to the forge owner's data.
#[must_use]
pub fn change_request_query(text: &str) -> Option<PullNumber> {
    let text = text.trim_matches(crate::matcher::whitespace);
    let prefix = text
        .get(..2)
        .filter(|prefix| prefix.eq_ignore_ascii_case("pr") || prefix.eq_ignore_ascii_case("mr"));
    let text = if prefix.is_some() {
        text.get(2..)?
            .trim_start_matches(crate::matcher::whitespace)
    } else {
        text
    };
    let text = text
        .strip_prefix('#')
        .or_else(|| text.strip_prefix('!'))
        .unwrap_or(text);
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    PullNumber::new(text.parse().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(id: u32, title: &str, at: i64, kind: Kind) -> Row<u32, u32> {
        Row {
            id,
            target: id,
            kind,
            visible: vec![title.to_owned()],
            hidden: Vec::new(),
            activity_at: DateTime::from_timestamp(at, 0),
        }
    }
    #[test]
    fn category_caps_recency_and_files_scope_keep_typed_targets_separate() {
        let mut rows: Vec<_> = (0..8)
            .map(|id| row(id, "agent", i64::from(id), Kind::Agent))
            .collect();
        rows.push(row(90, "file", 0, Kind::File));
        assert_eq!(
            project(&rows, "", Category::Agents, Scope::All)
                .iter()
                .map(|row| row.target)
                .collect::<Vec<_>>(),
            [7, 6, 5, 4, 3]
        );
        assert!(project(&rows, "agent", Category::Agents, Scope::Files).is_empty());
        assert_eq!(
            project(
                &rows,
                "node already filtered",
                Category::Files,
                Scope::Files
            )[0]
            .id,
            90
        );
    }
    #[test]
    fn visible_match_beats_recent_hidden_match_and_number_hits_do_not_need_title_tokens() {
        let mut hidden = row(2, "Other", 100, Kind::Agent);
        hidden.hidden.push("usage".to_owned());
        let rows = [
            hidden,
            row(1, "Usage", 1, Kind::Agent),
            row(
                3,
                "Release branch",
                0,
                Kind::Workspace {
                    change_request: Some(PullNumber::new(42).unwrap()),
                },
            ),
        ];
        assert_eq!(
            project(&rows, "usage", Category::Agents, Scope::All)[0].id,
            1
        );
        for query in [
            "42",
            "#42",
            "!42",
            "pr 42",
            "PR #42",
            "mr!42",
            "\u{feff}PR\u{feff}#42\u{feff}",
        ] {
            assert_eq!(
                project(&rows, query, Category::Workspaces, Scope::All)[0].id,
                3
            );
        }
        for query in ["fix-42-retries", "pr # 42", "42x", "0", "pr\u{85}42"] {
            assert!(change_request_query(query).is_none());
        }
    }
}
