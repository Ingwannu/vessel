## 작성기 shell, 전체 화면 및 명령 메뉴. Paseo composer/input과
## agentAutocomplete 기준. 버튼은 짧게, 오류는 완전한 문장으로 쓴다.

# 작은 화면에서 여러 줄 작성기를 펼치는 버튼.
composer-shell-fullscreen = 전체 화면
# 전체 화면 편집기를 접는 버튼.
composer-shell-exit-fullscreen = 전체 화면 종료
# 일반 채팅 편집기의 접근성 이름.
composer-shell-chat-input = 에이전트에게 메시지...
# 같은 편집기를 터미널 입력에 쓸 때의 접근성 이름.
composer-shell-terminal-input = 터미널 프롬프트
# 자동완성의 /exit 설명.
composer-shell-client-exit = 현재 에이전트 보관
# 자동완성의 /clear 설명.
composer-shell-client-clear = 이 에이전트를 보관하고 새 초안 시작
# 작업 공간을 선택하기 전 초안의 명령 메뉴.
composer-shell-choose-project = 명령을 보려면 프로젝트를 선택하세요
# 공급자와 모델을 선택하기 전 초안의 명령 메뉴.
composer-shell-choose-model = 명령을 보려면 모델을 선택하세요
# 공급자 명령을 조회하는 동안 표시.
composer-shell-commands-loading = 명령을 불러오는 중...
# 명령 조회 실패 후 재시도 전. 공급자의 stderr는 넣지 않는다.
composer-shell-commands-failed = 명령을 불러오지 못했습니다
# 생성 흐름이 전송 중 초안을 유지하고 잠근 작성기.
composer-shell-submit-locked = 전송 중...
# 보기 전용 문맥에 유지된 편집기.
composer-shell-read-only = 이 대화는 읽기 전용입니다.
# 허용된 작업 공간에서 파일 또는 이미지 링크를 찾지 못했을 때.
composer-shell-link-not-found = 이 작업 공간에서 일치하는 파일을 찾지 못했습니다.
# Files 영역에서 열 수 있는 모든 폴더 밖의 파일을 가리킬 때.
composer-shell-link-outside = 이 파일은 사용할 수 있는 작업 공간 폴더 밖에 있습니다.
# 원인을 유형으로 유지하여 언어 변경 시 다시 번역한다. $reason은
# 완전한 문장을 선택하며 노드 오류나 검증되지 않은 경로를 담지 않는다.
composer-shell-command-problem = { $reason ->
    [unsupported] 이 공급자는 명령 목록을 제공하지 않습니다.
    [version] 명령을 불러오려면 이 기기와 머신의 Vessel을 업데이트하세요.
    [access] 이 기기에는 명령을 불러올 권한이 없습니다.
    [disconnected] 명령을 보려면 이 머신에 연결하세요.
    [invalid] 초안 설정으로 명령을 불러올 수 없습니다.
   *[unavailable] 명령을 불러오지 못했습니다. 다시 시도하세요.
    }
