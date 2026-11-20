//! What a session row's hover card says (TRACKER 810), as Paseo's
//! `WorkspaceHoverCardContent` (`components/workspace-hover-card.tsx`): the
//! session's name, its pull request, its machine, its branch, its folder,
//! and the pull request's checks.
//!
//! [Decision Log]
//! - Purpose: what a row's one line leaves out, a hover away.
//! - Source: the card's own facts (no read when the card opens), as
//!   Paseo's card reads its workspace entry.
//! - The folder's words are Paseo's `workspaceDirectoryLabel`: a
//!   worktree's folder name, else the path with the home folder as `~`
//!   (`shortenPath`); the home folder itself (no folder named) reads `~`.
//!   Copying gives the path as the node wrote it.
//! - The checks row opens the checks page where the forge has one
//!   (GitHub's `/checks` after the pull request's address, Paseo's
//!   `buildForgeChecksUrl`), else the pull request's page.
//! - The checks row counts each state as Paseo's (`ChecksSummaryPill`
//!   over `COUNTED_CHECK_PRESENTATIONS`) from the summary's tally (ADR 0088
//!   part 2, protocol 39; TRACKER 824), and leads with the forge's mark.
//! - Differences, named: Paseo's diff stat is a server field the node does
//!   not send yet (TRACKER 819 adds it after the pull request's row); the
//!   node counts passed, failed and pending only (no warning, action
//!   required or manual), and a summary without a tally (an older node,
//!   or no check counted) says the checks' one state as the row's line
//!   does.

use vessel_proto::forge::{CheckTally, ForgeKind, PullNumber};

use crate::board::{Card, RunsIn};
use crate::workspace::pull::checks::Presentation;
use crate::workspace::pull::{CardChecks, CardPull, PullPhase};

/// GitHub's checks page under a pull request's address.
const GITHUB_CHECKS: &str = "/checks";

/// The card's facts, in Paseo's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverDetails {
    /// The pull request, when the node read one.
    pub pull: Option<HoverPull>,
    /// Its machine.
    pub machine: String,
    /// Its branch, when it runs on one.
    pub branch: Option<String>,
    /// Its folder.
    pub folder: HoverFolder,
    /// Its pull request's checks, when there are any.
    pub checks: Option<HoverChecks>,
}

/// The pull request's badge: its state's mark, its number, its page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverPull {
    /// Its number.
    pub number: PullNumber,
    /// Where it is in its life (its mark).
    pub phase: PullPhase,
    /// Its page.
    pub url: String,
}

/// The folder's words and what copying it gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverFolder {
    /// As the card says it.
    pub label: String,
    /// As the node wrote it.
    pub path: String,
}

/// The checks' row (`Card::checks_url` is its page).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverChecks {
    /// Passed, failed or running.
    pub state: CardChecks,
    /// Their counts per state, when the node counted them.
    pub tally: Option<CheckTally>,
    /// The forge (its mark leads the row).
    pub forge: ForgeKind,
}

impl HoverChecks {
    /// Paseo's pills: each state that has checks, with its count, in
    /// Paseo's order (success, failure, then pending). Empty without a
    /// tally, when the row says the one state instead.
    #[must_use]
    pub fn counted(&self) -> Vec<(Presentation, u32)> {
        let Some(tally) = self.tally else {
            return Vec::new();
        };
        [
            (Presentation::Success, tally.passed),
            (Presentation::Failure, tally.failed),
            (Presentation::Pending, tally.pending),
        ]
        .into_iter()
        .filter(|&(_, count)| count > 0)
        .collect()
    }
}

impl Card {
    /// What its hover card says.
    #[must_use]
    pub fn hover_details(&self) -> HoverDetails {
        let summary = self
            .detail
            .pull_request
            .as_deref()
            .and_then(|pull| pull.summary.as_ref());
        let card_pull = self.detail.pull_request.as_deref().and_then(CardPull::of);
        let pull = card_pull
            .zip(summary)
            .map(|(card_pull, summary)| HoverPull {
                number: card_pull.number,
                phase: card_pull.phase,
                url: summary.url.as_str().to_owned(),
            });
        HoverDetails {
            pull,
            machine: self.machine.clone(),
            branch: self.detail.branch.clone(),
            folder: self.hover_folder(),
            checks: card_pull
                .and_then(|card_pull| card_pull.checks)
                .zip(summary)
                .map(|(state, summary)| HoverChecks {
                    state,
                    tally: summary.tally,
                    forge: summary.repo.kind,
                }),
        }
    }

    /// The page its checks row opens: the forge's checks page, else the
    /// pull request's.
    #[must_use]
    pub fn checks_url(&self) -> Option<String> {
        let summary = self.detail.pull_request.as_deref()?.summary.as_ref()?;
        let page = summary.url.as_str().trim_end_matches('/');
        Some(match summary.repo.kind {
            ForgeKind::Github => format!("{page}{GITHUB_CHECKS}"),
            ForgeKind::Unknown => page.to_owned(),
        })
    }

    fn hover_folder(&self) -> HoverFolder {
        let Some(path) = self.detail.cwd.clone() else {
            return HoverFolder {
                label: "~".to_owned(),
                path: "~".to_owned(),
            };
        };
        let worktree = matches!(self.detail.runs_in, Some(RunsIn::Worktree(_)));
        let label = if worktree {
            last_folder(&path).to_owned()
        } else {
            short_path(&path)
        };
        HoverFolder { label, path }
    }
}

/// `path` with its home folder (`/home/name` or `/Users/name`) as `~`
/// (Paseo's `shortenPath`).
#[must_use]
pub fn short_path(path: &str) -> String {
    for home in ["/home/", "/Users/"] {
        if let Some(rest) = path.strip_prefix(home) {
            let after = rest.find('/').map_or("", |at| rest.split_at(at).1);
            if rest.split('/').next().is_some_and(|user| !user.is_empty()) {
                return format!("~{after}");
            }
        }
    }
    path.to_owned()
}

/// The last folder of `path` (a worktree's slug).
fn last_folder(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    //! The card's facts from a session's card, as Paseo's content reads
    //! its workspace entry.

    use vessel_proto::forge::{
        CheckTally, ChecksState, ForgeAccess, ForgeKind, PullState, SessionPullRequest,
    };
    use vessel_proto::{HarnessKind, ProjectId, SessionId, SessionInfo, SessionMode, SessionSpec};

    use super::{HoverChecks, HoverFolder, short_path};
    use crate::board::{Card, RunsIn};
    use crate::text::Lang;
    use crate::workspace::pull::checks::Presentation;
    use crate::workspace::pull::fixtures::summary;
    use crate::workspace::pull::{CardChecks, PullPhase};

    fn card() -> Card {
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        );
        let info = SessionInfo::new(
            SessionId::random(),
            spec,
            "2026-11-19T08:00:00Z".parse().unwrap(),
        );
        Card::from_session("laptop", &info)
    }

    #[test]
    fn a_plain_session_says_its_machine_and_its_home_folder() {
        let details = card().hover_details();
        assert_eq!(details.pull, None);
        assert_eq!(details.machine, "laptop");
        assert_eq!(details.branch, None);
        assert_eq!(
            details.folder,
            HoverFolder {
                label: "~".to_owned(),
                path: "~".to_owned(),
            }
        );
        assert_eq!(details.checks, None);
        assert_eq!(card().checks_url(), None);
    }

    #[test]
    fn a_worktree_session_says_its_branch_pull_request_and_checks() {
        let mut card = card();
        card.detail.branch = Some("fix-cart".to_owned());
        card.detail.cwd = Some("/home/dev/.vessel/worktrees/shop/fix-cart".to_owned());
        card.detail.runs_in = Some(RunsIn::Worktree(ProjectId::random()));
        let mut read = summary(PullState::Open, false);
        read.checks = ChecksState::Failing;
        card.detail.pull_request = Some(Box::new(SessionPullRequest {
            access: ForgeAccess::Ready { login: None },
            summary: Some(read),
            feedback: false,
        }));
        let details = card.hover_details();
        let pull = details.pull.unwrap();
        assert_eq!(pull.number.get(), 42);
        assert_eq!(pull.phase, PullPhase::Open);
        assert_eq!(pull.url, "https://github.com/acme/widgets/pull/42");
        assert_eq!(details.branch.as_deref(), Some("fix-cart"));
        // A worktree reads as its folder's name; copying gives the path.
        assert_eq!(details.folder.label, "fix-cart");
        assert_eq!(
            details.folder.path,
            "/home/dev/.vessel/worktrees/shop/fix-cart"
        );
        assert_eq!(
            details.checks.map(|checks| checks.state),
            Some(CardChecks::Failed)
        );
        assert_eq!(
            card.checks_url().as_deref(),
            Some("https://github.com/acme/widgets/pull/42/checks")
        );
        // A forge this client does not know: the pull request's own page.
        if let Some(pull) = card.detail.pull_request.as_mut()
            && let Some(read) = pull.summary.as_mut()
        {
            read.repo.kind = ForgeKind::Unknown;
        }
        assert_eq!(
            card.checks_url().as_deref(),
            Some("https://github.com/acme/widgets/pull/42")
        );
    }

    #[test]
    fn the_checks_row_counts_each_state_in_paseos_order() {
        let mut card = card();
        let mut read = summary(PullState::Open, false);
        read.checks = ChecksState::Failing;
        read.tally = Some(CheckTally {
            passed: 4,
            failed: 1,
            pending: 0,
        });
        card.detail.pull_request = Some(Box::new(SessionPullRequest {
            access: ForgeAccess::Ready { login: None },
            summary: Some(read),
            feedback: false,
        }));
        let checks = card.hover_details().checks.unwrap();
        assert_eq!(checks.forge, ForgeKind::Github);
        // A zero is left out, as Paseo's pill is.
        assert_eq!(
            checks.counted(),
            [(Presentation::Success, 4), (Presentation::Failure, 1)]
        );
        assert_eq!(
            Lang::En.sidebar().checks_counted(&checks.counted()),
            "Checks, Passed: 4, Failed: 1"
        );
        assert_eq!(
            Lang::Ko.sidebar().checks_counted(&checks.counted()),
            "검사, 통과: 4개, 실패: 1개"
        );
    }

    #[test]
    fn without_a_tally_the_row_says_one_state() {
        let checks = HoverChecks {
            state: CardChecks::Running,
            tally: None,
            forge: ForgeKind::Unknown,
        };
        assert!(checks.counted().is_empty());
    }

    #[test]
    fn a_path_reads_with_its_home_folder_as_a_tilde() {
        assert_eq!(short_path("/home/dev/work/shop"), "~/work/shop");
        assert_eq!(short_path("/Users/dev"), "~");
        assert_eq!(short_path("/srv/work/shop"), "/srv/work/shop");
        assert_eq!(short_path("/home/"), "/home/");
        // A folder in the checkout reads as the path, home shortened.
        let mut card = card();
        card.detail.cwd = Some("/home/dev/work/shop/web".to_owned());
        assert_eq!(card.hover_details().folder.label, "~/work/shop/web");
    }
}
