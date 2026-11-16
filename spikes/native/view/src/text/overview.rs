//! The words of Settings › Overview and Settings › Connections (Paseo's
//! `settings.host`: `badges`, `appearance`, `daemon.rename`,
//! `daemon.dangerZone`, `daemon.remove`, `connections`), in English and
//! Korean, "machine" where Paseo says "host" (see `text/mod.rs` for the
//! convention).

use super::Lang;
use crate::connections::Route;

impl Lang {
    /// The words of Settings › Overview in this language.
    #[must_use]
    pub const fn overview(self) -> &'static OverviewText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct OverviewText {
    pub lang: Lang,
    // The status badge at the top (Paseo's `formatConnectionStatus`). Short.
    pub online: &'static str,
    pub offline: &'static str,
    // The alert under the badges while the machine does not answer (Paseo
    // shows the connection's last error there; the app says it in its own
    // words, as the sidebar does).
    pub unreachable: &'static str,
    // The "Appearance" section and its "Name" row; the pencil's tooltip.
    pub appearance: &'static str,
    pub name: &'static str,
    pub rename: &'static str,
    // The rename dialog: its title, the field's placeholder (a name the
    // book accepts), its two buttons. Short.
    pub rename_title: &'static str,
    pub rename_placeholder: &'static str,
    pub rename_submit: &'static str,
    pub cancel: &'static str,
    // The "Danger zone" section's remove row, its button and its dialog.
    pub danger_zone: &'static str,
    pub remove_title: &'static str,
    pub remove_hint: &'static str,
    pub remove: &'static str,
    // Settings › Connections: its section's title (Paseo's
    // `connections.title`).
    pub connections: &'static str,
}

const EN: OverviewText = OverviewText {
    lang: Lang::En,
    online: "Online",
    offline: "Offline",
    unreachable: "This machine is not answering.",
    appearance: "Appearance",
    name: "Name",
    rename: "Rename machine",
    rename_title: "Rename machine",
    rename_placeholder: "my-machine",
    rename_submit: "Rename",
    cancel: "Cancel",
    danger_zone: "Danger zone",
    remove_title: "Remove machine",
    remove_hint: "Removes this machine and its saved connection from this device",
    remove: "Remove",
    connections: "Connections",
};

const KO: OverviewText = OverviewText {
    lang: Lang::Ko,
    online: "온라인",
    offline: "오프라인",
    unreachable: "이 머신이 응답하지 않습니다.",
    appearance: "모양",
    name: "이름",
    rename: "머신 이름 변경",
    rename_title: "머신 이름 변경",
    rename_placeholder: "my-machine",
    rename_submit: "이름 변경",
    cancel: "취소",
    danger_zone: "위험 구역",
    remove_title: "머신 제거",
    remove_hint: "이 머신과 저장된 연결을 이 기기에서 제거합니다",
    remove: "제거",
    connections: "연결",
};

impl OverviewText {
    /// The remove dialog's question (Paseo's `daemon.remove.confirmMessage`),
    /// naming the machine; a whole sentence. Korean puts the name before
    /// "머신을" so no particle follows the name itself.
    #[must_use]
    pub fn remove_confirm(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("Remove {machine}? This will delete its saved connection."),
            Lang::Ko => format!("{machine} 머신을 제거할까요? 저장된 연결이 삭제됩니다."),
        }
    }
}

impl OverviewText {
    /// A route's row title, as Paseo's `formatHostConnectionLabel`: its
    /// kind, then its endpoint in brackets ("Relay (relay.example:443)").
    /// A direct route is "TCP" whether or not it is a paired machine's
    /// (Paseo names every direct endpoint so); the kinds are Paseo's
    /// `badges.relay` and `badges.remoteSsh`. No particle follows the
    /// endpoint in Korean.
    #[must_use]
    pub fn route(&self, route: &Route) -> String {
        format!("{} ({})", self.route_kind(route), route.endpoint().as_str())
    }

    /// A route's kind alone, for a row whose endpoint is its value under the
    /// title (Paseo's host confirmation: `relayLabel` as the `DetailRow`
    /// label, the endpoint as its value).
    #[must_use]
    pub const fn route_kind(&self, route: &Route) -> &'static str {
        match (route, self.lang) {
            (Route::Tcp(_) | Route::Direct(_), _) => "TCP",
            (Route::Relay(_), Lang::En) => "Relay",
            (Route::Relay(_), Lang::Ko) => "릴레이",
            (Route::Ssh(_), Lang::En) => "Remote SSH",
            (Route::Ssh(_), Lang::Ko) => "원격 SSH",
        }
    }
}
