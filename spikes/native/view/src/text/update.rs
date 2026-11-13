//! The words of the app's updates (the title bar's status, the About
//! screen, the note after an update) and of the About screen, in English
//! and Korean. See `text/mod.rs` for the convention. The title bar says
//! Zed's words ("Checking for updates…", "Restart to update"); the About
//! screen says Paseo's, as its settings say them in both languages.

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
    // The title bar (Zed's words).
    pub checking: &'static str,
    pub restart: &'static str,
    pub failed: &'static str,
    pub try_again: &'static str,
    // The About screen: Paseo's settings words (`settings.about.*`,
    // `desktop.updates.status.*`, `desktop.daemon.logs.*`).
    pub about: &'static str,
    pub app_version: &'static str,
    pub this_device: &'static str,
    pub app_updates: &'static str,
    pub check: &'static str,
    pub checking_short: &'static str,
    pub update: &'static str,
    pub status_checking: &'static str,
    pub status_up_to_date: &'static str,
    pub status_failed: &'static str,
    pub status_idle: &'static str,
    pub log_file: &'static str,
    pub open_logs: &'static str,
    pub copy_path: &'static str,
    pub log_path_copied: &'static str,
    pub log_unavailable: &'static str,
    pub whats_new: &'static str,
    pub whats_new_hint: &'static str,
    pub install_title: &'static str,
    pub install_message: &'static str,
    pub install_confirm: &'static str,
    pub cancel: &'static str,
    // Vessel's own rows (Paseo has no switch for automatic updates, and
    // its development builds do not show the row).
    pub automatic: &'static str,
    pub automatic_note: &'static str,
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
    app_version: "App version",
    this_device: "This device",
    app_updates: "App updates",
    check: "Check",
    checking_short: "Checking...",
    update: "Update",
    status_checking: "Checking for app updates...",
    status_up_to_date: "App is up to date.",
    status_failed: "Failed to update app.",
    status_idle: "Update status has not been checked yet.",
    log_file: "Log file",
    open_logs: "Open logs",
    copy_path: "Copy path",
    log_path_copied: "Log path copied.",
    log_unavailable: "Log path unavailable",
    whats_new: "What's new",
    whats_new_hint: "Release notes for this version",
    install_title: "Install desktop update",
    install_message: "This updates Vessel on this computer",
    install_confirm: "Install update",
    cancel: "Cancel",
    automatic: "Update automatically",
    automatic_note: "When off, Vessel looks for a new build only when you check.",
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
    app_version: "앱 버전",
    this_device: "이 기기",
    app_updates: "앱 업데이트",
    check: "확인",
    checking_short: "확인 중...",
    update: "업데이트",
    status_checking: "앱 업데이트 확인 중...",
    status_up_to_date: "앱이 최신 상태입니다.",
    status_failed: "앱을 업데이트하지 못했습니다.",
    status_idle: "업데이트 상태를 아직 확인하지 않았습니다.",
    log_file: "로그 파일",
    open_logs: "로그 열기",
    copy_path: "경로 복사",
    log_path_copied: "로그 경로가 복사되었습니다.",
    log_unavailable: "로그 경로를 사용할 수 없습니다",
    whats_new: "새로운 소식",
    whats_new_hint: "이 버전의 릴리스 노트",
    install_title: "데스크톱 업데이트 설치",
    install_message: "이 컴퓨터의 Vessel을 업데이트합니다",
    install_confirm: "업데이트 설치",
    cancel: "취소",
    automatic: "자동으로 업데이트",
    automatic_note: "끄면 확인을 누를 때만 새 빌드를 찾습니다.",
    no_updates: "이 빌드는 스스로 업데이트하지 않습니다(개발 빌드).",
    dismiss: "닫기",
};

impl UpdateText {
    /// "Downloading update… 42%", in the title bar.
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

    /// A build as Paseo writes a version: "v0.0.0.0.6". A build made
    /// without a label (a developer's own, labelled "dev") keeps its word:
    /// "vdev" is no version.
    #[must_use]
    pub fn version(&self, label: &str) -> String {
        if label.starts_with(|c: char| c.is_ascii_digit()) {
            format!("v{label}")
        } else {
            label.to_owned()
        }
    }

    /// The update button once a build is in place: "Update to v0.0.0.0.6".
    #[must_use]
    pub fn update_to(&self, label: &str) -> String {
        let version = self.version(label);
        match self.lang {
            Lang::En => format!("Update to {version}"),
            Lang::Ko => format!("{version}(으)로 업데이트"),
        }
    }

    /// A newer build was found and is downloading.
    #[must_use]
    pub fn status_downloading(&self, label: &str) -> String {
        let version = self.version(label);
        match self.lang {
            Lang::En => format!("Update found: {version}. Downloading..."),
            Lang::Ko => format!("업데이트 발견: {version}. 다운로드 중..."),
        }
    }

    /// A newer build is in place and runs from the next start.
    #[must_use]
    pub fn status_ready(&self, label: &str) -> String {
        let version = self.version(label);
        match self.lang {
            Lang::En => format!("Update ready: {version}"),
            Lang::Ko => format!("업데이트 준비됨: {version}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_numbered_build_reads_as_a_version_and_a_developers_as_its_word() {
        let words = Lang::En.update();
        assert_eq!(words.version("0.0.0.0.7"), "v0.0.0.0.7");
        assert_eq!(words.version("dev"), "dev");
    }
}
