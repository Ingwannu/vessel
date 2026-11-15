//! Row summaries and full scope metadata have separate translation units.
use super::{DevicesWords, Lang};
use crate::devices::{Access, time::Age};
use vessel_proto::Scope;
impl DevicesWords {
    /// Short access label in the row's one hint; full grants are in its tooltip.
    #[must_use]
    pub const fn access(&self, access: Access) -> &'static str {
        match (self.lang, access) {
            (Lang::En, Access::Manage) => "Admin",
            (Lang::Ko, Access::Manage) => "관리",
            (Lang::En, Access::Sessions) => "Run sessions",
            (Lang::Ko, Access::Sessions) => "세션 실행",
            (Lang::En, Access::ReadOnly) => "Read only",
            (Lang::Ko, Access::ReadOnly) => "읽기 전용",
            (Lang::En, Access::Sharing) => "Settings sync",
            (Lang::Ko, Access::Sharing) => "설정 동기화",
            (Lang::En, Access::Forwarding) => "Other machine access",
            (Lang::Ko, Access::Forwarding) => "다른 머신 접근",
            (Lang::En, Access::None) => "No access",
            (Lang::Ko, Access::None) => "권한 없음",
        }
    }
    /// One compact access/last-use hint. A never-seen badge carries the
    /// absent timestamp; no second no-connection sentence is added.
    #[must_use]
    pub fn row_hint(&self, access: Access, seen: Option<Age>) -> String {
        let access = self.access(access);
        match (self.lang, seen) {
            (_, None) => access.to_owned(),
            (Lang::En, Some(age @ Age::Date(_))) => {
                format!("{access} · seen on {}", self.prose_age(age))
            }
            (Lang::En, Some(age)) => format!("{access} · seen {}", self.prose_age(age)),
            (Lang::Ko, Some(age)) => format!("{access} · {} 접속", self.prose_age(age)),
        }
    }
    /// Full grants in the detailed tooltip; none of the independent scopes
    /// is dropped merely because the row shows its primary access label.
    #[must_use]
    pub fn granted_scopes(&self, scopes: &[Scope]) -> String {
        if scopes.is_empty() {
            return match self.lang {
                Lang::En => "None",
                Lang::Ko => "없음",
            }
            .to_owned();
        }
        scopes
            .iter()
            .map(|scope| match (self.lang, scope) {
                (Lang::En, Scope::Read) => "Read information",
                (Lang::Ko, Scope::Read) => "정보 읽기",
                (Lang::En, Scope::Operate) => "Run sessions",
                (Lang::Ko, Scope::Operate) => "세션 실행",
                (Lang::En, Scope::Admin) => "Manage machine",
                (Lang::Ko, Scope::Admin) => "머신 관리",
                (Lang::En, Scope::Sync) => "Receive shared settings",
                (Lang::Ko, Scope::Sync) => "공유 설정 받기",
                (Lang::En, Scope::Relay) => "Access other machines",
                (Lang::Ko, Scope::Relay) => "다른 머신 접근",
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    /// Exact dates plus all scopes, shown only in the existing tooltip.
    #[must_use]
    pub fn device_tooltip(&self, times: &str, scopes: &[Scope]) -> String {
        let scopes = self.granted_scopes(scopes);
        match self.lang {
            Lang::En => format!("{times}\nAccess: {scopes}"),
            Lang::Ko => format!("{times}\n권한: {scopes}"),
        }
    }
}
