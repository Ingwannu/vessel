//! Row labels and complete numeric explanations; no sentence fragments.
use super::{Lang, SyncWords};
use crate::sync::{FileKind, FileMode, FileSize, KeptEdits, LocalCommand, Presence, Usage};
use vessel_proto::{Generation, ProtocolVersion};

impl SyncWords {
    /// Empty secret names; a failed reference read never implies no references.
    #[must_use]
    pub fn secret_empty(&self, empty: crate::sync::SecretEmpty) -> &'static str {
        match (self.lang, empty) {
            (_, crate::sync::SecretEmpty::NoneReported) => self.empty_secrets,
            (Lang::En, crate::sync::SecretEmpty::StoredOnly) => {
                "This machine stores no secret names. Shared references could not be checked."
            }
            (Lang::Ko, crate::sync::SecretEmpty::StoredOnly) => {
                "이 머신에 저장된 비밀값 이름이 없습니다. 공유 설정에서 사용하는 이름은 확인하지 못했습니다."
            }
        }
    }

    /// Overview title; `machine` is a book name and may wrap.
    #[must_use]
    pub fn on_machine(&self, machine: &str) -> String {
        match self.lang {
            Lang::En => format!("Sync on {machine}"),
            Lang::Ko => format!("{machine}의 동기화"),
        }
    }
    /// Shared version row title; numeric generation, not a raw enum.
    #[must_use]
    pub fn version(&self, version: Generation) -> String {
        let n = version.get();
        match self.lang {
            Lang::En => format!("Shared version {n}"),
            Lang::Ko => format!("공유 버전 {n}"),
        }
    }
    /// Human title above a separate fixed-width bundle path.
    #[must_use]
    pub const fn file_kind(&self, kind: FileKind) -> &'static str {
        match (self.lang, kind) {
            (Lang::En, FileKind::Instructions) => "Instructions",
            (Lang::Ko, FileKind::Instructions) => "지침",
            (Lang::En, FileKind::Skill) => "Skill",
            (Lang::Ko, FileKind::Skill) => "스킬",
            (Lang::En, FileKind::Other) => "Shared file",
            (Lang::Ko, FileKind::Other) => "공유 파일",
        }
    }
    /// A shared file row's hint under its path: its kind, its size and,
    /// when it can run as a program, that word; no label, no period (the
    /// coordinator's review: the path is the row's title, these are facts
    /// about it). `size` uses the existing binary-unit formatter.
    #[must_use]
    pub fn file_hint(&self, kind: FileKind, size: FileSize, mode: FileMode) -> String {
        let kind = self.file_kind(kind);
        let size = size.word();
        match (self.lang, mode) {
            (_, FileMode::Data) => format!("{kind} · {size}"),
            (Lang::En, FileMode::Executable) => format!("{kind} · {size} · executable"),
            (Lang::Ko, FileMode::Executable) => format!("{kind} · {size} · 실행 가능"),
        }
    }
    /// Preserved local files, or an explicit absent report; full count sentences.
    #[must_use]
    pub fn kept(&self, edits: KeptEdits) -> String {
        match (self.lang, edits) {
            (Lang::En, KeptEdits::Unreported) => "No local-change report received.".to_owned(),
            (Lang::Ko, KeptEdits::Unreported) => "로컬 수정의 보고가 없습니다.".to_owned(),
            (Lang::En, KeptEdits::Known(0)) => "No local edits kept.".to_owned(),
            (Lang::En, KeptEdits::Known(1)) => "Kept a local edit in 1 file.".to_owned(),
            (Lang::En, KeptEdits::Known(n)) => format!("Kept local edits in {n} files."),
            (Lang::Ko, KeptEdits::Known(0)) => "유지된 로컬 수정이 없습니다.".to_owned(),
            (Lang::Ko, KeptEdits::Known(n)) => format!("파일 {n}개의 로컬 수정을 유지했습니다."),
        }
    }
    /// Short connection badge; no inference from a member report.
    #[must_use]
    pub const fn connection_state(&self, reachable: bool) -> &'static str {
        match (self.lang, reachable) {
            (Lang::En, true) => "Reachable",
            (Lang::En, false) => "Unreachable",
            (Lang::Ko, true) => "연결 가능",
            (Lang::Ko, false) => "연결 불가",
        }
    }
    /// Extra metadata beneath a connection name; reachability is stated
    /// only by its badge. A bare version value has no sentence punctuation.
    #[must_use]
    pub fn connection_version(&self, protocol: ProtocolVersion) -> String {
        let n = protocol.get();
        match self.lang {
            Lang::En => format!("Connection version {n}"),
            Lang::Ko => format!("연결 버전 {n}"),
        }
    }
    /// A secret-name row's title, separate from its identifier and presence.
    #[must_use]
    pub const fn usage(&self, usage: Usage) -> &'static str {
        match (self.lang, usage) {
            (Lang::En, Usage::Shared) => "Named by shared servers",
            (Lang::Ko, Usage::Shared) => "공유 서버가 사용하는 이름",
            (Lang::En, Usage::LocalOnly) => "Stored on this machine",
            (Lang::Ko, Usage::LocalOnly) => "이 머신에 저장된 이름",
            (Lang::En, Usage::Unreported) => "Usage not checked",
            (Lang::Ko, Usage::Unreported) => "사용 여부 확인 안 됨",
        }
    }
    /// Short presence badge; never a value or its length.
    #[must_use]
    pub const fn presence(&self, presence: Presence) -> &'static str {
        match (self.lang, presence) {
            (Lang::En, Presence::Present) => "Present",
            (Lang::Ko, Presence::Present) => "저장됨",
            (Lang::En, Presence::Missing) => "Missing",
            (Lang::Ko, Presence::Missing) => "없음",
            (Lang::En, Presence::Unknown) => "Not checked",
            (Lang::Ko, Presence::Unknown) => "확인 안 됨",
        }
    }
    /// CLI-only task description; the command is a separate mono element.
    #[must_use]
    pub const fn command(&self, command: LocalCommand) -> &'static str {
        match (self.lang, command) {
            (Lang::En, LocalCommand::Init) => "Make this machine a home to share its files.",
            (Lang::Ko, LocalCommand::Init) => "이 머신을 홈으로 만들어 파일을 공유하세요.",
            (Lang::En, LocalCommand::Invite) => {
                "On the home, create an invitation and keep its link private."
            }
            (Lang::Ko, LocalCommand::Invite) => {
                "홈에서 초대 링크를 만들고 비밀번호처럼 보관하세요."
            }
            (Lang::En, LocalCommand::Join) => "Join on the machine receiving the shared files.",
            (Lang::Ko, LocalCommand::Join) => "공유받을 머신에서 가입하세요.",
            (Lang::En, LocalCommand::Leave) => {
                "Leave on the member machine; shared files stay in place."
            }
            (Lang::Ko, LocalCommand::Leave) => "구성원 머신에서 탈퇴하세요. 공유 파일은 남습니다.",
            (Lang::En, LocalCommand::Status) => "Check the last applied version on this machine.",
            (Lang::Ko, LocalCommand::Status) => "이 머신에서 마지막으로 적용한 버전을 확인하세요.",
        }
    }
}
