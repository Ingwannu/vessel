//! The words of the app's updates (the title bar's status, the About
//! screen, the note after an update) and of the About screen, in English
//! and Korean. See `text/mod.rs` for the convention. Zed's words where it
//! has them ("Checking for updates…", "Restart to update").

use super::Lang;

impl Lang {
    /// The words of updates and the About screen in this language.
    #[must_use]
    pub const fn update(self) -> &'static UpdateText {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

/// One language's sentences.
#[derive(Debug)]
#[expect(missing_docs, reason = "each field is named for the sentence it holds")]
pub struct UpdateText {
    pub lang: Lang,
    // The title bar.
    pub checking: &'static str,
    pub restart: &'static str,
    pub failed: &'static str,
    pub try_again: &'static str,
    // The About screen.
    pub about: &'static str,
    pub updates: &'static str,
    pub check_now: &'static str,
    pub automatic: &'static str,
    pub automatic_note: &'static str,
    pub on: &'static str,
    pub off: &'static str,
    pub not_checked: &'static str,
    pub no_updates: &'static str,
    // After an update.
    pub dismiss: &'static str,
}

const EN: UpdateText = UpdateText {
    lang: Lang::En,
    checking: "Checking for updates…",
    restart: "Restart to update",
    failed: "Update failed",
    try_again: "Try again",
    about: "About",
    updates: "Updates",
    check_now: "Check for updates",
    automatic: "Update automatically",
    automatic_note: "Off: Vessel looks for a new build only when you check.",
    on: "On",
    off: "Off",
    not_checked: "Not checked yet.",
    no_updates: "This build does not update itself (a development build).",
    dismiss: "Dismiss",
};

const KO: UpdateText = UpdateText {
    lang: Lang::Ko,
    checking: "업데이트 확인 중…",
    restart: "다시 시작해 업데이트",
    failed: "업데이트하지 못했습니다",
    try_again: "다시 시도",
    about: "정보",
    updates: "업데이트",
    check_now: "업데이트 확인",
    automatic: "자동으로 업데이트",
    automatic_note: "끄면 직접 확인할 때만 새 빌드를 찾습니다.",
    on: "켬",
    off: "끔",
    not_checked: "아직 확인하지 않았습니다.",
    no_updates: "이 빌드는 스스로 업데이트하지 않습니다(개발 빌드).",
    dismiss: "닫기",
};

impl UpdateText {
    /// "Downloading update… 42%".
    #[must_use]
    pub fn downloading(&self, percent: u8) -> String {
        match self.lang {
            Lang::En => format!("Downloading update… {percent}%"),
            Lang::Ko => format!("업데이트 받는 중… {percent}%"),
        }
    }

    /// The title of the note shown once after an update.
    #[must_use]
    pub fn updated_to(&self, label: &str) -> String {
        match self.lang {
            Lang::En => format!("Updated to {label}"),
            Lang::Ko => format!("{label}(으)로 업데이트했습니다"),
        }
    }

    /// The running build, on the About screen.
    #[must_use]
    pub fn version(&self, label: &str) -> String {
        match self.lang {
            Lang::En => format!("Version {label}"),
            Lang::Ko => format!("버전 {label}"),
        }
    }

    /// The feed's newest build is the one running (or older).
    #[must_use]
    pub fn up_to_date(&self, latest: &str) -> String {
        match self.lang {
            Lang::En => format!("Up to date (the newest build is {latest})."),
            Lang::Ko => format!("최신 버전입니다(가장 새 빌드: {latest})."),
        }
    }

    /// A newer build is in place and runs from the next start.
    #[must_use]
    pub fn ready(&self, label: &str) -> String {
        match self.lang {
            Lang::En => format!("Vessel {label} is ready; it runs when Vessel restarts."),
            Lang::Ko => {
                format!("Vessel {label}이(가) 준비됐습니다. 다시 시작하면 이 빌드로 엽니다.")
            }
        }
    }

    /// Why the last attempt failed, on the About screen.
    #[must_use]
    pub fn failed_because(&self, why: &str) -> String {
        match self.lang {
            Lang::En => format!("The last attempt failed: {why}"),
            Lang::Ko => format!("마지막 시도가 실패했습니다: {why}"),
        }
    }
}
