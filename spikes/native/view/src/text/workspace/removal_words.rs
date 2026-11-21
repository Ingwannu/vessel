//! The removal dialog pairs Paseo's warning copy with the count returned
//! by the same inspection (ADR 0120). Tidied worktree details still use
//! the ordinary kept reasons.

use vessel_proto::registry::DiffStat;
use vessel_proto::workspace::KeptReason;

use super::en::counted;
use crate::Lang;

/// The removal warning for a single reason, using counts only for dirty files.
#[must_use]
pub fn removal_reason(lang: Lang, reason: KeptReason, diff_stat: Option<DiffStat>) -> String {
    if !matches!(reason, KeptReason::Uncommitted { .. }) {
        return lang.workspace().kept_reason(reason);
    }
    let title = match lang {
        Lang::En => "Uncommitted changes",
        Lang::Ko => "커밋되지 않은 변경 사항",
    };
    let Some(stat) = diff_stat else {
        return title.to_owned();
    };
    let counts: Vec<_> = [
        (stat.additions, "added line", "added lines", "추가된 줄"),
        (stat.deletions, "deleted line", "deleted lines", "삭제된 줄"),
    ]
    .into_iter()
    .filter(|(count, ..)| *count > 0)
    .map(|(count, one, many, ko)| match lang {
        Lang::En => counted(u64::from(count), one, many),
        Lang::Ko => format!("{ko} {count}개"),
    })
    .collect();
    if counts.is_empty() {
        title.to_owned()
    } else {
        format!("{title} ({})", counts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffStat, KeptReason, Lang, removal_reason};

    #[test]
    fn the_dirty_warning_omits_zero_sides_and_uses_plain_copy_without_counts() {
        let dirty = KeptReason::Uncommitted { files: 1 };
        assert_eq!(removal_reason(Lang::En, dirty, None), "Uncommitted changes");
        let cases = [
            ((0, 0), "Uncommitted changes", "커밋되지 않은 변경 사항"),
            (
                (2, 1),
                "Uncommitted changes (2 added lines, 1 deleted line)",
                "커밋되지 않은 변경 사항 (추가된 줄 2개, 삭제된 줄 1개)",
            ),
            (
                (1, 0),
                "Uncommitted changes (1 added line)",
                "커밋되지 않은 변경 사항 (추가된 줄 1개)",
            ),
            (
                (0, 3),
                "Uncommitted changes (3 deleted lines)",
                "커밋되지 않은 변경 사항 (삭제된 줄 3개)",
            ),
        ];
        for ((additions, deletions), en, ko) in cases {
            let diff_stat = Some(DiffStat {
                additions,
                deletions,
            });
            assert_eq!(removal_reason(Lang::En, dirty, diff_stat), en);
            assert_eq!(removal_reason(Lang::Ko, dirty, diff_stat), ko);
        }
    }

    #[test]
    fn unique_commits_keep_their_own_reason() {
        let diff_stat = Some(DiffStat {
            additions: 2,
            deletions: 1,
        });
        let reason = KeptReason::UniqueCommits { commits: 1 };
        assert_eq!(
            removal_reason(Lang::En, reason, diff_stat),
            "1 commit on no other branch"
        );
    }
}
