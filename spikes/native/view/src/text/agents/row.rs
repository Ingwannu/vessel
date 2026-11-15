//! Provider-row status labels, distinct from catalogue badge labels.
use super::{AgentsWords, Lang};
use crate::agent_status::row::Row;
impl AgentsWords {
    /// Title-line status beside a semantic dot. No values; keep short.
    #[must_use]
    pub const fn row_status(&self, state: &Row) -> &'static str {
        match (self.lang, state) {
            (Lang::En, Row::Available { .. }) => "Available",
            (Lang::Ko, Row::Available { .. }) => "사용 가능",
            (Lang::En, Row::NotInstalled) => "Not installed",
            (Lang::Ko, Row::NotInstalled) => "미설치",
            (Lang::En, Row::NeedsAttention) => "Needs attention",
            (Lang::Ko, Row::NeedsAttention) => "확인 필요",
            (Lang::En, Row::Error { .. }) => "Error",
            (Lang::Ko, Row::Error { .. }) => "오류",
            (Lang::En, Row::Unknown) => "Not known",
            (Lang::Ko, Row::Unknown) => "확인 전",
            (Lang::En, Row::SignedOut) => "Signed out",
            (Lang::Ko, Row::SignedOut) => "로그아웃 상태",
            (Lang::En, Row::Loading) => "Loading",
            (Lang::Ko, Row::Loading) => "불러오는 중",
        }
    }
    /// The Providers page's list heading, scoped to the chosen machine:
    /// Paseo's `settings.providers.title` ("Providers", "프로바이더"), the
    /// page's own title, with the machine named. Value is a machine label;
    /// wrapping is allowed. Korean 의 is invariant for every label.
    #[must_use]
    pub fn row_section(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("Providers on {machine}"),
            Lang::Ko => format!("{machine}의 프로바이더"),
        }
    }
}
