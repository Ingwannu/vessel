## 대화 분기. Paseo의 assistant-fork-menu와 use-fork-agent를 참고합니다.
## Vessel은 초안 탭 대신 원본 옆에 새 세션을 만듭니다.

# 답변 아래 분기 버튼의 툴팁. 짧게 씁니다.
fork-tooltip = 여기에서 채팅 분기
# 노드가 지원하는 한 가지 분기 대상. 짧게 씁니다.
fork-in-new-session = 새 세션으로 분기
# 대화가 시작되지 않은 분기의 안내 제목.
fork-empty-heading = 분기가 시작되지 않았습니다
# 에이전트가 고른 지점을 무시하여 이어 갈 수 없습니다.
fork-empty-not-at-point = 에이전트가 고른 지점에서 분기하지 않았습니다. 여기서는 이어 갈 수 없으니 삭제하고 다시 분기하세요.
# 분기를 지원하지 않아 이어 갈 수 없습니다.
fork-empty-cannot-fork = 이 에이전트는 대화를 분기할 수 없습니다. 여기서는 이어 갈 수 없습니다.
# 원본은 남겨 두고 빈 분기만 삭제합니다. 짧게 씁니다.
fork-remove = 삭제
# 분기의 첫 줄. title은 원본 세션의 제목입니다.
fork-from-title = “{ $title }”에서 분기함
# 원본 제목을 모르거나 제목이 비어 있을 때의 첫 줄.
fork-from-other = 다른 세션에서 분기함

# Fork refusal recovery: Point. Diagnostic detail stays in logs.
fork-problem-point = 고른 지점에서 이 대화를 분기할 수 없습니다. 다른 턴을 골라 다시 시도하세요.

# Fork refusal recovery: Access. Diagnostic detail stays in logs.
fork-problem-access = 이 기기에는 이 대화를 분기할 권한이 없습니다.

# Fork refusal recovery: Version. Diagnostic detail stays in logs.
fork-problem-version = 분기 전에 이 기기와 머신의 Vessel을 업데이트하세요.

# Fork refusal recovery: Missing. Diagnostic detail stays in logs.
fork-problem-missing = 이 대화를 더 이상 사용할 수 없습니다.

# Fork refusal recovery: not ready. Diagnostic detail stays in logs.
fork-problem-not-ready = 지금은 이 대화를 분기할 수 없습니다. 다시 열고 시도하세요.

# Fork refusal recovery: Unsupported. Diagnostic detail stays in logs.
fork-problem-unsupported = 이 에이전트는 대화를 분기할 수 없습니다.

# Fork refusal recovery: missing agent. Diagnostic detail stays in logs.
fork-problem-agent-missing = 이 대화의 에이전트가 머신에 설치되어 있지 않습니다.

# Fork refusal recovery: Start. Diagnostic detail stays in logs.
fork-problem-start = 대화를 분기할 에이전트를 시작하지 못했습니다. 다시 시도하세요.

# Fork refusal recovery: Unavailable. Diagnostic detail stays in logs.
fork-problem-unavailable = 머신이 지금 이 대화를 분기할 수 없습니다. 나중에 다시 시도하세요.

# Fork refusal recovery: Failed. Diagnostic detail stays in logs.
fork-failed = 분기 결과를 확인하지 못했습니다. 다시 시도하기 전에 세션 목록을 확인하세요.

# Origin read recovery: Removed. Retrying never clears the fork.
fork-origin-removed = 분기한 원래 대화는 삭제되었습니다.

# Origin read recovery: Access. Retrying never clears the fork.
fork-origin-access = 이 기기에는 분기한 원래 대화를 읽을 권한이 없습니다.

# Origin read recovery: Version. Retrying never clears the fork.
fork-origin-version = 분기한 원래 대화를 읽으려면 이 기기와 머신의 Vessel을 업데이트하세요.

# Origin read recovery: Unavailable. Retrying never clears the fork.
fork-origin-unavailable = 머신에 연결할 수 없습니다. 세션을 다시 열어 분기한 원래 대화를 읽어 보세요.

# Origin read recovery: Failed. Retrying never clears the fork.
fork-origin-failed = 분기한 원래 대화를 읽지 못했습니다. 세션을 다시 열면 다시 읽습니다.

# Unknown empty-fork failure. The original diagnostic stays in session state.
fork-empty-other = 에이전트가 분기를 시작하지 못했습니다. 여기서는 이어 갈 수 없으니 삭제하고 다시 분기하세요.
