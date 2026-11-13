## 숫자와 날짜·시간의 순서와 표기. 호출자 코드는 언어를 구분하지 않습니다.
formats-group-separator = ,
formats-decimal-separator = .
formats-digits = 0123456789
formats-group-primary = 3
formats-group-secondary = 3
formats-hour-cycle = h24
formats-month = { NUMBER($month, useGrouping: "false") }월
formats-weekday = { $weekday ->
    [1] 월요일
    [2] 화요일
    [3] 수요일
    [4] 목요일
    [5] 금요일
    [6] 토요일
   *[7] 일요일
    }
formats-date = { NUMBER($year, useGrouping: "false") }년 { formats-month } { NUMBER($day, useGrouping: "false") }일 { formats-weekday }
formats-time = { $hour-cycle ->
    [h12] { $period ->
        [am] 오전
       *[pm] 오후
        } { NUMBER($hour-twelve, useGrouping: "false") }:{ NUMBER($minute, minimumIntegerDigits: 2, useGrouping: "false") } { $zone }
   *[h24] { NUMBER($hour, minimumIntegerDigits: 2, useGrouping: "false") }:{ NUMBER($minute, minimumIntegerDigits: 2, useGrouping: "false") } { $zone }
    }
formats-date-time = { formats-date } { formats-time }
