//! Row labels and complete numeric explanations; no sentence fragments.
use super::{Lang, SyncWords};
use crate::sync::{
    FileKind, FileMode, FileSize, KeptEdits, LocalCommand, MemberState, Presence, Usage,
};
use vessel_proto::{FleetNode, Generation};

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
            (Lang::En, FileKind::Skill) => "Skill file",
            (Lang::Ko, FileKind::Skill) => "스킬 파일",
            (Lang::En, FileKind::Other) => "Shared file",
            (Lang::Ko, FileKind::Other) => "공유 파일",
        }
    }
    /// File size and executable flag; `size` uses the existing binary-unit formatter.
    #[must_use]
    pub fn file_metadata(&self, size: FileSize, mode: FileMode) -> String {
        let size = size.word();
        match (self.lang, mode) {
            (Lang::En, FileMode::Data) => format!("Size: {size}."),
            (Lang::En, FileMode::Executable) => {
                format!("Size: {size}. This file can run as a program.")
            }
            (Lang::Ko, FileMode::Data) => format!("크기: {size}."),
            (Lang::Ko, FileMode::Executable) => format!("크기: {size}. 실행할 수 있는 파일입니다."),
        }
    }
    /// Short member badge; the behind count is explained beneath it.
    #[must_use]
    pub const fn member_state(&self, state: Option<MemberState>) -> &'static str {
        match (self.lang, state) {
            (Lang::En, Some(MemberState::UpToDate)) => "Up to date",
            (Lang::Ko, Some(MemberState::UpToDate)) => "최신",
            (Lang::En, Some(MemberState::Behind(_))) => "Behind",
            (Lang::Ko, Some(MemberState::Behind(_))) => "이전 버전",
            (Lang::En, Some(MemberState::NotYet)) => "Not reported",
            (Lang::Ko, Some(MemberState::NotYet)) => "보고 전",
            (Lang::En, None) => "Not checked",
            (Lang::Ko, None) => "확인 안 됨",
        }
    }
    /// Numeric comparison sentence beneath a member badge, only when useful.
    #[must_use]
    pub fn comparison(&self, state: Option<MemberState>) -> Option<String> {
        match (self.lang, state) {
            (Lang::En, Some(MemberState::Behind(1))) => {
                Some("1 version behind the home.".to_owned())
            }
            (Lang::En, Some(MemberState::Behind(n))) => {
                Some(format!("{n} versions behind the home."))
            }
            (Lang::Ko, Some(MemberState::Behind(n))) => {
                Some(format!("홈 머신보다 {n}개 이전 버전입니다."))
            }
            (Lang::En, None) => Some("The current shared version could not be checked.".to_owned()),
            (Lang::Ko, None) => Some("현재 공유 버전을 확인하지 못했습니다.".to_owned()),
            (_, Some(MemberState::NotYet | MemberState::UpToDate)) => None,
        }
    }
    /// Last applied version, or an explicit missing report; wraps.
    #[must_use]
    pub fn applied(&self, version: Option<Generation>) -> String {
        match (self.lang, version.map(Generation::get)) {
            (Lang::En, Some(n)) => format!("Applied version {n}."),
            (Lang::Ko, Some(n)) => format!("적용한 버전: {n}."),
            (Lang::En, None) => "No applied version reported.".to_owned(),
            (Lang::Ko, None) => "적용한 버전의 보고가 없습니다.".to_owned(),
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
    /// The actual connection version, in ordinary words below its name.
    #[must_use]
    pub fn connection_details(&self, node: &FleetNode) -> String {
        let n = node.protocol.get();
        match (self.lang, node.reachable) {
            (Lang::En, true) => format!("This machine can be reached. Connection version {n}."),
            (Lang::En, false) => format!("This machine cannot be reached. Connection version {n}."),
            (Lang::Ko, true) => format!("연결할 수 있는 머신입니다. 연결 버전: {n}."),
            (Lang::Ko, false) => format!("연결할 수 없는 머신입니다. 연결 버전: {n}."),
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
            (Lang::En, LocalCommand::Init) => {
                "Make a home on the machine that should share its files."
            }
            (Lang::Ko, LocalCommand::Init) => "파일을 공유할 머신에서 홈을 만드세요.",
            (Lang::En, LocalCommand::Invite) => {
                "Create a fleet invitation on the home. Treat its link like a password."
            }
            (Lang::Ko, LocalCommand::Invite) => {
                "홈 머신에서 공유 초대를 만드세요. 초대 링크는 비밀번호처럼 다루세요."
            }
            (Lang::En, LocalCommand::Join) => {
                "Join on the receiving machine. Paste the home's fleet invitation at its prompt."
            }
            (Lang::Ko, LocalCommand::Join) => {
                "파일을 받을 머신에서 가입하세요. 입력창에 홈 머신의 공유 초대 링크를 붙여 넣으세요."
            }
            (Lang::En, LocalCommand::Leave) => {
                "Leave on the member machine. Its shared files stay in place."
            }
            (Lang::Ko, LocalCommand::Leave) => {
                "구성원 머신에서 탈퇴하세요. 공유받은 파일은 그대로 남습니다."
            }
            (Lang::En, LocalCommand::Status) => {
                "Check the last applied version locally. Changes are pulled automatically while the node runs."
            }
            (Lang::Ko, LocalCommand::Status) => {
                "머신에서 마지막으로 적용한 버전을 확인하세요. 노드가 실행되는 동안 변경 사항을 자동으로 가져옵니다."
            }
        }
    }
}
