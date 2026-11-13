## 번역 계층의 계약 예제. 화면 문구가 아닙니다.
examples-welcome = { $name }님, 환영합니다.
examples-sessions = 실행 중인 세션 { NUMBER($count) }개
examples-state = { $state ->
    [connecting] 머신에 연결하고 있습니다.
   *[ready] 머신이 준비되었습니다.
    }
examples-remove = { $name }{ PARTICLE($name, "object") } 삭제하세요.
examples-ago = { $unit ->
    [minute] { NUMBER($count) }분 전
   *[hour] { NUMBER($count) }시간 전
    }

examples-grouped = 세션 총 { NUMBER($count) }개입니다.
examples-date = 페어링 날짜: { DATETIME($date) }
examples-time = 페어링 시각: { DATETIME($date) }
examples-timestamp = 페어링 일시: { DATETIME($date) }
examples-in-minutes = { $count }분 후
