// Workspace tab chrome uses the browser language, as account usage does.
// Keep the compact and desktop actions on the same vocabulary.

const en = {
  workspace: "Workspace tabs",
  agent: "Agent",
  terminal: "Terminal",
  terminalNeedsAgent:
    "Unarchive an agent or start a new one in this workspace to open a terminal.",
  changes: "Changes",
  diff: "Diff",
  files: "Files",
  newTab: "New tab",
  newAgent: "New agent",
  newTerminal: "New terminal",
  actions: "Tab actions",
  close: "Close",
  cancel: "Cancel",
  archive: "Archive",
  rename: "Rename",
  name: "Name",
  copyAgent: "Copy agent ID",
  copyTerminal: "Copy terminal id",
  copyPath: "Copy file path",
  copyResume: "Copy resume command",
  reload: "Reload agent",
  reloadHint: "Reload agent to update skills, MCPs or login status.",
  before: "Close tabs to the left",
  after: "Close tabs to the right",
  above: "Close tabs above",
  below: "Close tabs below",
  others: "Close other tabs",
  terminalTitle: "Close terminal?",
  terminalBody:
    "Any running process in this terminal will be stopped immediately.",
  archiveAgentTitle: "Archive agent?",
  agentTitle: "Archive running agent?",
  agentBody: "The agent is running. Closing this tab will archive the agent.",
  fileTitle: "Close unsaved tab?",
  leaveTitle: "Leave this workspace?",
  leave: "Leave",
  fileBody: "Unsaved changes will be lost.",
  disconnected: "Host is not connected",
  terminalFailed: "Unable to subscribe to terminal",
  terminalMissing: "Terminal not found",
  terminalExited: "Terminal exited",
  loading: "Loading",
  dropped: "Keys dropped: the terminal is not reading its input.",
  unavailable: "This tab is unavailable.",
  copied: "Copied",
  switcher: (count: number) => `Switch tabs (${count} open)`,
  bulk: (agents: number, terminals: number, tabs: number, dirty: number) => {
    const work = [
      agents === 0 ? "" : `archive ${agents} agent(s)`,
      terminals === 0 ? "" : `close ${terminals} terminal(s)`,
      tabs === 0 ? "" : `close ${tabs} tab(s)`,
    ].filter(Boolean);
    return (
      `This will ${new Intl.ListFormat("en").format(work)}.` +
      (terminals === 0
        ? ""
        : " Any running process in a closed terminal will be stopped immediately.") +
      (dirty === 0 ? "" : `\n\n${dirty} tab(s) have unsaved changes.`)
    );
  },
};

const ko: typeof en = {
  workspace: "워크스페이스 탭",
  agent: "에이전트",
  terminal: "터미널",
  terminalNeedsAgent:
    "터미널을 열려면 이 워크스페이스에서 에이전트 보관을 해제하거나 새 에이전트를 시작하세요.",
  changes: "변경 사항",
  diff: "비교",
  files: "파일",
  newTab: "새 탭",
  newAgent: "새 에이전트",
  newTerminal: "새 터미널",
  actions: "탭 작업",
  close: "닫기",
  cancel: "취소",
  archive: "보관",
  rename: "이름 변경",
  name: "이름",
  copyAgent: "에이전트 ID 복사",
  copyTerminal: "터미널 ID 복사",
  copyPath: "파일 경로 복사",
  copyResume: "재개 명령 복사",
  reload: "에이전트 다시 불러오기",
  reloadHint: "스킬, MCP 또는 로그인 상태를 갱신합니다.",
  before: "왼쪽 탭 닫기",
  after: "오른쪽 탭 닫기",
  above: "위쪽 탭 닫기",
  below: "아래쪽 탭 닫기",
  others: "다른 탭 닫기",
  terminalTitle: "터미널을 닫을까요?",
  terminalBody: "이 터미널에서 실행 중인 프로세스가 즉시 중지됩니다.",
  archiveAgentTitle: "에이전트를 보관할까요?",
  agentTitle: "실행 중인 에이전트를 보관할까요?",
  agentBody: "에이전트가 실행 중입니다. 이 탭을 닫으면 에이전트가 보관됩니다.",
  fileTitle: "저장하지 않은 탭을 닫을까요?",
  leaveTitle: "이 작업 공간을 나갈까요?",
  leave: "나가기",
  fileBody: "저장하지 않은 변경 사항이 사라집니다.",
  disconnected: "호스트에 연결되어 있지 않습니다",
  terminalFailed: "터미널을 구독할 수 없습니다",
  terminalMissing: "터미널을 찾을 수 없습니다",
  terminalExited: "터미널이 종료되었습니다",
  loading: "불러오는 중",
  dropped: "터미널이 입력을 읽지 않아 키 입력이 누락되었습니다.",
  unavailable: "이 탭을 사용할 수 없습니다.",
  copied: "복사됨",
  switcher: (count) => `탭 전환 (열린 탭 ${count}개)`,
  bulk: (agents, terminals, tabs, dirty) => {
    const work = [
      agents === 0 ? "" : `에이전트 ${agents}개 보관`,
      terminals === 0 ? "" : `터미널 ${terminals}개 닫기`,
      tabs === 0 ? "" : `탭 ${tabs}개 닫기`,
    ].filter(Boolean);
    return (
      `${new Intl.ListFormat("ko").format(work)} 작업을 진행합니다.` +
      (terminals === 0
        ? ""
        : " 닫힌 터미널에서 실행 중인 프로세스가 즉시 중지됩니다.") +
      (dirty === 0
        ? ""
        : `\n\n탭 ${dirty}개에 저장하지 않은 변경 사항이 있습니다.`)
    );
  },
};

export type TabWords = typeof en;
export function tabWords(language: string): TabWords {
  return /^ko(?:-|$)/i.test(language) ? ko : en;
}
