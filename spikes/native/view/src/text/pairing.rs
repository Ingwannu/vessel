//! The words of pairing a machine, as Paseo's (`i18n/resources/{en,ko}.ts`:
//! `pairing.link`, `pairing.hostConfirmation`, `pairing.securityWarning`),
//! in English and Korean. See `text/mod.rs` for the convention.
//!
//! One deliberate difference: Paseo's sentences call the same thing a
//! server, a daemon and a host across the two steps; here it is a
//! machine in both steps and both languages, the word the rest of the
//! app uses (the sidebar, Add a machine, the pairing screen's name field;
//! the coordinator's review of 0.0.0.0.8, 2026-11-13).

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
    helper: "Paste the pairing link your machine printed.",
    label: "Pairing link",
    security_warning: "Treat this pairing link like a password. Anyone with it can access this machine.",
    cancel: "Cancel",
    pair: "Pair",
    pairing: "Pairing...",
    confirm_title: "Connect to this machine?",
    confirm_description: "This machine will be able to run code in this app and reach your other connected machines. Only connect if you recognize it.",
    host: "Machine",
    fingerprint: "Key fingerprint",
    relay: "Relay",
    connect: "Connect",
};

const KO: PairingWords = PairingWords {
    title: "페어링 링크 붙여넣기",
    helper: "머신이 출력한 페어링 링크를 붙여넣으세요.",
    label: "페어링 링크",
    security_warning: "이 페어링 링크는 비밀번호처럼 취급하세요. 링크를 가진 사람은 누구나 이 머신에 접근할 수 있습니다.",
    cancel: "취소",
    pair: "페어링",
    pairing: "페어링 중...",
    confirm_title: "이 머신에 연결할까요?",
    confirm_description: "이 머신은 이 앱에서 코드를 실행하고 연결된 다른 머신에 접근할 수 있게 됩니다. 알고 있는 머신인 경우에만 연결하세요.",
    host: "머신",
    fingerprint: "키 지문",
    relay: "릴레이",
    connect: "연결",
};
