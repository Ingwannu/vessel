//! The workspace's words in Korean: the English table's sentences, said as
//! a Korean reader would say them (the app's other Korean is in the same
//! formal register).

use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::ServicePort;
use vessel_proto::workspace::{
    ChangeStatus, CloneFailure, CloneStage, ConfigKey, DiffScope, GitOperation, GitRefusal,
    GitSupport, KeptReason, ProjectState, RepositoryConfigReason, SetupFailure, SetupState,
    WorktreeRefusal,
};

use super::{AddForm, Labels, Local, WorkspaceWords, labels, lines};
use crate::workspace::branch_name::BranchNameProblem;
use crate::workspace::changes::ScopeKind;
use crate::workspace::command_changes::{Change, Compared, ScriptLine};
use crate::workspace::git_quick::{Blocked, GitAction};
use crate::workspace::place::PlaceProblem;
use crate::workspace::problem::Problem;
use crate::workspace::save::SaveFailure;

/// Korean.
#[derive(Debug, Clone, Copy)]
pub struct Ko;

fn config_lines(key: Option<&ConfigKey>, reason: RepositoryConfigReason) -> (String, String) {
    let named = key.map_or_else(
        || "이 저장소의 git 설정".to_owned(),
        |key| format!("이 저장소의 설정 {key}"),
    );
    let next = "Vessel은 저장소 자체 설정이 지정한 프로그램 없이 git을 실행합니다. 이 설정을 전역 git 설정으로 옮기거나(이 저장소에만 쓰려면 includeIf 아래에), 터미널에서 실행하세요.";
    match reason {
        RepositoryConfigReason::Program => lines(
            format!("{named}이(가) 이 작업이 실행할 프로그램을 지정합니다."),
            next,
        ),
        RepositoryConfigReason::Layout => lines(
            format!("{named}이(가) 작업 트리를 폴더 밖으로 옮깁니다."),
            "Vessel에서 이 폴더를 쓰려면 저장소 설정에서 그 항목을 지우세요.",
        ),
        RepositoryConfigReason::Credential => lines(
            format!("{named}이(가) 자격 증명을 다른 곳으로 보낼 수 있습니다."),
            next,
        ),
        RepositoryConfigReason::Unnamable => lines(
            format!("{named}이(가) git이 한 번의 실행에서 끌 수 없는 드라이버를 지정합니다."),
            "저장소 설정에서 드라이버 이름을 바꾸거나, 터미널에서 실행하세요.",
        ),
        RepositoryConfigReason::TooMany => lines(
            "저장소 설정이 지정한 드라이버가 Vessel이 한 번에 끄는 수보다 많습니다.",
            "필요 없는 것을 지우거나, 터미널에서 실행하세요.",
        ),
        RepositoryConfigReason::TooLarge => lines(
            "저장소 설정이 Vessel이 git 실행 전에 읽는 크기보다 큽니다.",
            "설정을 줄이거나, 터미널에서 실행하세요.",
        ),
        RepositoryConfigReason::Unknown => lines(
            format!("{named} 때문에 멈췄습니다. 이 앱이 모르는 이유입니다."),
            "읽으려면 이 앱의 Vessel을 업데이트하세요.",
        ),
    }
}

impl Ko {
    pub(super) fn script_part(self, line: &ScriptLine) -> Option<String> {
        let script = line.script();
        let named = format!(
            "{} {}",
            self.script_kind(script.kind()),
            script.name.as_str()
        );
        match line {
            ScriptLine::Line {
                change: Change::Same,
                ..
            } => None,
            ScriptLine::Line {
                change: Change::Added,
                ..
            } => Some(format!("{named} 추가")),
            ScriptLine::Line {
                change: Change::Removed,
                ..
            } => Some(format!("{named} 삭제")),
            ScriptLine::Changed { script, was } => {
                Some(format!("{named} 변경({})", self.script_change(was, script)))
            }
        }
    }
}

impl WorkspaceWords for Ko {
    fn branch_name(&self, problem: BranchNameProblem) -> String {
        match problem {
            BranchNameProblem::Empty => "브랜치 이름이 필요합니다.".to_owned(),
            BranchNameProblem::TooLong => "이름이 너무 깁니다.".to_owned(),
            BranchNameProblem::Reserved(name) => {
                format!("\u{201c}{name}\u{201d}은(는) git이 받는 브랜치 이름이 아닙니다.")
            }
            BranchNameProblem::LeadingDash => "브랜치 이름은 대시(-)로 시작할 수 없습니다.".into(),
            BranchNameProblem::SpaceOrControl => {
                "브랜치 이름에 공백이나 제어 문자를 넣을 수 없습니다.".to_owned()
            }
            BranchNameProblem::Character(character) => {
                format!("브랜치 이름에 \u{201c}{character}\u{201d}을(를) 넣을 수 없습니다.")
            }
            BranchNameProblem::Sequence(sequence) => {
                format!("브랜치 이름에 \u{201c}{sequence}\u{201d}을(를) 넣을 수 없습니다.")
            }
            BranchNameProblem::Slashes => "브랜치 이름은 \u{201c}/\u{201d}로 시작하거나 끝날 수 없고, 두 개를 잇달아 쓸 수 없습니다.".to_owned(),
            BranchNameProblem::TrailingDot => "브랜치 이름은 점으로 끝날 수 없습니다.".to_owned(),
            BranchNameProblem::DotPart => {
                "브랜치 이름의 어느 부분도 점으로 시작할 수 없습니다.".to_owned()
            }
            BranchNameProblem::LockPart => {
                "브랜치 이름의 어느 부분도 \u{201c}.lock\u{201d}으로 끝날 수 없습니다.".to_owned()
            }
        }
    }

    fn place(&self, problem: PlaceProblem) -> String {
        match problem {
            PlaceProblem::NotOnMachine => "그 프로젝트는 이 머신에 없습니다.".to_owned(),
            PlaceProblem::StillCloning => "프로젝트를 아직 복제하는 중입니다.".to_owned(),
            PlaceProblem::CloneFailed => {
                "프로젝트 복제가 실패했습니다. 설정 \u{203a} 프로젝트에서 다시 복제하세요."
                    .to_owned()
            }
            PlaceProblem::FolderGone => "프로젝트 폴더가 없어졌습니다.".to_owned(),
            PlaceProblem::UnknownState => "프로젝트가 이 앱이 모르는 상태입니다.".to_owned(),
            PlaceProblem::NotGit => {
                "이 프로젝트는 git 저장소가 아닙니다. 폴더에서 실행하세요.".to_owned()
            }
            PlaceProblem::ChooseBranch => "실행할 브랜치를 고르세요.".to_owned(),
            PlaceProblem::BranchName(problem) => self.branch_name(problem),
        }
    }

    fn files(&self, count: u64) -> String {
        format!("파일 {count}개")
    }

    fn clone_stage(&self, stage: CloneStage) -> &'static str {
        match stage {
            CloneStage::Connecting => "연결 중",
            CloneStage::Counting => "객체 세는 중",
            CloneStage::Compressing => "객체 압축 중",
            CloneStage::Receiving => "객체 받는 중",
            CloneStage::Resolving => "델타 푸는 중",
            CloneStage::CheckingOut => "파일 꺼내는 중",
            CloneStage::Unknown => "복제 중",
        }
    }

    fn clone_failure(&self, reason: CloneFailure) -> &'static str {
        match reason {
            CloneFailure::NeedsCredential => {
                "원격 저장소가 자격 증명을 요구했습니다. 그 머신에서 로그인한 뒤(자격 증명 도우미나 SSH 키) 다시 복제하세요."
            }
            CloneFailure::NotFound => {
                "그 주소에서 저장소를 찾지 못했습니다. 주소를 확인하고 다시 복제하세요."
            }
            CloneFailure::Network => {
                "머신이 원격 저장소에 닿지 못했습니다. 네트워크를 확인하고 다시 복제하세요."
            }
            CloneFailure::TimedOut => "복제가 한 시간을 넘겨 멈췄습니다.",
            CloneFailure::Other | CloneFailure::Unknown => {
                "git이 복제하지 못했습니다. git이 남긴 말은 아래에 있습니다."
            }
        }
    }

    fn project_state(&self, state: &ProjectState) -> Option<String> {
        match state {
            ProjectState::Ready => None,
            ProjectState::Cloning { stage, percent, .. } => Some(match percent {
                None => self.clone_stage(*stage).to_owned(),
                Some(percent) => format!("{} \u{b7} {percent}%", self.clone_stage(*stage)),
            }),
            ProjectState::CloneFailed { .. } => Some("복제 실패".to_owned()),
            ProjectState::Missing => Some("폴더 없음".to_owned()),
            ProjectState::Unknown => Some("알 수 없는 상태".to_owned()),
        }
    }

    fn git_support(&self, git: &GitSupport) -> Option<String> {
        match git {
            GitSupport::Found { .. } | GitSupport::Unknown => None,
            GitSupport::TooOld { version, minimum, .. } => Some(format!(
                "이 머신의 git {}.{}은(는) 너무 오래됐습니다. 프로젝트를 쓰려면 {}.{} 이상이 필요합니다.",
                version.major, version.minor, minimum.major, minimum.minor
            )),
            GitSupport::Missing => Some(
                "이 머신에 git이 없습니다. 폴더를 추가할 수는 있지만 복제할 수 없고, 워크트리와 변경 사항도 쓸 수 없습니다."
                    .to_owned(),
            ),
        }
    }

    fn setup_failure(&self, failure: &SetupFailure, commands: &[&str]) -> String {
        match failure {
            SetupFailure::Exit { command, code, .. } => {
                let how = code.map_or_else(
                    || "신호를 받아 멈췄습니다".to_owned(),
                    |code| format!("코드 {code}로 끝났습니다"),
                );
                let named = usize::try_from(*command)
                    .ok()
                    .and_then(|index| commands.get(index));
                match named {
                    Some(named) => format!("`{named}`이(가) {how}."),
                    None => format!("셋업 명령 {}이(가) {how}.", command.saturating_add(1)),
                }
            }
            SetupFailure::TimedOut => "셋업이 30분 동안 돌아 멈췄습니다.".to_owned(),
            SetupFailure::Spawn => "머신이 셋업 명령(셸)을 시작하지 못했습니다.".to_owned(),
            SetupFailure::Unknown => "셋업이 실패했습니다.".to_owned(),
        }
    }

    fn setup_state(&self, setup: &SetupState) -> &'static str {
        match setup {
            SetupState::None => "셋업 없음",
            SetupState::Skipped => "셋업 건너뜀",
            SetupState::Running { .. } => "셋업 중",
            SetupState::Succeeded { .. } => "셋업 완료",
            SetupState::Failed { .. } => "셋업 실패",
            SetupState::Unknown => "알 수 없음",
        }
    }

    fn kept_reason(&self, reason: KeptReason) -> String {
        match reason {
            KeptReason::Uncommitted { files, .. } => {
                format!("커밋하지 않은 변경이 있는 파일 {files}개")
            }
            KeptReason::UniqueCommits { commits, .. } => {
                format!("다른 브랜치에 없는 커밋 {commits}개")
            }
            KeptReason::Operation { operation: op, .. } => {
                format!("{} 진행 중", self.operation(op))
            }
            KeptReason::Submodules => "변경을 검사하지 않는 서브모듈".to_owned(),
            KeptReason::FlaggedFiles => "git이 변경을 보여 주지 않도록 표시된 파일".to_owned(),
            KeptReason::WorktreeRefs => "이 워크트리만의 참조".to_owned(),
            KeptReason::InUse => "실행 중인 세션, 터미널이나 서비스, 또는 셋업이 쓰는 중".into(),
            KeptReason::InspectionFailed => "git이 검사하지 못함".to_owned(),
            KeptReason::Unknown => "이 앱이 모르는 이유".to_owned(),
        }
    }

    fn worktree_refusal(&self, reason: &WorktreeRefusal) -> String {
        match reason {
            WorktreeRefusal::NotGit => "이 프로젝트는 git 저장소가 아니어서 워크트리가 없습니다. 세션을 폴더에서 실행하세요.".to_owned(),
            WorktreeRefusal::NotReady => "프로젝트가 아직 준비되지 않았습니다(복제 중이거나 폴더가 없음).".to_owned(),
            WorktreeRefusal::NoCommit => "저장소에 아직 커밋이 없습니다. 먼저 폴더에서 한 번 커밋하세요.".to_owned(),
            WorktreeRefusal::BranchExists => "그 이름의 브랜치가 있습니다. 다른 이름을 고르거나 기존 브랜치에서 실행하세요.".to_owned(),
            WorktreeRefusal::CheckedOutElsewhere { path, .. } => format!(
                "그 브랜치는 다른 워크트리({})에 체크아웃되어 있습니다.",
                path.display()
            ),
            WorktreeRefusal::NoSuchBase => "기준 브랜치를 찾지 못했습니다. 다른 것을 고르거나 먼저 페치하세요.".to_owned(),
            WorktreeRefusal::NoSuchBranch => "그 브랜치를 찾지 못했습니다.".to_owned(),
            WorktreeRefusal::NoSuchSnapshot => "그 스냅숏은 없어졌습니다.".to_owned(),
            WorktreeRefusal::Limit => "이 프로젝트의 워크트리가 머신이 두는 최대 수에 닿았습니다. 먼저 하나를 지우세요.".to_owned(),
            WorktreeRefusal::Unknown => "머신이 이 앱이 모르는 이유로 워크트리를 만들지 않았습니다.".to_owned(),
        }
    }

    fn operation(&self, operation: GitOperation) -> &'static str {
        match operation {
            GitOperation::Merge => "병합",
            GitOperation::Rebase => "리베이스",
            GitOperation::CherryPick => "체리픽",
            GitOperation::Revert => "되돌리기",
            GitOperation::Bisect => "이분 탐색",
        }
    }

    fn refusal_lines(&self, refusal: &GitRefusal) -> (String, String) {
        match refusal {
            GitRefusal::NeedsCredential => lines(
                "원격 저장소가 자격 증명을 요구했습니다.",
                "그 머신에서 로그인한 뒤(묻지 않는 자격 증명 도우미나 SSH 키) 다시 시도하세요.",
            ),
            GitRefusal::TimedOut => lines(
                "git이 너무 오래 걸려 멈췄습니다.",
                "머신의 네트워크나 원격 저장소를 확인하고 다시 시도하세요.",
            ),
            GitRefusal::NonFastForward => lines(
                "원격에 이 브랜치에 없는 커밋이 있습니다.",
                "먼저 풀한 뒤 다시 푸시하세요. Vessel은 강제로 푸시하지 않습니다.",
            ),
            GitRefusal::Diverged => lines(
                "이 브랜치와 업스트림이 서로 다르게 나아갔습니다.",
                "풀은 빨리 감기만 합니다. 터미널에서 병합하거나 리베이스하거나, 에이전트에게 맡기세요.",
            ),
            GitRefusal::WouldOverwrite { .. } => {
                lines("로컬 변경이 덮어써집니다.", "먼저 커밋하거나 스태시하세요.")
            }
            GitRefusal::Conflicts { .. } => lines(
                "풀어야 할 충돌이 남았습니다.",
                "이 파일들에서 충돌을 푼 뒤 스테이지하고 커밋하세요.",
            ),
            GitRefusal::NothingToCommit => lines(
                "커밋할 것이 없습니다.",
                "먼저 변경을 스테이지하거나 모두 커밋하세요.",
            ),
            GitRefusal::NoIdentity => lines(
                "그 머신의 git에 커밋할 이름과 이메일이 없습니다.",
                "그 머신의 git 설정에 user.name과 user.email을 넣으세요.",
            ),
            GitRefusal::HookFailed { .. } => lines(
                "저장소의 훅이 거부했습니다.",
                "훅이 남긴 말을 읽고 고친 뒤 다시 시도하세요.",
            ),
            GitRefusal::NoUpstream => lines(
                "이 브랜치에는 업스트림이 없습니다.",
                "푸시하면 업스트림이 정해집니다.",
            ),
            GitRefusal::Detached => lines(
                "체크아웃된 브랜치가 없습니다.",
                "먼저 브랜치로 전환하거나 새로 만드세요.",
            ),
            GitRefusal::CheckedOutElsewhere { path, .. } => lines(
                "그 브랜치는 다른 워크트리에 체크아웃되어 있습니다.",
                format!(
                    "그곳({})에서 쓰거나 다른 브랜치를 고르세요.",
                    path.display()
                ),
            ),
            GitRefusal::NothingToStash => lines(
                "스태시할 것이 없습니다.",
                "스태시는 커밋하지 않은 변경을 보관하는데, 그런 변경이 없습니다.",
            ),
            GitRefusal::StashMoved => lines(
                "목록을 읽은 뒤 스태시가 바뀌었습니다.",
                "목록을 다시 보고 항목을 다시 고르세요.",
            ),
            GitRefusal::BranchExists => lines(
                "그 이름의 브랜치가 있습니다.",
                "다른 이름을 고르거나 그 브랜치로 전환하세요.",
            ),
            GitRefusal::NoSuchRef => lines(
                "그 이름의 브랜치, 태그, 커밋이 없습니다.",
                "이름을 확인하거나 먼저 페치하세요.",
            ),
            GitRefusal::Operation { operation, .. } => lines(
                format!("저장소가 {} 도중입니다.", self.operation(*operation)),
                "먼저 끝내거나 중단하세요.",
            ),
            GitRefusal::RemoteFailed { .. } => lines(
                "원격 저장소가 거부했습니다.",
                "아래에 원격이 남긴 말이 있습니다.",
            ),
            GitRefusal::RepositoryConfig { key, reason, .. } => config_lines(key.as_ref(), *reason),
            GitRefusal::Unknown => lines(
                "git이 이 앱이 모르는 이유로 거부했습니다.",
                "읽으려면 이 앱의 Vessel을 업데이트하세요.",
            ),
        }
    }

    fn action(&self, action: GitAction) -> &'static str {
        match action {
            GitAction::Commit => "커밋",
            GitAction::Push => "푸시",
            GitAction::Pull => "풀",
            GitAction::Fetch => "페치",
        }
    }

    fn blocked(&self, blocked: Blocked) -> &'static str {
        match blocked {
            Blocked::Clean => "워크트리가 깨끗합니다. 커밋하려면 먼저 변경하세요.",
            Blocked::DetachedPush => "분리된 HEAD입니다. 푸시하려면 브랜치를 체크아웃하세요.",
            Blocked::DetachedPull => "분리된 HEAD입니다. 풀하려면 브랜치를 체크아웃하세요.",
            Blocked::Diverged => "브랜치가 업스트림과 갈라졌습니다. 먼저 리베이스나 병합을 하세요.",
            Blocked::Behind => {
                "브랜치가 업스트림보다 뒤처졌습니다. 푸시 전에 풀이나 리베이스를 하세요."
            }
            Blocked::NothingToPush => "푸시할 로컬 커밋이 없습니다.",
            Blocked::NoUpstream => "이 브랜치에는 풀할 업스트림이 없습니다.",
        }
    }

    fn script_kind(&self, kind: ScriptKind) -> &'static str {
        super::script_words::kind(crate::Lang::Ko, kind)
    }

    fn script_port(&self, port: Option<ServicePort>) -> String {
        super::script_words::port(crate::Lang::Ko, port)
    }

    fn script_change(&self, was: &ScriptEntry, now: &ScriptEntry) -> String {
        super::script_words::change(crate::Lang::Ko, was, now)
    }

    fn compared(&self, compared: &Compared, local: &Local) -> String {
        super::comparison_words::ko(*self, compared, *local)
    }

    fn scope_label(&self, scope: &DiffScope) -> String {
        match scope {
            DiffScope::Base { .. } => "변경 사항".to_owned(),
            DiffScope::Uncommitted => "커밋 안 됨".to_owned(),
            DiffScope::Staged => "스테이지됨".to_owned(),
            DiffScope::Unstaged => "스테이지 안 됨".to_owned(),
            DiffScope::Commit { id, .. } => {
                let short: String = id.as_str().chars().take(7).collect();
                format!("커밋 {short}")
            }
        }
    }

    fn scope_detail(&self, kind: ScopeKind) -> &'static str {
        match kind {
            ScopeKind::Base => "기준 브랜치 이후의 모든 것",
            ScopeKind::Uncommitted => "아직 커밋하지 않음",
            ScopeKind::Staged => "다음 커밋에 들어갈 것",
            ScopeKind::Unstaged => "아직 스테이지하지 않음",
        }
    }

    fn scope_empty(&self, scope: &DiffScope) -> &'static str {
        match scope {
            DiffScope::Base { .. } => "기준과 비교해 변경이 없습니다.",
            DiffScope::Uncommitted | DiffScope::Unstaged => "HEAD와 비교해 변경이 없습니다.",
            DiffScope::Staged => "스테이지된 것이 없습니다.",
            DiffScope::Commit { .. } => "이 커밋은 아무것도 바꾸지 않습니다.",
        }
    }

    fn status_word(&self, status: ChangeStatus) -> &'static str {
        match status {
            ChangeStatus::Added => "추가됨",
            ChangeStatus::Deleted => "삭제됨",
            ChangeStatus::Modified => "수정됨",
            ChangeStatus::Renamed => "이름 바뀜",
            ChangeStatus::Copied => "복사됨",
            ChangeStatus::TypeChanged => "종류 바뀜",
            ChangeStatus::Conflicted => "충돌",
            ChangeStatus::Unknown => "바뀜",
        }
    }

    fn problem(&self, problem: &Problem) -> String {
        match problem {
            Problem::Node { message, .. } => message.clone(),
            Problem::Older => {
                "이 머신의 Vessel이 너무 오래돼 쓸 수 없습니다. 업데이트하세요.".to_owned()
            }
            // Not the transport's text (English, with the endpoint).
            Problem::Gone(_) => {
                "머신에 연결할 수 없습니다. 머신이 켜져 있고 노드가 실행 중인지 확인하세요."
                    .to_owned()
            }
            Problem::Unsent => "보내지 못했습니다. 다시 시도하세요.".to_owned(),
        }
    }

    fn save_failure(&self, failure: &SaveFailure) -> String {
        match failure {
            SaveFailure::NoGuard => "읽을 때 파일이 너무 커서 비교할 수 없었으므로, 덮어쓰지 않고는 저장할 수 없습니다. 다시 불러와 편집하세요.".to_owned(),
            SaveFailure::Call(problem) => self.problem(problem),
        }
    }

    fn labels(&self) -> &'static Labels {
        &labels::KO
    }

    fn projects_on(&self, machine: &str) -> String {
        format!("{machine}의 프로젝트")
    }

    fn no_projects_body(&self, machine: &str) -> String {
        format!("{machine}의 폴더를 추가하거나, 그곳에 저장소를 복제하거나, 새로 시작하세요.")
    }

    fn older_machine(&self, machine: &str) -> String {
        format!("{machine}의 Vessel은 프로젝트를 지원하지 않는 이전 버전입니다.")
    }

    fn forgot(&self, name: &str) -> String {
        format!("{name}을(를) 목록에서 지웠습니다. 파일은 그대로 있습니다")
    }

    fn not_removed(&self, name: &str, why: &str) -> String {
        format!("{name}을(를) 제거하지 못했습니다: {why}")
    }

    fn live(&self, count: usize) -> String {
        format!("실행 중 {count}개")
    }

    fn kept(&self, reason: &str) -> String {
        format!("남긴 이유: {reason}")
    }

    fn commands_title(&self, setup: usize, teardown: usize, scripts: usize) -> String {
        let lists = format!("셋업 {setup}개, 정리 {teardown}개");
        if scripts == 0 {
            lists
        } else {
            format!("{lists}, 스크립트·서비스 {scripts}개")
        }
    }

    fn invalid_commands(&self, message: &str) -> String {
        format!("vessel.toml을 읽을 수 없어 아무것도 실행하지 않습니다: {message}")
    }

    fn commands_not(&self, approved: bool, why: &str) -> String {
        let what = if approved {
            "승인하지"
        } else {
            "철회하지"
        };
        format!("명령을 {what} 못했습니다: {why}")
    }

    fn setup_not_started(&self, why: &str) -> String {
        format!("셋업을 시작하지 못했습니다: {why}")
    }

    fn removed(&self, branch: &str, snapshot: Option<&str>) -> String {
        match snapshot {
            None => format!("{branch}을(를) 제거했습니다"),
            Some(commit) => {
                format!("{branch}을(를) 제거했습니다. 작업은 스냅숏 {commit}에 남아 있습니다")
            }
        }
    }

    fn remove_title(&self, branch: &str) -> String {
        format!("\"{branch}\"을(를) 제거할까요?")
    }

    fn discard_says(&self, path: &str, branch: &str) -> String {
        format!(
            "버리면 {path} 폴더와 커밋하지 않은 변경, 브랜치 {branch}가 지워지고, 다른 브랜치에 없는 커밋은 사라집니다."
        )
    }

    fn snapshot_not_dropped(&self, name: &str, why: &str) -> String {
        format!("{name}의 스냅숏을 삭제하지 못했습니다: {why}")
    }

    fn add_hint(&self, form: AddForm, machine: &str, clone_target: Option<(&str, &str)>) -> String {
        match (form, clone_target) {
            (AddForm::Clone, None) => {
                format!("{machine}의 폴더입니다. 저장소는 그 안의 폴더에 들어갑니다.")
            }
            (AddForm::Clone, Some((folder, name))) => {
                format!("저장소는 {folder}/{name}에 만들어집니다.")
            }
            (AddForm::New, _) => {
                format!("{machine}의 비어 있거나 새 폴더입니다. git이 그곳에 저장소를 시작합니다.")
            }
            (AddForm::Folder, _) => format!(
                "{machine}의 절대 경로 폴더입니다. 저장소 안이면 저장소의 최상위가 프로젝트가 됩니다."
            ),
        }
    }

    fn approval_copy(&self, place: &str, scripts: bool) -> String {
        let started = if scripts {
            " 스크립트와 서비스는 누군가 시작할 때 실행됩니다."
        } else {
            ""
        };
        format!(
            "저장소의 vessel.toml에 이 명령들이 있습니다. 셋업과 정리는 {place} 내 계정과 환경으로 실행됩니다.{started} 승인은 정확히 이 세트에만 해당하며, 명령이 하나라도 바뀌면 다시 묻습니다."
        )
    }

    fn approval_place(&self, project: &str, machine: &str, each: bool) -> String {
        if each {
            format!("{machine}에 있는 {project}의 새 워크트리마다")
        } else {
            format!("{machine}에 있는 {project}의 새 워크트리에서")
        }
    }

    fn add_lines(&self, first: usize, last: usize) -> String {
        if first == last {
            format!("{first}번 줄 채팅에 추가")
        } else {
            format!("{first}\u{2013}{last}번 줄 채팅에 추가")
        }
    }

    fn editor_lines(&self, count: usize) -> String {
        format!("{count}줄")
    }

    fn found_count(&self, files: usize, searched: u32, complete: bool, capped: bool) -> String {
        if capped {
            format!("가장 가까운 파일 {files}개만 보입니다. 이름을 더 입력해 좁혀 보세요")
        } else if complete {
            format!("파일 {files}개")
        } else {
            format!("파일 {files}개 · 앞쪽 파일 {searched}개만 검색했습니다")
        }
    }

    fn grepped_count(&self, lines: usize, files: usize, more: bool) -> String {
        if more {
            format!("처음 {lines}줄만 보입니다(파일 {files}개). 더 있으니 검색어를 좁혀 보세요")
        } else {
            format!("파일 {files}개에서 {lines}줄")
        }
    }

    fn digest(&self, digest: &str) -> String {
        format!("다이제스트 {digest}")
    }

    fn was(&self, command: &str) -> String {
        format!("이전: {command}")
    }
}
