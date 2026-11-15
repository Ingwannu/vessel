## 설정 › 프로바이더: 선택한 머신이 보고한 레지스트리 항목.

# 레지스트리 타일 위의 짧은 섹션 제목. 값 없음.
agent-catalog-title = 프로바이더 추가

# 이름과 식별자로 레지스트리를 검색하는 입력란. 값 없음.
agent-catalog-search = 프로바이더 검색

# 머신에 고정 버전의 명령을 등록하는 버튼. 설치 프로그램은 실행하지 않음.
agent-catalog-add = 추가

# 추가 요청을 처리하는 동안의 짧은 버튼 레이블. 값 없음.
agent-catalog-adding = 추가 중…

# 실제 버튼 레이블을 참조하는 완전한 안내 문장. 설치나 실행을 뜻하지 않음.
agent-catalog-add-help = 프로바이더를 등록하려면 “{ agent-catalog-add }” 버튼을 누르세요. 프로바이더를 설치하거나 실행하지는 않습니다.

# 다른 이름이나 버전이어도 같은 레지스트리 식별자가 이미 등록됨.
agent-catalog-recorded = 이 머신에 이미 추가된 프로바이더입니다.

# 추가는 가능함. 대화를 시작하기 전에 머신에 패키지 실행기가 필요함.
agent-catalog-runner-missing = 프로바이더를 추가할 수 있지만, 대화하려면 먼저 머신에 패키지 실행기를 설치해야 합니다.

# 머신이 이 항목을 기본 제공 에이전트로 분류함.
agent-catalog-built-in = 기본 제공 프로바이더로 별도 관리되는 항목입니다.

# 머신은 보관 파일만 보고하므로 명령으로 직접 추가해야 함.
agent-catalog-download = 머신에 프로바이더를 설치한 다음 명령으로 추가하세요.

# 머신의 플랫폼에서 실행할 수 있는 배포 항목이 없음.
agent-catalog-unavailable = 이 머신에서는 사용할 수 없는 프로바이더입니다.

# 레지스트리를 정상적으로 읽었지만 항목이 없음. 읽기 실패와 다름.
agent-catalog-empty = 이 머신에서 표시할 레지스트리 프로바이더가 없습니다.

# 이름과 식별자 검색에 일치하는 항목이 없음.
agent-catalog-no-matches = 검색과 일치하는 프로바이더가 없습니다.

# 머신이 아직 레지스트리를 읽지 않음. 정상적인 빈 목록과 구분함.
agent-catalog-not-read = 아직 프로바이더 레지스트리를 읽지 않았습니다.

# 기존 작은 로더 옆에 표시하는 첫 읽기 상태. 값 없음.
agent-catalog-reading = 프로바이더 레지스트리를 읽는 중…

# Registry-only retry/refresh control; keep short.
agent-catalog-refresh = 새로 고침

# A reused book route was refused before sending a registry read.
agent-catalog-changed = 이 이름이 가리키는 머신이 변경되었습니다. 다시 시도하기 전에 “{ agent-catalog-refresh }” 버튼을 선택해 프로바이더 목록을 읽으세요.

# The existing Agents hub refused a new worker.
agent-catalog-busy = 설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.

# Reconnect before retrying the registry read.
agent-catalog-offline = 머신에 연결되지 않았습니다. 연결한 뒤 “{ agent-catalog-refresh }” 버튼을 선택하세요.

# A definite older peer has no registry method; not empty data.
agent-catalog-older = 프로바이더 레지스트리를 읽으려면 이 머신의 Vessel을 업데이트하세요.

# The node refused registry read or explicit refresh permission.
agent-catalog-permission = 이 기기에는 이 레지스트리 작업을 수행할 권한이 없습니다. 머신 관리자에게 접근 권한을 요청하세요.

# Only the client wait ended; does not claim cancellation of node fetch.
agent-catalog-timeout = 머신이 제한 시간 안에 프로바이더 레지스트리를 보내지 않았습니다. “{ agent-catalog-refresh }” 버튼을 선택해 다시 시도하세요.

# Unmodified machine prose is diagnostic detail, not this sentence.
agent-catalog-read-error = 프로바이더 레지스트리를 읽지 못했습니다. “{ agent-catalog-refresh }” 버튼을 선택해 다시 시도하세요.

# Browser transport is absent, not an empty catalog.
agent-catalog-no-transport = 이 브라우저에서는 아직 머신에 연결할 수 없습니다.
