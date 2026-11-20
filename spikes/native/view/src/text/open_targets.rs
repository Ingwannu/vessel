//! Open in editor and reveal's words (TRACKER 467): Paseo's English and
//! Korean (`i18n/resources/en.ts`, `ko.ts`, pinned 0.15.4) for its items,
//! and Vessel's own for the reasons a launch is refused, which Paseo's
//! desktop says in English only (so Paseo's fallbacks "Failed to open
//! workspace" and "Failed to reveal entry", shown when its error has no
//! message, have no use here; nor its "Open", the header being icon only).
//! Target names are proper nouns.
use std::path::Path;

use super::Lang;
use crate::open_targets::{Refused, TargetId, Unavailable};

/// Open in editor and reveal's words.
#[derive(Debug)]
pub struct OpenTargetWords {
    /// The caret's name (`openInEditor.chooseEditor`).
    pub choose_editor: &'static str,
    /// The sidebar's item (`sidebar.project.actions.openFolder`).
    pub open_folder: &'static str,
    /// Its failure (`openFolderFailed`).
    pub open_folder_failed: &'static str,
    lang: Lang,
}

impl Lang {
    /// Open in editor and reveal's words in the chosen language.
    #[must_use]
    pub const fn open_targets(self) -> &'static OpenTargetWords {
        match self {
            Self::En => &EN,
            Self::Ko => &KO,
        }
    }
}

const EN: OpenTargetWords = OpenTargetWords {
    choose_editor: "Choose editor",
    open_folder: "Open in file manager",
    open_folder_failed: "Couldn't open folder",
    lang: Lang::En,
};

const KO: OpenTargetWords = OpenTargetWords {
    choose_editor: "편집기 선택",
    open_folder: "파일 관리자에서 열기",
    open_folder_failed: "폴더를 열 수 없습니다.",
    lang: Lang::Ko,
};

impl OpenTargetWords {
    /// The header button's name without a file (`openInEditor.openIn`).
    #[must_use]
    pub fn open_workspace_in(&self, target: TargetId) -> String {
        let target = target.label();
        match self.lang {
            Lang::En => format!("Open workspace in {target}"),
            Lang::Ko => format!("{target}에서 워크스페이스 열기"),
        }
    }

    /// The header button's name with the shown file
    /// (`openInEditor.openFileIn`).
    #[must_use]
    pub fn open_file_in(&self, file: &str, target: TargetId) -> String {
        let target = target.label();
        match self.lang {
            Lang::En => format!("Open {file} in {target}"),
            Lang::Ko => format!("{target}에서 {file} 열기"),
        }
    }

    /// A folder's item in the Files menu (`fileActions.openIn`).
    #[must_use]
    pub fn open_in(&self, target: TargetId) -> String {
        let target = target.label();
        match self.lang {
            Lang::En => format!("Open in {target}"),
            Lang::Ko => format!("{target}에서 열기"),
        }
    }

    /// An entry's item in the Files menu (`fileActions.revealIn`).
    #[must_use]
    pub fn reveal_in(&self, target: TargetId) -> String {
        let target = target.label();
        match self.lang {
            Lang::En => format!("Reveal in {target}"),
            Lang::Ko => format!("{target}에서 보기"),
        }
    }

    /// Why a path was refused before anything started.
    #[must_use]
    pub fn refused(&self, refused: &Refused) -> String {
        let shown = |path: &Path| path.display().to_string();
        match (self.lang, refused) {
            (Lang::En, Refused::NotAbsolute(path)) => {
                format!("Not an absolute local path: {}", shown(path))
            }
            (Lang::Ko, Refused::NotAbsolute(path)) => {
                format!("로컬 절대 경로가 아닙니다: {}", shown(path))
            }
            (Lang::En, Refused::Missing(path)) => format!("Path does not exist: {}", shown(path)),
            (Lang::Ko, Refused::Missing(path)) => format!("경로가 없습니다: {}", shown(path)),
            (Lang::En, Refused::Outside(path)) => {
                format!("This path is outside the workspace: {}", shown(path))
            }
            (Lang::Ko, Refused::Outside(path)) => {
                format!("워크스페이스 밖의 경로입니다: {}", shown(path))
            }
        }
    }

    /// Why the target could not be started.
    #[must_use]
    pub fn unavailable(&self, unavailable: &Unavailable) -> String {
        match (self.lang, unavailable) {
            (Lang::En, Unavailable::NotInstalled(target)) => {
                format!("{} is not installed", target.label())
            }
            (Lang::Ko, Unavailable::NotInstalled(target)) => {
                format!("{} 설치를 찾지 못했습니다", target.label())
            }
            (Lang::En, Unavailable::NoXcodeDocument(folder)) => format!(
                "No Xcode project, workspace, or Swift package in {}",
                folder.display()
            ),
            (Lang::Ko, Unavailable::NoXcodeDocument(folder)) => format!(
                "{}에 Xcode 프로젝트, 워크스페이스, Swift 패키지가 없습니다",
                folder.display()
            ),
        }
    }

    /// The target's program could not be started.
    #[must_use]
    pub fn start_failed(&self, target: TargetId) -> String {
        let target = target.label();
        match self.lang {
            Lang::En => format!("Couldn't start {target}"),
            Lang::Ko => format!("{target} 실행에 실패했습니다"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn paseo_s_items_read_in_both_languages() {
        let en = Lang::En.open_targets();
        let ko = Lang::Ko.open_targets();
        assert_eq!(en.open_in(TargetId::Zed), "Open in Zed");
        assert_eq!(ko.open_in(TargetId::Zed), "Zed에서 열기");
        assert_eq!(en.reveal_in(TargetId::FileManager), "Reveal in Files");
        assert_eq!(ko.reveal_in(TargetId::Finder), "Finder에서 보기");
        assert_eq!(
            en.open_file_in("main.rs", TargetId::VsCode),
            "Open main.rs in VS Code"
        );
        assert_eq!(
            ko.open_workspace_in(TargetId::Cursor),
            "Cursor에서 워크스페이스 열기"
        );
        assert_eq!(
            en.refused(&Refused::Missing(PathBuf::from("/w/a"))),
            "Path does not exist: /w/a"
        );
        assert_eq!(
            ko.unavailable(&Unavailable::NotInstalled(TargetId::Zed)),
            "Zed 설치를 찾지 못했습니다"
        );
    }
}
