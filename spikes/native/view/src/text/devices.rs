//! Devices' sentences, including safe, localized failures.

use super::Lang;
mod details;
mod row;

use crate::devices::Problem;
use crate::devices::time::Remaining;

use vessel_proto::ErrorCode;

impl Lang {
    /// The Devices page's words.
    #[must_use]
    pub const fn devices(self) -> &'static DevicesWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// A complete language of the Devices page.
#[derive(Debug)]
pub struct DevicesWords {
    /// Language for interpolated sentences.
    pub lang: Lang,
    /// Paired devices section.
    pub paired: &'static str,
    /// Device-list refresh row label.
    pub list_controls: &'static str,
    /// A failed list row label.
    pub list_error_title: &'static str,
    /// Read-only details row: exact pairing time in the viewer's zone.
    pub paired_at_label: &'static str,
    /// Read-only details row: exact last authentication time, or its absence.
    pub last_seen_label: &'static str,
    /// Read-only details row: every independent grant, not the primary summary.
    pub access_label: &'static str,
    /// Details value when the node has no last-seen timestamp.
    pub never_connected: &'static str,
    /// Close the read-only details; this does not cancel a revoke.
    pub close_details: &'static str,
    /// Invitation section.
    pub pair_another: &'static str,
    /// Device name label.
    pub name: &'static str,
    /// Initial device label.
    pub phone: &'static str,
    /// Creating an invitation.
    pub new_link: &'static str,
    /// Busy invitation label.
    pub creating: &'static str,
    /// Retrying a failed read.
    pub retry: &'static str,
    /// Refreshing a ready list.
    pub refresh: &'static str,
    /// Empty device list.
    pub empty: &'static str,
    /// No machines.
    pub no_machines: &'static str,
    /// A disconnected machine.
    pub offline: &'static str,
    /// Revocation action.
    pub revoke: &'static str,
    /// A pending revocation.
    pub revoking: &'static str,
    /// Cancel confirmation.
    pub cancel: &'static str,
    /// Reveal link.
    pub show: &'static str,
    /// Mask link.
    pub hide: &'static str,
    /// Clipboard action.
    pub copy: &'static str,
    /// Clipboard completion, no credential text.
    pub copied: &'static str,
    /// Masked credential placeholder.
    pub hidden: &'static str,
    /// Password-equivalent warning, in Paseo's placement below the link.
    pub warning: &'static str,
    /// Device verification guidance.
    pub verify: &'static str,
    /// QR size fallback.
    pub qr_unavailable: &'static str,
    /// Expired invitation.
    pub expired: &'static str,
    /// Badge while the list is loading.
    pub loading_badge: &'static str,
}

const EN: DevicesWords = DevicesWords {
    lang: Lang::En,
    qr_unavailable: "QR code unavailable. Copy the link instead.",
    paired: "Paired devices",
    list_controls: "Device list",
    list_error_title: "Devices unavailable",
    paired_at_label: "Paired",
    last_seen_label: "Last seen",
    access_label: "Access",
    never_connected: "No connection recorded",
    close_details: "Close",
    pair_another: "Pair another device",
    name: "Device name",
    phone: "phone",
    new_link: "New link",
    creating: "Creating link",
    retry: "Try again",
    refresh: "Refresh",
    empty: "No devices yet. One paired with a link below shows up here.",
    no_machines: "No machines yet.",
    offline: "Connect this machine to list its devices and make a pairing link.",
    revoke: "Revoke",
    revoking: "Revoking access",
    cancel: "Cancel",
    show: "Show pairing link",
    hide: "Hide pairing link",
    copy: "Copy link",
    copied: "Link copied and hidden",
    hidden: "Pairing link hidden",
    warning: "Treat this pairing link like a password. Anyone with it can access this machine.",
    verify: "Paste the link at the other device’s prompt, never on a command line. Compare this key before connecting:",
    expired: "This invitation expired. Make a new link.",
    loading_badge: "Loading",
};
const KO: DevicesWords = DevicesWords {
    lang: Lang::Ko,
    qr_unavailable: "QR 코드를 표시할 수 없습니다. 링크를 복사하세요.",
    paired: "페어링된 기기",
    list_controls: "기기 목록",
    list_error_title: "기기를 불러올 수 없음",
    paired_at_label: "페어링",
    last_seen_label: "마지막 접속",
    access_label: "권한",
    never_connected: "접속 기록 없음",
    close_details: "닫기",
    pair_another: "다른 기기 페어링",
    name: "기기 이름",
    phone: "휴대폰",
    new_link: "새 링크",
    creating: "링크 만드는 중",
    retry: "다시 시도",
    refresh: "새로고침",
    empty: "아직 기기가 없습니다. 아래 링크로 페어링하면 여기에 나타납니다.",
    no_machines: "아직 머신이 없습니다.",
    offline: "머신을 연결하면 기기를 확인하고 페어링 링크를 만들 수 있습니다.",
    revoke: "접근 해제",
    revoking: "접근 해제 중",
    cancel: "취소",
    show: "페어링 링크 보기",
    hide: "페어링 링크 숨기기",
    copy: "링크 복사",
    copied: "링크를 복사하고 숨겼습니다",
    hidden: "페어링 링크 숨김",
    warning: "페어링 링크는 비밀번호처럼 다루세요. 링크를 가진 사람은 누구나 이 머신에 접근할 수 있습니다.",
    verify: "다른 기기의 입력창에 링크를 붙여 넣으세요. 명령줄에는 넣지 마세요. 연결하기 전에 이 키가 맞는지 확인하세요:",
    expired: "초대가 만료되었습니다. 새 링크를 만드세요.",
    loading_badge: "불러오는 중",
};

impl DevicesWords {
    /// Hint below the device-name label, beside the New link button.
    /// `machine` is the selected book name; this wraps, unlike a badge.
    /// Keep the destination here so it remains visible after scrolling.
    #[must_use]
    pub fn name_hint(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "A link that pairs one device with {machine}; it works once, for ten minutes."
            ),
            Lang::Ko => {
                format!(
                    "{machine}에 기기 하나를 페어링하는 링크입니다. 10분 동안 한 번만 사용할 수 있습니다."
                )
            }
        }
    }

    /// Which machine these devices belong to.
    #[must_use]
    pub fn paired_with(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("Paired with {machine}"),
            Lang::Ko => format!("{machine}에 페어링된 기기"),
        }
    }
    /// Revocation dialog title.
    #[must_use]
    pub fn revoke_title(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("Revoke {name}?"),
            Lang::Ko => format!("{name}의 접근을 해제할까요?"),
        }
    }
    /// The consequence confirmed before revoking.
    #[must_use]
    pub fn revoke_hint(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!(
                "This device loses access to {machine} now, and its open connections close. Pairing it again needs a new link."
            ),
            Lang::Ko => format!(
                "이 기기는 지금부터 {machine}에 접근할 수 없으며, 열려 있는 연결도 닫힙니다. 다시 페어링하려면 새 링크가 필요합니다."
            ),
        }
    }
    /// Invitation row title: whole minutes left, under a minute, or the
    /// expired sentence. `Minutes` carries a numeric count, never a date.
    #[must_use]
    pub fn expires(&self, remaining: Remaining) -> String {
        match (self.lang, remaining) {
            (Lang::En, Remaining::Minutes(1)) => "Expires in 1 minute".to_owned(),
            (Lang::En, Remaining::Minutes(n)) => format!("Expires in {n} minutes"),
            (Lang::Ko, Remaining::Minutes(n)) => format!("{n}분 뒤 만료"),
            (Lang::En, Remaining::Soon) => "Expires in less than a minute".to_owned(),
            (Lang::Ko, Remaining::Soon) => "1분 안에 만료".to_owned(),
            (Lang::En | Lang::Ko, Remaining::Expired) => self.expired.to_owned(),
        }
    }
    /// A failure without ever forwarding a peer's credential-bearing text.
    #[must_use]
    pub fn problem<'a>(
        &self,
        problem: Problem,
        translations: &'a crate::localization::Translator,
    ) -> &'a str {
        match (self.lang, problem) {
            (Lang::En | Lang::Ko, Problem::TargetChanged) => translations.device_target().changed(),
            (Lang::En, Problem::Offline) => {
                "The machine is not connected. Reconnect the machine. Then try again."
            }
            (Lang::Ko, Problem::Offline) => {
                "머신이 연결되어 있지 않습니다. 머신을 연결하세요. 그런 다음 다시 시도하세요."
            }
            (Lang::En, Problem::Exhausted) => "Reopen the app to continue safely.",
            (Lang::Ko, Problem::Exhausted) => "계속하려면 앱을 다시 여세요.",
            (Lang::En, Problem::NoRoute) => {
                "This machine has no address for pairing. Set a public address or relay in its Vessel setup. Then make a new link."
            }
            (Lang::Ko, Problem::NoRoute) => {
                "이 머신에 페어링용 주소가 없습니다. 머신의 Vessel 설정에서 공인 주소나 릴레이를 지정하세요. 그런 다음 새 링크를 만드세요."
            }
            (Lang::En, Problem::InvalidReply) => {
                "The machine returned an invalid response. Try again."
            }
            (Lang::Ko, Problem::InvalidReply) => {
                "머신의 응답이 올바르지 않습니다. 다시 시도하세요."
            }
            (lang, Problem::Refused(code)) => Self::refusal(lang, code),
        }
    }
    const fn refusal(lang: Lang, code: ErrorCode) -> &'static str {
        match (lang, code) {
            (Lang::En, ErrorCode::Unauthorized | ErrorCode::Forbidden) => {
                "This device cannot manage access. Use an administrator device."
            }
            (Lang::Ko, ErrorCode::Unauthorized | ErrorCode::Forbidden) => {
                "이 기기에는 접근 관리 권한이 없습니다. 관리자 기기를 사용하세요."
            }
            (Lang::En, ErrorCode::ProtocolMismatch | ErrorCode::Unsupported) => {
                "Update the machine to use this feature."
            }
            (Lang::Ko, ErrorCode::ProtocolMismatch | ErrorCode::Unsupported) => {
                "이 기능을 사용하려면 머신을 업데이트하세요."
            }
            (Lang::En, ErrorCode::NotFound) => "That device no longer exists. Refresh the list.",
            (Lang::Ko, ErrorCode::NotFound) => {
                "기기가 더 이상 존재하지 않습니다. 목록을 새로고침하세요."
            }
            (Lang::En, ErrorCode::InvalidRequest) => {
                "The machine refused the request. Check the request. Then try again."
            }
            (Lang::Ko, ErrorCode::InvalidRequest) => {
                "머신이 요청을 거부했습니다. 요청을 확인하세요. 그런 다음 다시 시도하세요."
            }
            (
                Lang::En,
                ErrorCode::Conflict
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Unavailable
                | ErrorCode::Internal,
            ) => "The machine could not complete the request. Try again.",
            (
                Lang::Ko,
                ErrorCode::Conflict
                | ErrorCode::HarnessNotInstalled
                | ErrorCode::SpawnFailed
                | ErrorCode::Unavailable
                | ErrorCode::Internal,
            ) => "머신이 요청을 완료하지 못했습니다. 다시 시도하세요.",
        }
    }
}
