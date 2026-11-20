// The pull request pane's words, in English and Korean: the `pulls-`
// messages of the native app's Fluent catalog
// (view/locales/{en,ko}/pulls.ftl) that the native pane draws
// (app/src/areas/pulls/pane*.rs), which keep Paseo's words
// (i18n/resources/{en,ko}.ts: workspace.git.pr, checksOverview,
// panels.pullRequest). The git menu's, the form's, the merge's and the
// sidebar card's messages are other features and are not here.
//
// A count is Fluent's `NUMBER($count)`: digits grouped by the language
// ("1,234"); a plural is English's one/other, Korean has none. A host or
// a forge's name is a value in a sentence the translator owns; Korean
// picks its particle from the value (particle.ts). Words for the agent
// ("Add to chat"'s text) are English only and live in attach.ts.

import { particle } from "./particle";

const EN_NUMBER = new Intl.NumberFormat("en");
const KO_NUMBER = new Intl.NumberFormat("ko");

/** `count` as English writes it. */
function n(count: number): string {
  return EN_NUMBER.format(count);
}

/** `one` when `count` is one, `other` otherwise (English's plural). */
function plural(count: number, one: string, other: string): string {
  return count === 1 ? one : other;
}

/** `count` as Korean writes it. */
function k(count: number): string {
  return KO_NUMBER.format(count);
}

/** `host` and its subject particle (이/가). */
function subject(host: string): string {
  return `${host}${particle(host, "subject")}`;
}

const en = {
  // The pane's frame.
  checks: "Checks",
  activity: "Activity",
  paneView: "View",
  openOn: (host: string) => `Open on ${host}`,
  refreshState: (forge: string) => `Refresh git and ${forge} state`,
  refreshing: "Refreshing",
  paneFailed: "Failed to refresh git state.",
  paneRetry: "Retry",
  emptyTitle: "No pull request yet",
  emptyDescription:
    "Create a pull request for this checkout to see its details here.",
  // The state beside its icon.
  stateOpen: "Open",
  stateDraft: "Draft",
  stateMerged: "Merged",
  stateClosed: "Closed",
  stateUnknown: "Unknown",
  // The checks' headline, counts, detail line, groups and rows.
  checksFailure: "Some checks were not successful",
  checksPending: "Some checks haven't completed yet",
  checksSuccess: "All checks have passed",
  checksNone: "No checks",
  checksUnknown: "Checks haven't been read yet",
  countFailing: (count: number) => `${n(count)} failing`,
  countPending: (count: number) => `${n(count)} in progress`,
  countPassed: (count: number) => `${n(count)} successful`,
  countSkipped: (count: number) => `${n(count)} skipped`,
  checksDetail: (parts: string, total: number) =>
    `${parts} ${plural(total, "check", "checks")}`,
  groupFailing: (count: number) =>
    `${n(count)} failing ${plural(count, "check", "checks")}`,
  groupPending: (count: number) =>
    `${n(count)} in progress ${plural(count, "check", "checks")}`,
  groupPassed: (count: number) =>
    `${n(count)} successful ${plural(count, "check", "checks")}`,
  groupSkipped: (count: number) =>
    `${n(count)} skipped ${plural(count, "check", "checks")}`,
  checkRunning: (duration: string) => `running ${duration}`,
  // The review standing and the activity pills.
  approved: "Approved",
  changesRequested: "Changes requested",
  reviewRequired: "Review required",
  unresolved: (count: number) =>
    `${n(count)} unresolved ${plural(count, "thread", "threads")}`,
  mergeReady: "Ready to merge",
  mergeConflicts: "Conflicts with its base",
  mergeBlocked: "Blocked by a rule",
  mergeUnstable: "Checks aren't all passing",
  mergeBehind: "Behind its base",
  mergeUnknown: "Not known yet",
  // Why there is none (the empty pane).
  noBranch: "Create PR isn't available because you aren't on a branch",
  noRemote:
    "Create PR isn't available because this branch has no GitHub remote",
  noForge: (host: string) =>
    `Create PR isn't available right now because ${host} isn't connected`,
  signedOut: (host: string) =>
    `Create PR isn't available because this machine isn't signed in to ${host}. Run gh auth login on the machine.`,
  refused: (host: string) =>
    `Create PR isn't available because ${host} refused this machine's sign-in`,
  rateLimited: (host: string) =>
    `Create PR isn't available right now because ${host} is limiting requests. Try again later.`,
  unreachable: (host: string) =>
    `Create PR isn't available right now because ${host} can't be reached`,
  limit:
    "Create PR isn't available because this machine is watching as many pull requests as it can",
  ended: "Create PR isn't available because this session has ended",
  unavailable: "Create PR isn't available right now",
  // The activity's cards.
  noActivity: "No activity yet",
  verbCommented: "commented",
  verbApproved: "approved",
  verbChangesRequested: "requested changes",
  verbReviewed: "reviewed",
  threadDiscussion: "Discussion thread",
  threadResolved: "Resolved",
  threadOutdated: "Outdated",
  unknownAuthor: "unknown",
  ageNow: "just now",
  ageMinutes: (count: number) => `${n(count)}m ago`,
  ageHours: (count: number) => `${n(count)}h ago`,
  ageDays: (count: number) => `${n(count)}d ago`,
  ageMonths: (count: number) => `${n(count)}mo ago`,
  ageYears: (count: number) => `${n(count)}y ago`,
  // The cards' actions.
  addToChat: "Add to chat",
  addingToChat: "Adding...",
  addAllToChat: "Add all to chat",
  copy: "Copy",
  commentActions: "Comment actions",
  threadActions: "Thread actions",
};

const ko: typeof en = {
  checks: "검사",
  activity: "활동",
  paneView: "보기",
  openOn: (host) => `${host}에서 열기`,
  refreshState: (forge) => `Git 및 ${forge} 상태 새로고침`,
  refreshing: "새로고침 중",
  paneFailed: "git 상태를 새로 고치지 못했습니다.",
  paneRetry: "다시 시도",
  emptyTitle: "아직 풀 리퀘스트가 없습니다",
  emptyDescription:
    "이 체크아웃에 풀 리퀘스트를 만들면 세부 정보가 여기에 표시됩니다.",
  stateOpen: "열림",
  stateDraft: "초안",
  stateMerged: "병합됨",
  stateClosed: "닫힘",
  stateUnknown: "알 수 없음",
  checksFailure: "실패한 검사가 있습니다",
  checksPending: "아직 완료되지 않은 검사가 있습니다",
  checksSuccess: "모든 검사를 통과했습니다",
  checksNone: "검사 없음",
  checksUnknown: "검사를 아직 읽지 않았습니다",
  countFailing: (count) => `${k(count)}개 실패`,
  countPending: (count) => `${k(count)}개 진행 중`,
  countPassed: (count) => `${k(count)}개 성공`,
  countSkipped: (count) => `${k(count)}개 건너뜀`,
  checksDetail: (parts, total) => `검사 ${k(total)}개: ${parts}`,
  groupFailing: (count) => `실패한 검사 ${k(count)}개`,
  groupPending: (count) => `진행 중인 검사 ${k(count)}개`,
  groupPassed: (count) => `성공한 검사 ${k(count)}개`,
  groupSkipped: (count) => `건너뛴 검사 ${k(count)}개`,
  checkRunning: (duration) => `${duration} 실행 중`,
  approved: "승인됨",
  changesRequested: "변경 요청됨",
  reviewRequired: "리뷰 필요",
  unresolved: (count) => `해결되지 않은 스레드 ${k(count)}개`,
  mergeReady: "병합 가능",
  mergeConflicts: "기준 브랜치와 충돌",
  mergeBlocked: "규칙에 막힘",
  mergeUnstable: "통과하지 않은 검사 있음",
  mergeBehind: "기준 브랜치보다 뒤처짐",
  mergeUnknown: "아직 모름",
  noBranch: "브랜치에 있지 않아 PR 생성을 사용할 수 없습니다",
  noRemote: "이 브랜치에 GitHub 원격이 없어 PR 생성을 사용할 수 없습니다",
  noForge: (host) =>
    `${subject(host)} 연결되어 있지 않기 때문에 지금은 PR 생성을 사용할 수 없습니다`,
  signedOut: (host) =>
    `이 머신이 ${host}에 로그인되어 있지 않아 PR 생성을 사용할 수 없습니다. 머신에서 gh auth login을 실행하세요.`,
  refused: (host) =>
    `${subject(host)} 이 머신의 로그인을 거부해 PR 생성을 사용할 수 없습니다`,
  rateLimited: (host) =>
    `${subject(host)} 요청을 제한하고 있어 지금은 PR 생성을 사용할 수 없습니다. 나중에 다시 시도하세요.`,
  unreachable: (host) =>
    `${host}에 연결할 수 없어 지금은 PR 생성을 사용할 수 없습니다`,
  limit:
    "이 머신이 지켜볼 수 있는 풀 리퀘스트 수가 다 차서 PR 생성을 사용할 수 없습니다",
  ended: "이 세션이 끝나서 PR 생성을 사용할 수 없습니다",
  unavailable: "지금은 PR 생성을 사용할 수 없습니다",
  noActivity: "아직 활동이 없습니다",
  verbCommented: "댓글 남김",
  verbApproved: "승인함",
  verbChangesRequested: "변경 요청함",
  verbReviewed: "리뷰함",
  threadDiscussion: "토론 스레드",
  threadResolved: "해결됨",
  threadOutdated: "오래됨",
  unknownAuthor: "알 수 없음",
  ageNow: "방금",
  ageMinutes: (count) => `${k(count)}분 전`,
  ageHours: (count) => `${k(count)}시간 전`,
  ageDays: (count) => `${k(count)}일 전`,
  ageMonths: (count) => `${k(count)}개월 전`,
  ageYears: (count) => `${k(count)}년 전`,
  addToChat: "채팅에 추가",
  addingToChat: "추가하는 중...",
  addAllToChat: "모두 채팅에 추가",
  copy: "복사",
  commentActions: "댓글 작업",
  threadActions: "스레드 작업",
};

export type PullWords = typeof en;

/** The pane's words for `language` (a BCP 47 tag): Korean for `ko` and
 * its regions, English otherwise. */
export function pullWords(language: string): PullWords {
  return /^ko(?:-|$)/i.test(language) ? ko : en;
}
