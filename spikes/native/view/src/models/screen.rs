//! Models search shares the provider sheet's matcher and stable node order.
use super::page::{Listed, Snapshot, StartProblem, State};
use crate::matcher::Query;
use vessel_proto::ModelRoute;

/// Meaning of a failed read; the link retains its original typed source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Reconnect before retrying.
    Offline,
    /// Wait before retrying a read or a full shared command queue.
    Busy,
    /// The read ended without a complete snapshot.
    Timeout,
    /// No browser transport is available.
    NoTransport,
    /// The node refused this device's read access.
    Denied,
    /// This node cannot answer the request.
    Unsupported,
    /// Other typed node/transport failures.
    Failed,
}

/// The reference ranks visible title/id/description fields; this node reports
/// a route name, model id and provider instead. Equal scores keep node order.
#[must_use]
pub fn search<'a>(routes: &'a [ModelRoute], text: &str) -> Vec<&'a ModelRoute> {
    let query = Query::new(text);
    if query.is_empty() {
        return routes.iter().collect();
    }
    let mut matched: Vec<_> = routes
        .iter()
        .filter_map(|route| {
            query
                .score(
                    &[
                        route.name.as_str(),
                        route.model.as_str(),
                        route.provider.as_str(),
                    ],
                    None,
                )
                .map(|score| (score, route))
        })
        .collect();
    matched.sort_by_key(|(score, _)| *score);
    matched.into_iter().map(|(_, route)| route).collect()
}

/// A refresh is a real action only with a machine and a free request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    /// A new read may start.
    Ready,
    /// Keep cached rows while the current read is pending.
    Pending,
    /// No machine or no remaining request identity.
    Unavailable,
}
/// A source category whose whole sentence is translated at the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    /// A dispatched read failed.
    Read(Problem),
    /// The page refused a new read before dispatch.
    Start(StartProblem),
}
impl Issue {
    /// This notice reports a failed read/refusal; it is never a loading badge.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::Read(..) | Self::Start(..) => crate::alert::Variant::Error,
        }
    }
}
/// Mutually exclusive main bodies; a failed first read is never an empty list.
#[derive(Debug)]
pub enum Content<'a> {
    /// Nothing can be read before a machine is chosen.
    NoMachine,
    /// No snapshot yet; the initial read is running or about to start.
    Loading,
    /// Initial failure, with one translated message and retry action.
    Failed(Issue),
    /// The peer cannot list configured models at its protocol version.
    Unsupported,
    /// A successful available list was empty.
    Empty,
    /// A nonempty query has no matching routes.
    NoMatches,
    /// Ranked rows from the current machine's actual snapshot.
    Rows {
        /// The same snapshot supplies profile and agent labels for these rows.
        snapshot: &'a Snapshot,
        /// The first matching route, making an empty row body impossible.
        first: &'a ModelRoute,
        /// Remaining matching routes in stable relevance order.
        rest: Vec<&'a ModelRoute>,
    },
}
/// A cached failure is separate from the still visible successful body.
#[derive(Debug)]
pub struct Presentation<'a> {
    /// Main body, including the single initial-failure message.
    pub content: Content<'a>,
    /// A failure above retained rows, never duplicated in the body.
    pub retained_issue: Option<Issue>,
    /// Whether the refresh control may dispatch a read.
    pub refresh: Refresh,
}
/// Follow `ProviderModalBody`'s initial loading/error/search/empty precedence,
/// while honoring the selected machine and the existing cached snapshot.
#[must_use]
pub fn presentation<'a, E>(
    state: &'a State<E>,
    refused: Option<StartProblem>,
    query: &str,
    problem: impl Fn(&E) -> Problem,
) -> Presentation<'a> {
    let page = state.page();
    let refresh = if state.machine().is_none() || refused == Some(StartProblem::Exhausted) {
        Refresh::Unavailable
    } else if page.pending().is_some() {
        Refresh::Pending
    } else {
        Refresh::Ready
    };
    // A second gesture cannot replace its active ticket or turn loading
    // into an error; the pending presentation already explains refusal.
    let refused = refused
        .filter(|refusal| !matches!(refusal, StartProblem::Busy) || page.pending().is_none());
    let issue = refused
        .map(Issue::Start)
        .or_else(|| page.problem().map(|error| Issue::Read(problem(error))));
    let (content, retained_issue) = if state.machine().is_none() {
        (Content::NoMachine, None)
    } else if let Some(snapshot) = page.snapshot() {
        let content = snapshot_content(snapshot, page.pending(), issue, query);
        let retained = if matches!(&content, Content::Failed(_)) {
            None
        } else {
            issue
        };
        (content, retained)
    } else if page.pending().is_some() {
        (Content::Loading, None)
    } else if let Some(issue) = issue {
        (Content::Failed(issue), None)
    } else {
        (Content::Loading, None)
    };
    Presentation {
        content,
        retained_issue,
        refresh,
    }
}

fn snapshot_content<'a>(
    snapshot: &'a Snapshot,
    pending: Option<vessel_proto::RequestId>,
    issue: Option<Issue>,
    query: &str,
) -> Content<'a> {
    match &snapshot.routes {
        Listed::OlderNode => Content::Unsupported,
        Listed::Available(routes) => {
            let rows = search(routes, query);
            if routes.is_empty() && pending.is_some() {
                Content::Loading
            } else if routes.is_empty() {
                if let Some(issue) = issue {
                    Content::Failed(issue)
                } else if Query::new(query).is_empty() {
                    Content::Empty
                } else {
                    Content::NoMatches
                }
            } else {
                let mut rows = rows.into_iter();
                match rows.next() {
                    Some(first) => Content::Rows {
                        snapshot,
                        first,
                        rest: rows.collect(),
                    },
                    None => Content::NoMatches,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod presentation;
    #[test]
    fn visible_model_fields_match_and_equal_scores_keep_the_nodes_order() {
        let routes: Vec<ModelRoute> = serde_json::from_str(
            r#"[
          {"name":"second","provider":"fixture","api":"openai_responses","model":"shared-model"},
          {"name":"first","provider":"fixture","api":"openai_responses","model":"shared-model"},
          {"name":"other","provider":"local","api":"anthropic_messages","model":"한글-모델"}
        ]"#,
        )
        .unwrap();
        for (query, expected) in [
            ("  ", vec!["second", "first", "other"]),
            ("shared", vec!["second", "first"]),
            ("fixture", vec!["second", "first"]),
            ("한글", vec!["other"]),
            ("missing", vec![]),
        ] {
            assert_eq!(
                search(&routes, query)
                    .iter()
                    .map(|row| row.name.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[cfg(test)]
mod ranking_tests {
    use super::*;
    #[test]
    fn strongest_model_match_precedes_an_earlier_weaker_match() {
        // The catalogue's Codex/cod fixture establishes prefix relevance.
        // Configured route names are ids, so use valid names in this shape.
        let routes: Vec<ModelRoute> = serde_json::from_str(r#"[
          {"name":"opencode","provider":"fixture","api":"openai_responses","model":"different/one"},
          {"name":"codex","provider":"fixture","api":"openai_responses","model":"different/two"},
          {"name":"unrelated","provider":"fixture","api":"openai_responses","model":"different/three"}
        ]"#).unwrap();
        assert_eq!(
            search(&routes, "cod")
                .iter()
                .map(|route| route.name.as_str())
                .collect::<Vec<_>>(),
            ["codex", "opencode"]
        );
    }
}
