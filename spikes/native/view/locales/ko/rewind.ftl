## Rewind from a person's message. Reference: Paseo's rewind menu and
## use-rewind-agent-mutation; Vessel rewinds the conversation, not files.

# The person's rewind mark tooltip. Keep short.
rewind-tooltip = 이 메시지로 되감기
# Noninteractive warning at the top of its menu.
rewind-warning = 이 작업은 되돌릴 수 없습니다
# The one mode supported by the node. Keep short.
rewind-conversation = 대화 되감기
# The person's message copy action. Keep short.
rewind-copy-message = 메시지 복사
# Frame notice after refusal. $kind distinguishes absent/present diagnostic;
# $message is the node's existing diagnostic. Its typed replacement is separate.
rewind-failed = { $kind ->
    [detail] 에이전트를 되감지 못했습니다: { $message }
   *[plain] 에이전트를 되감지 못했습니다
    }
