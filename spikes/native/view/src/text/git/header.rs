//! Compact toolbar words have their own place; the tooltip keeps complete
//! sentences rather than reusing a shortened caption as an explanation.

use vessel_proto::workspace::{CommitId, Distance, GitOperation, GitStatus};

use crate::{
    Lang,
    workspace::git::{self, DistanceName, HeadActivity, HeadName, HeadState},
};

/// A commit's compact code value and its exact code tooltip value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHashText {
    /// Twelve hexadecimal characters keep the detached head compact.
    pub short: String,
    /// The node's complete commit id remains separate from prose.
    pub full: CommitId,
}

impl GitHashText {
    /// Compact and exact code values share one length policy everywhere.
    #[must_use]
    pub fn of(commit: &CommitId) -> Self {
        Self {
            short: commit.to_string().chars().take(12).collect(),
            full: commit.clone(),
        }
    }
}

/// The Changes head's short lines and full explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHeaderText {
    /// The branch name once, or a detached/missing-branch label; may wrap.
    pub name: String,
    /// A short state beneath the branch name; may wrap when a distinct
    /// upstream name is long. Complete sentences belong to `tooltip`.
    pub state: String,
    /// A detached head’s commit, rendered in a separate fixed-width element.
    pub commit: Option<GitHashText>,
    /// A different base's short comparison, when it adds information.
    pub base: Option<String>,
    /// Full branch, upstream, base, operation and count sentences for the
    /// head's tooltip, including comparisons omitted from the short head.
    pub tooltip: String,
}

impl Lang {
    /// Changes-toolbar words; `status` is the machine's complete bounded
    /// reply and `activity` describes the displayed read/write state.
    #[must_use]
    pub fn git_header(self, status: &GitStatus, activity: HeadActivity) -> GitHeaderText {
        let head = git::git_head(status, activity);
        let name = match (self, head.name) {
            (_, HeadName::Branch(branch)) => branch.as_str().to_owned(),
            (Self::En, HeadName::Detached) => "Detached HEAD".to_owned(),
            (Self::Ko, HeadName::Detached) => "브랜치에서 분리됨".to_owned(),
            (Self::En, HeadName::Missing) => "No branch".to_owned(),
            (Self::Ko, HeadName::Missing) => "브랜치 없음".to_owned(),
        };
        let state = match (self, head.state) {
            (_, HeadState::Upstream { distance, name }) => self.git_head_distance(distance, name),
            (Self::En, HeadState::Working) => "Working…".to_owned(),
            (Self::Ko, HeadState::Working) => "작업 중…".to_owned(),
            (Self::En, HeadState::Refreshing) => "Refreshing…".to_owned(),
            (Self::Ko, HeadState::Refreshing) => "새로 읽는 중…".to_owned(),
            (Self::En, HeadState::Incomplete) => "Some changes are not listed".to_owned(),
            (Self::Ko, HeadState::Incomplete) => "목록에 없는 변경 있음".to_owned(),
            (Self::En, HeadState::NoCommits) => "No commits yet".to_owned(),
            (Self::Ko, HeadState::NoCommits) => "아직 커밋 없음".to_owned(),
            (Self::En, HeadState::NoUpstream) => "No upstream set".to_owned(),
            (Self::Ko, HeadState::NoUpstream) => "추적 브랜치 연결 없음".to_owned(),
            (Self::En, HeadState::Detached(_)) => "At commit".to_owned(),
            (Self::Ko, HeadState::Detached(_)) => "현재 커밋".to_owned(),
            (_, HeadState::Operation(operation)) => self.git_head_operation(operation).to_owned(),
        };
        let commit = match head.state {
            HeadState::Detached(commit) => Some(GitHashText::of(commit)),
            HeadState::Working
            | HeadState::Refreshing
            | HeadState::Operation(_)
            | HeadState::Incomplete
            | HeadState::NoCommits
            | HeadState::NoUpstream
            | HeadState::Upstream { .. } => None,
        };
        let mut tooltip = vec![match head.name {
            HeadName::Branch(_) => self.git_branch_state(status),
            HeadName::Detached => match self {
                Self::En => "HEAD is detached.".to_owned(),
                Self::Ko => "HEAD가 브랜치에서 분리되어 있습니다.".to_owned(),
            },
            HeadName::Missing => match self {
                Self::En => "No branch is checked out.".to_owned(),
                Self::Ko => "체크아웃된 브랜치가 없습니다.".to_owned(),
            },
        }];
        if status.head.is_none() {
            tooltip.push(match self {
                Self::En => "The repository has no commits yet.".to_owned(),
                Self::Ko => "저장소에 아직 커밋이 없습니다.".to_owned(),
            });
        }
        if let Some(upstream) = &status.upstream {
            tooltip.push(self.git_distance(upstream));
        } else if status.branch.is_some() {
            tooltip.push(self.git_labels().no_upstream.to_owned());
        }
        if let Some(base) = &status.base {
            tooltip.push(self.git_distance(base));
        }
        if let Some(operation) = status.operation {
            tooltip.push(self.git_operation_line(operation));
        }
        tooltip.push(self.git_change_counts(status));
        match activity {
            HeadActivity::Idle => {}
            HeadActivity::Reading => tooltip.push(self.git_labels().read_first.to_owned()),
            HeadActivity::Writing => tooltip.push(self.git_labels().busy.to_owned()),
        }
        GitHeaderText {
            name,
            state,
            commit,
            base: head
                .base
                .map(|base| self.git_head_distance(base, DistanceName::Explicit)),
            tooltip: tooltip.join("\n"),
        }
    }

    /// A short comparison line, using numbers and words. An implicit
    /// upstream avoids repeating the current branch's long name; a
    /// distinct target stays named. No particle follows a Korean name.
    #[must_use]
    pub fn git_head_distance(self, distance: &Distance, label: DistanceName) -> String {
        let ahead = crate::grouped(u64::from(distance.ahead));
        let behind = crate::grouped(u64::from(distance.behind));
        let name = &distance.name;
        match (self, distance.ahead, distance.behind, label) {
            (Self::En, 0, 0, DistanceName::ImplicitUpstream) => "Up to date".to_owned(),
            (Self::Ko, 0, 0, DistanceName::ImplicitUpstream) => "최신 상태".to_owned(),
            (Self::En, 0, 0, DistanceName::Explicit) => format!("Up to date with {name}"),
            (Self::Ko, 0, 0, DistanceName::Explicit) => format!("{name} 기준 최신 상태"),
            (Self::En, _, 0, DistanceName::ImplicitUpstream) => format!("{ahead} ahead"),
            (Self::Ko, _, 0, DistanceName::ImplicitUpstream) => format!("{ahead}개 앞섬"),
            (Self::En, 0, _, DistanceName::ImplicitUpstream) => format!("{behind} behind"),
            (Self::Ko, 0, _, DistanceName::ImplicitUpstream) => format!("{behind}개 뒤처짐"),
            (Self::En, _, _, DistanceName::ImplicitUpstream) => {
                format!("{ahead} ahead, {behind} behind")
            }
            (Self::Ko, _, _, DistanceName::ImplicitUpstream) => {
                format!("{ahead}개 앞섬, {behind}개 뒤처짐")
            }
            (Self::En, _, 0, DistanceName::Explicit) => format!("{ahead} ahead of {name}"),
            (Self::Ko, _, 0, DistanceName::Explicit) => format!("{name} 기준 {ahead}개 앞섬"),
            (Self::En, 0, _, DistanceName::Explicit) => format!("{behind} behind {name}"),
            (Self::Ko, 0, _, DistanceName::Explicit) => format!("{name} 기준 {behind}개 뒤처짐"),
            (Self::En, _, _, DistanceName::Explicit) => {
                format!("{ahead} ahead, {behind} behind {name}")
            }
            (Self::Ko, _, _, DistanceName::Explicit) => {
                format!("{name} 기준 {ahead}개 앞섬, {behind}개 뒤처짐")
            }
        }
    }

    /// Short operation status under the branch; the complete next step
    /// is a separate whole sentence in the same trigger's tooltip.
    #[must_use]
    pub const fn git_head_operation(self, operation: GitOperation) -> &'static str {
        match (self, operation) {
            (Self::En, GitOperation::Merge) => "Merge in progress",
            (Self::Ko, GitOperation::Merge) => "병합 중",
            (Self::En, GitOperation::Rebase) => "Rebase in progress",
            (Self::Ko, GitOperation::Rebase) => "리베이스 중",
            (Self::En, GitOperation::CherryPick) => "Cherry-pick in progress",
            (Self::Ko, GitOperation::CherryPick) => "체리픽 중",
            (Self::En, GitOperation::Revert) => "Revert in progress",
            (Self::Ko, GitOperation::Revert) => "되돌리기 중",
            (Self::En, GitOperation::Bisect) => "Bisect in progress",
            (Self::Ko, GitOperation::Bisect) => "이분 탐색 중",
        }
    }
}
