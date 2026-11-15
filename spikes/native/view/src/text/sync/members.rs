//! Compact metadata has its own words; these are not badge labels or errors.
use super::{Lang, SyncWords};
use crate::{
    devices::time::Age,
    sync::{INCOMING_SUFFIX, MemberBadge, MemberState},
};
use vessel_proto::Generation;
impl SyncWords {
    /// One short badge at a member row's trailing edge.
    #[must_use]
    pub const fn member_badge(&self, badge: MemberBadge) -> &'static str {
        match (self.lang, badge) {
            (Lang::En, MemberBadge::Unreported) => "Not reported",
            (Lang::Ko, MemberBadge::Unreported) => "보고 전",
            (Lang::En, MemberBadge::VersionUnreported) => "Version unknown",
            (Lang::Ko, MemberBadge::VersionUnreported) => "버전 미보고",
            (Lang::En, MemberBadge::Current) => "Up to date",
            (Lang::Ko, MemberBadge::Current) => "최신",
            (Lang::En, MemberBadge::Behind) => "Behind",
            (Lang::Ko, MemberBadge::Behind) => "이전 버전",
            (Lang::En, MemberBadge::Unchecked) => "Not checked",
            (Lang::Ko, MemberBadge::Unchecked) => "확인 안 됨",
        }
    }
    /// One hint with the version/comparison and report age. Missing fields
    /// add no boilerplate; wholly absent reports are stated by the badge.
    #[must_use]
    pub fn member_hint(
        &self,
        applied: Option<Generation>,
        state: Option<MemberState>,
        age: Option<Age>,
    ) -> Option<String> {
        let when = age.map(|age| self.report_age(age));
        let version = applied.map(Generation::get);
        let behind = match state {
            Some(MemberState::Behind(n)) => Some(n),
            Some(MemberState::NotYet | MemberState::UpToDate) | None => None,
        };
        match (self.lang, version, behind, when) {
            (Lang::En, Some(v), Some(n), Some(when)) => {
                Some(format!("Version {v} · {n} behind home · reported {when}"))
            }
            (Lang::Ko, Some(v), Some(n), Some(when)) => {
                Some(format!("버전 {v} · 홈보다 {n}개 이전 · {when} 보고"))
            }
            (Lang::En, Some(v), Some(n), None) => Some(format!("Version {v} · {n} behind home")),
            (Lang::Ko, Some(v), Some(n), None) => Some(format!("버전 {v} · 홈보다 {n}개 이전")),
            (Lang::En, Some(v), None, Some(when)) => Some(format!("Version {v} · reported {when}")),
            (Lang::Ko, Some(v), None, Some(when)) => Some(format!("버전 {v} · {when} 보고")),
            (Lang::En, Some(v), None, None) => Some(format!("Version {v}")),
            (Lang::Ko, Some(v), None, None) => Some(format!("버전 {v}")),
            (Lang::En, None, _, Some(when)) => Some(format!("Reported {when}")),
            (Lang::Ko, None, _, Some(when)) => Some(format!("{when} 보고")),
            (_, None, _, None) => None,
        }
    }
    /// A complete sentence following the path list. Rendering changes only
    /// the suffix's family, preserving the language's sentence order.
    #[must_use]
    pub fn received_copies(&self) -> String {
        match self.lang {
            Lang::En => format!(
                "Received shared copies were kept beside the originals with {INCOMING_SUFFIX} appended to their names."
            ),
            Lang::Ko => {
                format!("받은 공유본은 같은 자리에 이름 끝에 {INCOMING_SUFFIX}을 붙여 두었습니다.")
            }
        }
    }
}
