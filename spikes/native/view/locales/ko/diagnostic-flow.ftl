# Diagnostics collection framing. No private values are interpolated.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-client = 클라이언트
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-desktop = 데스크톱
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-hosts = 머신
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-pending = 대기 중
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-running = 수집 중
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-done = 수집 완료
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-offline = 머신이 연결되어 있지 않습니다. 다시 연결한 뒤 새로 고침하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-busy = 진단이 진행 중입니다. 현재 조회가 끝나면 다시 시도하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-unavailable = 이 버전에서는 이 진단 정보를 제공하지 않습니다.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-unsupported = 이 머신은 이 진단을 지원하지 않습니다.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-forbidden = 이 기기에서는 이 진단을 조회할 수 없습니다. 머신 접근 권한을 확인하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-timed-out = 진단 시간이 초과되었습니다. 새로 고침하여 다시 시도하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-invalid-reply = 머신에서 올바르지 않은 진단 정보를 받았습니다. 다시 연결한 뒤 새로 고침을 누르세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-failed = 진단 정보를 수집하지 못했습니다. 새로 고침을 눌러 다시 시도하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-too-large = 통합 보고서가 너무 큽니다. 머신 한 대의 보고서를 생성하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-exhausted = 진단 요청을 더 이상 보낼 수 없습니다. 앱을 종료한 뒤 다시 실행하세요.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-daemon = 데몬 진단
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-process = 프로세스 진단
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-logs = 데몬 로그
# Independent basic and daemon source collection, not a refresh of credentials.
diagnostic-flow-daemon-note = 기본 머신 정보와 데몬 보고서는 각각 수집합니다.
# Only the daemon self-process numeric metadata; unsupported sources remain explicit.
diagnostic-flow-process-note = 데몬 자체의 숫자형 프로세스 메타데이터만 수집합니다. 지원하지 않는 항목은 별도로 표시합니다.
# The bounded failed-call summary excludes raw daemon logs.
diagnostic-flow-logs-note = 원본 데몬 로그는 수집하지 않습니다. 보고서에는 실패한 노드 호출의 제한된 요약만 포함합니다.
# Projection allowlist for the shareable report, not a general redaction guarantee.
diagnostic-flow-privacy = 보고서는 허용된 머신 메타데이터만 사용합니다. 머신 이름, 접속 경로, 키, 파일 경로, 원본 오류는 제외합니다.
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-report = 보고서 생성
# Heading and per-host summary for host diagnostics, not a partial-status badge.
diagnostic-flow-partial = 머신 진단
# Diagnostic sheet status or recovery; keep buttons short.
diagnostic-flow-no-hosts = 저장된 머신이 없습니다. 클라이언트 보고서는 복사할 수 있습니다.

# Short source availability badge; the row description carries the full explanation.
diagnostic-flow-unavailable-badge = 제공 안 됨

# A settled host read whose independent sources are incomplete.
diagnostic-flow-collected-partial = 일부 항목 수집 완료

# A reconnect invalidates the previous observation, including same-node reconnects.
diagnostic-flow-connection-changed = 머신 연결이 변경되었습니다. 보고서를 다시 생성하세요.

# Availability is resolved by each explicit host report, never inferred from this badge.
diagnostic-flow-host-dependent = 머신에 따라 다름

# Collection admission refused because this node is shutting down; not a missing API.
diagnostic-flow-stopping = 머신이 종료 중입니다. 다시 연결한 뒤 진단 정보를 수집하세요.
