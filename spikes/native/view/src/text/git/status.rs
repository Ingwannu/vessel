//! Branch state and counts are sentences, never an unexplained pair of
//! arrows. Only a commit id uses the reference's familiar short form.

use vessel_proto::workspace::{Distance, GitStatus};

use crate::Lang;
use crate::workspace::git::CommitForm;

impl Lang {
    /// A branch's relationship to its upstream or base, including the
    /// zero and singular cases.
    #[must_use]
    pub fn git_distance(self, distance: &Distance) -> String {
        let Distance {
            name,
            ahead,
            behind,
            ..
        } = distance;
        let (ahead_text, behind_text) = (
            crate::grouped(u64::from(*ahead)),
            crate::grouped(u64::from(*behind)),
        );
        match (self, *ahead, *behind) {
            (Self::En, 0, 0) => format!("Up to date with {name}."),
            (Self::En, a, 0) => format!("{ahead_text} {} ahead of {name}.", commits(a)),
            (Self::En, 0, b) => format!("{behind_text} {} behind {name}.", commits(b)),
            (Self::En, a, b) => format!(
                "{ahead_text} {} ahead of {name} and {behind_text} {} behind it.",
                commits(a),
                commits(b)
            ),
            (Self::Ko, 0, 0) => format!("{name} 기준으로 최신 상태입니다."),
            (Self::Ko, _, 0) => format!("{name}보다 커밋 {ahead_text}개 앞서 있습니다."),
            (Self::Ko, 0, _) => format!("{name}보다 커밋 {behind_text}개 뒤처져 있습니다."),
            (Self::Ko, _, _) => {
                format!("{name}보다 커밋 {ahead_text}개 앞서고 {behind_text}개 뒤처져 있습니다.")
            }
        }
    }

    /// The branch, a detached HEAD or a repository before its first
    /// commit, each described without an unexplained bare value.
    #[must_use]
    pub fn git_branch_state(self, status: &GitStatus) -> String {
        match (self, &status.branch, &status.head) {
            (Self::En, Some(branch), Some(_)) => format!("On branch {branch}."),
            (Self::Ko, Some(branch), Some(_)) => format!("{branch} 브랜치입니다."),
            (Self::En, Some(branch), None) => format!("On branch {branch}. No commits yet."),
            (Self::Ko, Some(branch), None) => {
                format!("{branch} 브랜치입니다. 아직 커밋이 없습니다.")
            }
            (Self::En, None, Some(head)) => format!(
                "No branch is checked out. At commit {}.",
                short(head.as_str())
            ),
            (Self::Ko, None, Some(head)) => format!(
                "체크아웃된 브랜치가 없습니다. 현재 커밋: {}.",
                short(head.as_str())
            ),
            (Self::En, None, None) => "No branch is checked out. No commits yet.".to_owned(),
            (Self::Ko, None, None) => {
                "체크아웃된 브랜치가 없습니다. 아직 커밋이 없습니다.".to_owned()
            }
        }
    }

    /// Changed and staged file counts. A truncated status does not
    /// pretend to contain the repository's total number of changed files.
    #[must_use]
    pub fn git_change_counts(self, status: &GitStatus) -> String {
        let summary = CommitForm::of(status);
        let (changed, staged) = (summary.changed.get(), summary.staged.get());
        let (changed_text, staged_text) = (count(changed), count(staged));
        if status.more && changed == 0 {
            return match self {
                Self::En => "The change list is incomplete. No files were listed.".to_owned(),
                Self::Ko => "변경 목록이 완전하지 않습니다. 표시된 파일이 없습니다.".to_owned(),
            };
        }
        match (self, status.more) {
            (Self::En, false) => format!(
                "{changed_text} {} changed; {staged_text} {} staged.",
                files(changed),
                files(staged)
            ),
            (Self::Ko, false) => {
                format!("파일 {changed_text}개가 바뀌었고 {staged_text}개가 스테이지되어 있습니다.")
            }
            (Self::En, true) => format!(
                "At least {changed_text} {} changed; {staged_text} of the listed files staged.",
                files(changed)
            ),
            (Self::Ko, true) => format!(
                "최소 {changed_text}개 파일이 바뀌었고, 표시된 파일 중 {staged_text}개가 스테이지되어 있습니다."
            ),
        }
    }
}

const fn commits(count: u32) -> &'static str {
    if count == 1 { "commit" } else { "commits" }
}
const fn files(count: usize) -> &'static str {
    if count == 1 { "file" } else { "files" }
}
fn short(id: &str) -> &str {
    id.get(..7).unwrap_or(id)
}

fn count(value: usize) -> String {
    u64::try_from(value).map_or_else(|_| value.to_string(), crate::grouped)
}
