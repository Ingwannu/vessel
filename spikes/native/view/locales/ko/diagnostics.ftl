## Settings Diagnostics controls, after Paseo. The report is fixed technical data.

# 설정 탐색 목록과 페이지 제목. 짧게 씁니다.
diagnostics-title = 진단

# 보고서 행과 대화 상자의 제목. 짧게 씁니다.
diagnostics-report-title = 앱 진단

# 실행 행의 설명. 복사하는 기술 보고서는 고정 스키마를 유지합니다.
diagnostics-description = 연결, 머신, 에이전트 및 로그 세부 정보를 수집합니다.

# 보고서를 수집하고 대화 상자를 엽니다. 짧게 씁니다.
diagnostics-run = 실행

# 수집한 보고서를 복사합니다. 짧게 씁니다.
diagnostics-copy = 복사

# 보고서를 다시 수집합니다. 짧게 씁니다.
diagnostics-refresh = 새로 고침

# 보고서를 복사한 뒤 띄우는 복사 토스트의 이름("{ $label } 복사됨", Paseo의 settings.diagnostics.app.copyLabel).
diagnostics-copied-label = 진단

## 앱 진단의 진행 상황(Paseo의 settings.diagnostics.app.running과 진행 행). 이름은 머신 이름입니다.

# 첫 진단을 수집하는 동안 스피너 옆에 표시합니다.
diagnostics-running = 진단을 실행하는 중...

# 앱 자신의 정보에 대한 진행 행. 짧게 씁니다.
diagnostics-step-app = 앱

# 머신에 묻는 동안의 진행 행. $label은 단계 이름입니다.
diagnostics-step-running = { $label }: 실행 중

# 머신이 답했거나 물을 것이 없을 때의 진행 행.
diagnostics-step-done = { $label }: 완료

# 머신에 묻지 못했을 때의 진행 행.
diagnostics-step-failed = { $label }: 실패

# 진단 제목 줄의 복사 동작 접근성 이름.
diagnostics-copy-label = 진단 복사

# 진단 제목 줄의 새로 고침 동작 접근성 이름.
diagnostics-refresh-label = 진단 새로 고침

# 수집하는 동안 새로 고침 동작의 접근성 이름.
diagnostics-refreshing-label = 진단을 새로 고치는 중

## 에이전트 진단(Paseo의 settings.providers.diagnostic).

# 에이전트 세부 정보 옆의 버튼과 대화 상자 제목. 짧게 씁니다.
diagnostics-agent = 진단

# 아무것도 수집하지 못했을 때 대화 상자 본문.
diagnostics-agent-none = 진단 정보가 없습니다

# 진단이나 노드 로그를 읽지 못한 이유: 연결 없음.
diagnostics-problem-offline = 머신이 연결되어 있지 않습니다.

# 읽지 못한 이유: 노드가 거부함. $code는 "forbidden" 같은 통신 코드입니다.
diagnostics-problem-refused = 노드가 거부했습니다({ $code }).

# 읽지 못한 이유: 호출 제한 시간 안에 답이 없음.
diagnostics-problem-timed-out = 머신이 제시간에 답하지 않았습니다.

## 머신의 로그(Paseo의 desktop.daemon.logs).

# 로그 행이 있는 개요 섹션의 제목. 짧게 씁니다.
diagnostics-log-section = 노드


# 개요 행의 제목. 짧게 씁니다.
diagnostics-log-title = 로그 파일

# 경로를 읽을 수 없을 때 행의 설명과 대화 상자의 경로 줄.
diagnostics-log-unavailable = 로그 경로를 알 수 없음

# 로그 경로를 복사합니다. 짧게 씁니다.
diagnostics-log-copy-path = 경로 복사

# 로그 대화 상자를 엽니다. 짧게 씁니다.
diagnostics-log-open = 로그 열기

# 로그 대화 상자 제목.
diagnostics-log-dialog = 노드 로그

# 파일에 줄이 없을 때 대화 상자 내용.
diagnostics-log-empty = (로그 파일이 비어 있음)

# 경로를 복사한 뒤 띄우는 토스트.
diagnostics-log-copied = 로그 경로를 복사했습니다.

# 대화 상자가 로그를 읽는 동안 스피너 옆에 표시합니다.
diagnostics-log-loading = 노드 로그를 불러오는 중...

# 노드가 자신을 시작한 터미널에 로그를 쓸 때 행의 설명.
diagnostics-log-terminal = 이 노드는 시작된 터미널에 로그를 씁니다.

# 노드의 로그가 노드가 관리하는 파일이 아닐 때 행의 설명.
diagnostics-log-elsewhere = 이 노드의 로그는 노드가 관리하는 파일이 아닙니다(서비스 관리자의 저널 등).
