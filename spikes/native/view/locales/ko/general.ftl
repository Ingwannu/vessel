## Settings General: existing labels and delivery explanations.

# Settings navigation and General page title. Short.
general-title = 일반

# Heading over the send preference rows. Short.
general-sending = 전송

# Label of the physical send-key picker; the chord is typed data. Short.
general-send-shortcut = 전송 단축키

# Label of the default delivery-mode picker. Short.
general-default-send = 기본 전송

# Queue delivery-mode choice. Short.
general-queue = 대기열

# Steer delivery-mode choice. Short.
general-steer = 지시

# Interrupt delivery-mode choice. Short.
general-interrupt = 중단

# Heading over local preference storage. Short.
general-kept = 보관

# Label of the preference storage row. Short.
general-preferences = 환경설정

# Status when preferences last only until this app quits. Short.
general-this-launch = 이번 실행에만

# Whole hint for queue delivery, including the permission-wait exception.
general-queue-hint = 에이전트가 일하는 동안 보낸 메시지는 기다렸다가, 차례가 끝나면 갑니다. 권한 요청이 기다리는 동안에는 지금 차례를 멈추고 갑니다.

# Whole hint for steer delivery, including its unsupported-agent fallback.
general-steer-hint = 에이전트가 일하는 동안 보낸 메시지는, 에이전트가 받을 수 있으면 지금 차례에 합류하고 아니면 지금 차례를 멈추고 다음 차례를 시작합니다.

# Whole hint for interrupt delivery.
general-interrupt-hint = 에이전트가 일하는 동안 보낸 메시지는 지금 차례를 멈추고 다음 차례를 시작합니다.

# Whole storage hint. $path is a display path; keep its own line.
general-kept-file =
    Vessel이 이 컴퓨터에 보관하므로, 실행할 때마다 그대로 열립니다.
    { $path }

# Whole explanation when this surface has no place to keep preferences.
general-kept-no-place = Vessel을 끝낼 때까지만 보관합니다: 이 컴퓨터에 보관할 곳이 없습니다.

# Whole safe explanation; an arbitrary path-discovery diagnostic is not UI text.
general-kept-no-path = Vessel을 끝낼 때까지만 보관합니다: 환경설정 위치를 정할 수 없습니다.

# Whole explanation for a newer preferences schema. $path is a display path.
# $version is a numeric schema version, displayed without grouping.
general-kept-newer = Vessel을 끝낼 때까지만 보관합니다: { $path }{ PARTICLE($path, "topic") } 더 새 Vessel(버전 { NUMBER($version, useGrouping: "false") })이 썼습니다. 그대로 둡니다.

# Safe fallback if the typed storage message cannot be prepared before paint.
general-storage-unavailable = 환경설정 보관 정보를 표시할 수 없습니다.

# Frame notice after copying the preference file path.
general-path-copied = 경로를 복사했습니다.

# Whole unreadable-file recovery. $problem is closed; $path is a display path.
general-kept-unreadable = Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: { $path } ({ $problem ->
        [not-toml] TOML 형식이 아닙니다(닫히지 않은 따옴표나 괄호, `=`가 없는 줄을 확인하세요)
        [not-json] JSON 형식이 아닙니다
        [unknown-key] 이 파일에서 쓰지 않는 키가 있습니다
        [duplicate-key] 같은 키가 두 번 있습니다
        [missing-key] 꼭 있어야 하는 키가 없습니다
        [wrong-token] `token`에는 토큰 자체가 아니라 토큰이 든 파일을 적습니다: token = { "{" } file = "…" { "}" }
        [wrong-value] 값의 종류가 맞지 않습니다
       *[cannot-open] 파일을 열 수 없습니다(소유자와 권한을 확인하세요)
    }). 고치거나 지운 뒤 Vessel을 다시 시작하세요.

# Whole wrong-type recovery for a known field. $field names only a reader-approved key.
# $path is a display path.
general-kept-unreadable-field = Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: { $path } (`{ $field }` 값의 종류가 맞지 않습니다). 고치거나 지운 뒤 Vessel을 다시 시작하세요.

# Whole unreadable-file recovery. $problem is closed; $path is a display path.
# $line and $column are numeric source positions, displayed without grouping.
general-kept-unreadable-at = Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: { $path } ({ NUMBER($line, useGrouping: "false") }번째 줄 { NUMBER($column, useGrouping: "false") }번째 칸: { $problem ->
        [not-toml] TOML 형식이 아닙니다(닫히지 않은 따옴표나 괄호, `=`가 없는 줄을 확인하세요)
        [not-json] JSON 형식이 아닙니다
        [unknown-key] 이 파일에서 쓰지 않는 키가 있습니다
        [duplicate-key] 같은 키가 두 번 있습니다
        [missing-key] 꼭 있어야 하는 키가 없습니다
        [wrong-token] `token`에는 토큰 자체가 아니라 토큰이 든 파일을 적습니다: token = { "{" } file = "…" { "}" }
        [wrong-value] 값의 종류가 맞지 않습니다
       *[cannot-open] 파일을 열 수 없습니다(소유자와 권한을 확인하세요)
    }). 고치거나 지운 뒤 Vessel을 다시 시작하세요.

# Whole wrong-type recovery for a known field. $field names only a reader-approved key.
# $path is a display path.
# $line and $column are numeric source positions, displayed without grouping.
general-kept-unreadable-field-at = Vessel을 끝낼 때까지만 보관합니다: 다음 파일을 읽지 못했습니다: { $path } ({ NUMBER($line, useGrouping: "false") }번째 줄 { NUMBER($column, useGrouping: "false") }번째 칸: `{ $field }` 값의 종류가 맞지 않습니다). 고치거나 지운 뒤 Vessel을 다시 시작하세요.
