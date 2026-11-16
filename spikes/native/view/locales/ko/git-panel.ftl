## Git 비교 화면. 오류의 기술적인 세부 내용은 별도의 코드 영역에 표시합니다.

# Title of the Changes pane. Keep short.
git-panel-title = 변경 사항
# Accessible label of the comparison mode picker. Keep short.
git-panel-mode = Diff 모드
# Comparison choice: merge base against working files, including new files. Keep short.
git-panel-mode-all = 모든 변경 사항
# Comparison choice: HEAD against working files. Keep short.
git-panel-mode-uncommitted = 커밋되지 않음
# Comparison choice: merge base against HEAD only. Keep short.
git-panel-mode-committed = 커밋됨
# Tooltip and accessible name of the diff options trigger. Keep short.
git-panel-options = Diff 옵션
# Menu choice that shows diff bodies directly inside Changes. Keep short.
git-panel-inline = 인라인 Diff
# Layout choice for one column with both sides of a change. Keep short.
git-panel-unified = 통합 diff
# Layout choice for old and new text in two columns. Keep short.
git-panel-split = 나란히 보기 diff
# Menu action enabling source-line wrapping. Keep short.
git-panel-wrap = 긴 줄 줄바꿈
# Menu action disabling wrapping and restoring horizontal scrolling. Keep short.
git-panel-no-wrap = 긴 줄 스크롤
# Menu action that omits whitespace-only changes. Keep short.
git-panel-hide-whitespace = 공백 숨기기
# Menu action that restores whitespace-only changes. Keep short.
git-panel-show-whitespace = 공백 표시
# Menu action expanding every file body. Keep short.
git-panel-expand-files = 모든 파일 확장
# Menu action folding every file body. Keep short.
git-panel-collapse-files = 모든 파일 축소
# Menu action expanding every changed-file folder. Keep short.
git-panel-expand-folders = 모든 폴더 확장
# Menu action folding every changed-file folder. Keep short.
git-panel-collapse-folders = 모든 폴더 축소
# Button label for rereading git facts; it does not fetch remotes. Keep short.
git-panel-refresh = 새로고침
# Refresh button label while rereading facts. Keep short.
git-panel-refreshing = 새로고침 중
# Title of the compact sheet that chooses a file to scroll to. Keep short.
git-panel-jump = 파일로 이동
# Status while the first repository status read is pending.
git-panel-checking = 저장소 확인 중...
# Empty state only when authoritative project facts say this is not a repository.
git-panel-not-repository = Git 저장소가 아닙니다
# Empty state after a complete comparison contains no changed files.
git-panel-empty = 표시할 변경 사항이 없습니다
# Empty-state action switching from Committed to Uncommitted. Keep short.
git-panel-see-uncommitted = 커밋되지 않은 변경 사항 보기
# Empty-state action switching from Uncommitted to Committed. Keep short.
git-panel-see-committed = 커밋된 변경 사항 보기
# Heading when the bounded diff document cannot be previewed.
git-panel-too-large-title = Diff가 너무 커서 미리 볼 수 없습니다
# Next step beneath the too-large preview heading.
git-panel-too-large-description = 여기에서 미리 보려면 비교 범위를 좁히세요.
# Placeholder for a binary file body. Keep short.
git-panel-binary = 바이너리 파일
# Heading of the collapsible branch commit section. Keep short.
git-panel-commits = 커밋
# Accessible description of the history loading skeleton.
git-panel-commits-loading = 커밋 로드 중…
# Empty state for a complete empty commit listing.
git-panel-commits-empty = 아직 커밋이 없습니다.
# 오류의 원인과 다음 행동을 하나의 문장으로 유지합니다.
# Complete recovery after the app could not enqueue a read; nothing reached the machine.
git-panel-problem-unsent = 보내지 못했습니다. 다시 시도하세요.
# Complete recovery after losing the machine connection.
git-panel-problem-connection = 머신의 연결이 끊겼습니다. 다시 연결하고 변경 사항을 새로고침하세요.
# Complete recovery after a read grant was refused.
git-panel-problem-access = 이 기기에서 머신의 변경 사항을 읽을 수 없습니다. 접근 권한을 확인하세요.
# Complete recovery for an older or unassigned protocol method.
git-panel-problem-version = 변경 사항을 보려면 이 기기와 머신의 Vessel을 업데이트하세요.
# Complete recovery when the registered folder no longer exists.
git-panel-problem-missing = 이 폴더를 더 이상 사용할 수 없습니다. 다른 워크스페이스를 여세요.
# Complete recovery for a transient read refusal or timeout.
git-panel-problem-unavailable = 지금은 머신에서 변경 사항을 읽을 수 없습니다. 새로고침하고 다시 시도하세요.
# Complete recovery for an otherwise unclassified read failure; diagnostic is separate.
git-panel-problem-failed = 머신에서 변경 사항을 읽지 못했습니다. 새로고침하고 다시 시도하세요.
# Complete recovery when the machine cannot provide this read; this does not say the folder lacks git.
git-panel-problem-unsupported = 이 머신에서 변경 사항을 제공할 수 없습니다. 머신의 Git과 Vessel을 확인하고 새로고침하세요.
# Alert heading for a failed status or comparison read; recovery and git output follow separately.
git-panel-failed-title = 변경 사항을 읽지 못했습니다

# Complete recovery when the selected authenticated connection changed; never choose another alias.
git-panel-problem-destination = 연결이 변경되었습니다. 이 워크스페이스를 다시 열고 변경 사항을 새로고침하세요.
# Complete recovery when a file body names a different path; never install it under the requested file.
git-panel-problem-file = 머신에서 다른 파일의 변경 사항을 보냈습니다. 새로고침하고 다시 시도하세요.

# Neutral recovery for different stored/requested base refs; both identities are separate code detail.
git-panel-problem-base = 저장된 기준 브랜치와 요청한 기준 브랜치가 다릅니다. 두 브랜치를 확인하고 비교 기준을 다시 선택하세요.

# Brief commit acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-commit-success = 커밋됨

# Brief push acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-push-success = 푸시됨

# Brief pull acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-pull-success = 가져옴

# Brief fetch acknowledgement inside the same git menu row; exact facts remain in the success notice.
git-panel-fetch-success = 원격 변경 가져옴

# 앱의 동시 읽기 한도에 도달해 노드 요청 전 거절됐을 때의 완전한 복구 문장.
git-panel-problem-capacity = 한 번에 너무 많은 변경을 읽고 있습니다. 다시 시도하세요.
# 읽기 제한 시간이 지났을 때의 완전한 복구 문장. 연결이 끊겼다고 단정하지 않습니다.
git-panel-problem-timeout = 읽기를 제시간에 마치지 못했습니다. 다시 시도하세요.

# Complete guidance when the canonical workspace root has not been admitted; do not infer non-Git.
git-panel-problem-workspace = 워크스페이스의 저장소 정보를 사용할 수 없습니다. 등록된 워크스페이스를 열고 다시 시도하세요.
# Complete recovery when lifetime correlation identities are exhausted; retry alone cannot help.
git-panel-problem-keys = 요청을 안전하게 구분할 수 없습니다. Vessel을 다시 시작하고 시도하세요.
# Complete recovery when this screen's retirement owner has not initialized; no request is sent.
git-panel-problem-owner = 이 화면에서 저장소 변경을 안전하게 추적할 수 없습니다. Vessel을 다시 시작하고 워크스페이스를 다시 여세요.

# Honest notice when the node marks the summary incomplete; zero listed files is not an empty comparison.
git-panel-partial = 목록에 없는 변경 파일이 있습니다. 새로고침하거나 비교 범위를 줄이세요.

# Label before the exact original identity in a diagnostic code block.
git-panel-detail-expected = 요청한 값

# Label before the exact peer identity in a diagnostic code block.
git-panel-detail-received = 받은 값

# Label before the exact recorded worktree base ref; no particle follows its name.
git-panel-detail-stored-base = 저장된 기준 브랜치

# Label before the exact requested branch or commit base in a diagnostic code block.
git-panel-detail-requested-base = 요청한 비교 기준

# Complete recovery when a typed comparison has no registered or explicit base; never claim it is empty.
git-panel-problem-no-base = 비교할 기준 브랜치가 없습니다. 기준 브랜치를 선택한 뒤 다시 비교하세요.

# Complete recovery when the canonical comparison exceeds its output limit, not an empty successful result.
git-panel-problem-too-large = 비교 결과가 너무 커서 미리 볼 수 없습니다. 비교 범위를 줄인 뒤 다시 시도하세요.

# Advisory toast when the drawn legacy repository has no original authenticated read receipt; no write was attempted.
git-panel-unverified-view = 여기에 표시된 저장소를 확인할 수 없습니다. 변경하기 전에 워크스페이스를 다시 여세요.

# Complete recovery when a file continuation has changed metadata or invalid hunk order; never silently append it.
git-panel-problem-page = 받은 변경 사항이 서로 맞지 않습니다. 새로고침하고 다시 시도하세요.

# File preview has no accepted body yet; do not claim an active node request.
git-panel-body-waiting = 파일 변경 사항을 아직 불러오지 않았습니다.
# The node cut a source line or a hunk. Keep this warning beside the preserved preview.
git-panel-truncated = 머신이 일부 변경 사항을 잘라 보냈습니다.
# Local readonly-buffer admission failed; raw typed cause is shown separately.
git-panel-invalid-document = 변경 사항을 표시하지 못했습니다. 새로고침하고 다시 시도하세요.
