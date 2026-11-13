//! The words of pairing a machine, as Paseo's (`i18n/resources/{en,ko}.ts`:
//! `pairing.link`, `pairing.hostConfirmation`, `pairing.securityWarning`),
//! in English and Korean. See `text/mod.rs` for the convention.

use super::Lang;

impl Lang {
    /// The words of pairing in this language.
    #[must_use]
    pub const fn pairing(self) -> &'static PairingWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct PairingWords {
    // The link (Paseo's paste-link modal).
    pub title: &'static str,
    pub helper: &'static str,
    pub label: &'static str,
    pub security_warning: &'static str,
    pub cancel: &'static str,
    pub pair: &'static str,
    pub pairing: &'static str,
    // The host to confirm (Paseo's host confirmation).
    pub confirm_title: &'static str,
    pub confirm_description: &'static str,
    pub host: &'static str,
    pub fingerprint: &'static str,
    pub relay: &'static str,
    pub connect: &'static str,
}

const EN: PairingWords = PairingWords {
    title: "Paste pairing link",
    helper: "Paste the pairing link from your server.",
    label: "Pairing link",
    security_warning: "Treat this pairing link like a password. Anyone with it can access this daemon.",
    cancel: "Cancel",
    pair: "Pair",
    pairing: "Pairing...",
    confirm_title: "Connect to this host?",
    confirm_description: "This host will be able to run code in this app and reach your other connected hosts. Only connect if you recognize it.",
    host: "Host",
    fingerprint: "Key fingerprint",
    relay: "Relay",
    connect: "Connect",
};

const KO: PairingWords = PairingWords {
    title: "페어링 링크 붙여넣기",
    helper: "서버에서 받은 페어링 링크를 붙여넣으세요.",
    label: "페어링 링크",
    security_warning: "이 페어링 링크는 비밀번호처럼 취급하세요. 링크를 가진 사람은 누구나 이 데몬에 접근할 수 있습니다.",
    cancel: "취소",
    pair: "페어링",
    pairing: "페어링 중...",
    confirm_title: "이 호스트에 연결할까요?",
    confirm_description: "이 호스트는 이 앱에서 코드를 실행하고 연결된 다른 호스트에 접근할 수 있게 됩니다. 알고 있는 호스트인 경우에만 연결하세요.",
    host: "호스트",
    fingerprint: "키 지문",
    relay: "릴레이",
    connect: "연결",
};
