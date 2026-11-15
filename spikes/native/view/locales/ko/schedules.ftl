## Schedule-list labels and feedback.
## Existing cadence, state/action and editor-error words remain in the legacy
## schedule table until its separately owned migration. Never duplicate them here.

# Machine filter choice; keep short.
schedules-all-machines = 모든 머신

# Active filter empty title; centred, short.
schedules-empty-active = 활성 예약 없음

# Ended filter empty title; centred, short.
schedules-empty-ended = 종료된 예약 없음

# Forward-compatible filter empty title; centred, short.
schedules-empty-unknown = 다른 상태의 예약 없음

# Active empty description below the calendar-clock glyph.
schedules-empty-description = 예약은 정해진 주기로 에이전트를 실행합니다.

# Failed initial load, above its retry; no transport diagnostic.
schedules-unable = 예약을 불러올 수 없습니다

# Ghost load-error retry and safe host retry action; short.
schedules-retry = 다시 시도

# Cancel schedule deletion; short.
schedules-cancel = 취소

# Nonselectable pending menu item, replacing Pause; short.
schedules-pausing = 일시 정지 중...

# Nonselectable pending menu item, replacing Resume; short.
schedules-resuming = 재개 중...

# Nonselectable pending menu item, replacing Run now; short.
schedules-starting = 실행 시작 중...

# Nonselectable pending menu item, replacing Delete; short.
schedules-deleting = 삭제 중...

# Existing chat target whose cached title is empty; one target line.
schedules-target-chat-untitled = 제목 없는 대화

# No cached target chat; never a claim it was deleted.
schedules-target-chat-unavailable = 대화 정보를 불러올 수 없음

# Explicit project target whose cached name is unavailable.
schedules-target-project-unavailable = 프로젝트 정보를 불러올 수 없음

# Explicit worktree target whose node path is unavailable.
schedules-target-worktree-unavailable = 작업 트리 정보를 불러올 수 없음

# New chat using its agent default folder; no fabricated path.
schedules-target-default-folder = 에이전트 기본 폴더

# History clause when the node reports no past run; short.
schedules-never-run = 아직 실행하지 않음

# List unavailable because the peer has no schedule method.
schedules-failed-older = 예약을 사용하려면 이 머신을 업데이트하세요.

# Lost connection; safe read retry only, never an automatic write retry.
schedules-failed-offline = 머신에 연결할 수 없습니다. 연결을 확인한 뒤 다시 시도하세요.

# Bounded app request pool is occupied; safe read may be retried.
schedules-failed-busy = 다른 예약 요청을 처리하고 있습니다. 잠시 후 다시 시도하세요.

# Reply identity/shape differs from the requested resource.
schedules-failed-unexpected = 머신에서 예상과 다른 응답을 받았습니다. 새로 고침한 뒤 다시 시도하세요.

# Local request ids exhausted; no retry button for this failure.
schedules-failed-exhausted = 예약 요청을 계속하려면 앱을 다시 여세요.

# Read refusal; distinct from the existing read-only mutation message.
schedules-failed-view-forbidden = 이 기기에서는 이 머신의 예약을 조회할 수 없습니다.

# Node rejected the request; diagnostics stay at the link edge.
schedules-failed-invalid = 머신에서 이 예약을 처리할 수 없습니다. 새로 고침한 뒤 설정을 확인하세요.

# Device credential no longer accepted.
schedules-failed-unauthorized = 이 기기를 머신과 다시 연결하세요.

# Negotiated peer protocol cannot carry the requested operation.
schedules-failed-protocol = 연결할 수 있도록 앱과 머신을 업데이트하세요.

# Node says schedule or target absent; do not guess which one.
schedules-failed-not-found = 이 예약 또는 실행 대상을 찾을 수 없습니다. 목록을 새로 고침하세요.

# Node reports changed state; no automatic mutation retry.
schedules-failed-conflict = 예약이 변경되었거나 현재 상태에서 실행할 수 없습니다. 목록을 새로 고침하세요.

# Node explicitly refuses this operation as unsupported.
schedules-failed-unsupported = 이 머신에서 해당 예약 작업을 지원하지 않습니다.

# Node could not find the scheduled agent installation.
schedules-failed-agent-missing = 실행하기 전에 이 예약의 에이전트를 머신에 설치하세요.

# Node could not start the scheduled agent.
schedules-failed-spawn = 머신에서 에이전트를 시작하지 못했습니다. 설치 상태와 폴더를 확인하세요.

# Node busy/shutting down; explicit retry is possible.
schedules-failed-unavailable = 머신이 다른 작업을 처리하거나 종료 중입니다. 잠시 후 다시 시도하세요.

# Node failed internally; no raw diagnostic or hidden write retry.
schedules-failed-internal = 머신에서 요청을 완료하지 못했습니다. 새로 고침한 뒤 다시 시도하세요.

# Typed calendar/catalog formatting failure in a row; no Display text.
schedules-display-unavailable = 일부 예약 정보를 표시할 수 없습니다. 앱을 업데이트한 뒤 다시 시도하세요.


## Prepared relative time and metadata

# Elapsed time below one minute, including future timestamps; short.
schedules-age-now = 방금

# Floor elapsed minutes, $count numeric; short.
schedules-age-minutes = { NUMBER($count, useGrouping: "false") }분 전

# Floor elapsed hours, $count numeric; short.
schedules-age-hours = { NUMBER($count, useGrouping: "false") }시간 전

# Floor elapsed days below one week, $count numeric; short.
schedules-age-days = { NUMBER($count, useGrouping: "false") }일 전

# Older viewer calendar date. $month and $day are numeric date parts; global month names are shared.
schedules-age-date = { NUMBER($month, useGrouping: "false") }월 { NUMBER($day, useGrouping: "false") }일

# Older date in explicit UTC fallback; $month/$day numeric.
schedules-age-date-utc = { NUMBER($month, useGrouping: "false") }월 { NUMBER($day, useGrouping: "false") }일 UTC

# One complete creation clause; $age is prepared localized elapsed time/date.
schedules-created-age = 생성: { $age }

# One history clause. $age localized distance, $outcome from the accepted result table; node result only.
schedules-last-result-age = 최근 실행: { $age } ({ $outcome })

# Next-run relative distance below one minute; short.
schedules-countdown-soon = 곧

# Rounded next-run minutes, $count numeric; short.
schedules-countdown-minutes = { NUMBER($count, useGrouping: "false") }분 후

# Rounded next-run hours, $count numeric; short.
schedules-countdown-hours = { NUMBER($count, useGrouping: "false") }시간 후

# Rounded next-run days, $count numeric; short.
schedules-countdown-days = { NUMBER($count, useGrouping: "false") }일 후

# Next clause. $at typed date/time already in viewer zone, $countdown localized distance. DATETIME names its zone.
schedules-next-known = 다음 실행: { $countdown } · 내 시각: { DATETIME($at) }

# Next clause when platform zone is unavailable; typed $at is explicitly UTC.
schedules-next-utc = 다음 실행: { $countdown } · { DATETIME($at) } · 내 시간대를 확인할 수 없음

# Adjacent rule-zone identity when it differs; $zone is a validated rule IANA name, never machine zone metadata.
schedules-rule-zone = 예약 시간대: { $zone }

# Complete metadata without next run. Inputs are independently prepared cadence/creation/history phrases; translators may reorder.
schedules-metadata = { $cadence } · { $created } · { $last }

# Complete metadata with next run; $next is the prepared next-run clause.
schedules-metadata-next = { $cadence } · { $created } · { $last } · { $next }

# Multi-machine metadata prefix only. $host is its book display name, $metadata the prepared row phrases.
schedules-metadata-host = { $host } · { $metadata }

# Destructive confirmation body. $title is the displayed schedule title; keep the whole warning together.
schedules-delete-confirm = “{ $title }” 항목을 삭제할까요? 이 작업은 되돌릴 수 없습니다.

# Destructive confirmation button only; short, distinct from the existing product-specific menu label.
schedules-delete-confirm-action = 삭제

# Lost write reply: the node might already have acted. No automatic retry, and no completed result is fabricated.
schedules-write-unconfirmed = 작업 결과를 확인할 수 없습니다. 작업이 완료되었을 수도 있습니다. 최신 상태를 확인한 뒤 다시 시도하세요.

# Bundled zone lookup failed; no guessed offset or diagnostic.
schedules-clock-zone = 이 시간대를 사용할 수 없습니다. 시각을 표시하려면 앱을 업데이트하세요.

# A platform zone identity cannot be represented by the app.
schedules-clock-name = 시간대 이름을 표시할 수 없습니다. 앱을 업데이트하세요.

# Typed display-calendar conversion failure.
schedules-clock-range = 표시할 수 있는 날짜 범위를 벗어났습니다.

# Machine selector with no match, distinct from an empty schedule filter.
schedules-search-empty = 일치하는 머신 없음

# Short menu trigger tooltip; never the row status or feature title.
schedules-more = 예약 작업

# Complete partial-host error line; the reason is already localized, never a transport diagnostic.
schedules-host-error = { $host }: { $reason }

# Next-run clause with a different rule zone; references retain their typed arguments.
schedules-next-known-rule = { schedules-next-known } · { schedules-rule-zone }

# Explicit UTC viewer fallback plus the separate rule zone.
schedules-next-utc-rule = { schedules-next-utc } · { schedules-rule-zone }

# Paused cron metadata still names the rule zone without promising a next run.
schedules-metadata-rule = { schedules-metadata } · { schedules-rule-zone }

# Browser build has no node transport; no fabricated schedule data or retry.
schedules-failed-no-transport = 이 브라우저 버전에서는 예약을 사용할 수 없습니다.

# `HostFilter` picker title; not searchable, as the reference, keep short.
schedules-filter-by-machine = 머신별로 보기

# Unassigned local feature: no call sent; updating the machine is not promised as a fix.
schedules-failed-not-enabled = 이 버전에서는 해당 예약 기능을 사용할 수 없습니다.

# Cadence preset picker labels; not the saved row’s full cadence description.
schedules-preset-minute = 매분

# Hourly cadence preset picker label; not a saved cadence description.
schedules-preset-hour = 매시간

# Daily-at-nine cadence preset picker label; keep short.
schedules-preset-daily = 매일 09:00

# Monday-to-Friday-at-nine cadence preset picker label; keep short.
schedules-preset-weekdays = 평일 09:00

# Monday-at-nine cadence preset picker label; keep short.
schedules-preset-mondays = 월요일 09:00

# Original selected physical admission retired before an operation; no retarget/retry.
schedules-failed-connection-changed = 머신 연결이 변경되었습니다. 새로 고침한 뒤 다시 시도하세요.

# Safe read deadline; a lost write uses the existing unconfirmed whole sentence.
schedules-failed-timeout = 머신에서 제시간에 응답하지 않았습니다. 연결을 확인한 뒤 다시 시도하세요.

## Form validation, separate from accepted cadence/error tables

# Whole field error; numeric maximum comes from the typed node bound.
schedules-form-name-invalid = 제어 문자 없이 짧은 이름을 한 줄로 입력하세요.

# Whole field error; numeric maximum comes from the typed node bound.
schedules-form-prompt-invalid = 프롬프트를 { NUMBER($maximum, useGrouping: "false") } UTF-8 바이트 이내로 입력하세요. 제어 문자는 탭과 줄바꿈만 사용할 수 있습니다.

# Whole field error; numeric maximum comes from the typed node bound.
schedules-form-max-runs-invalid = 1부터 { NUMBER($maximum, useGrouping: "false") } 사이의 정수를 입력하거나, 실행 횟수를 제한하지 않으려면 비워 두세요.

# Required structured chat target; a missing selection blocks saving.
schedules-form-target-required = 예약에 사용할 에이전트와 폴더를 선택하세요.

## Node-only editor preview failures

# Whole typed preview failure; never show wire message/Display.
schedules-preview-mismatched = 머신에서 규칙과 다르거나 불완전한 미리보기를 받았습니다. 실행 시각을 표시할 수 없습니다.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-unknown-zone = 이 머신에서 해당 시간대를 지원하지 않습니다. 다른 시간대를 선택하거나 머신을 업데이트하세요.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-unknown-cadence = 이 종류의 예약을 미리 보려면 머신을 업데이트하세요.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-no-future-run = 머신에서 지원하는 기간 안에 이 규칙으로 실행할 시각 세 개를 찾을 수 없습니다.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-out-of-range = 선택한 시각이 머신에서 지원하는 범위를 벗어납니다.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-unknown = 머신의 미리보기 거절 사유를 보려면 앱을 업데이트하세요.

# Save waits for the exact current node preview, except an explicit older-node refusal.
schedules-form-preview-required = 저장하기 전에 현재 규칙의 실행 시각을 미리 확인하세요.

## Read-only schedule history

# Read-only row-menu entry opening past runs; short.
schedules-history-menu = 실행 기록 보기

# History dialog title; $title is the original displayed schedule title.
schedules-history-title = 실행 기록 · { $title }

# History has loaded successfully but no run is retained.
schedules-history-empty = 실행 기록이 없습니다

# Close the read-only history dialog; short.
schedules-history-close = 닫기

# Explicit history reload; short.
schedules-history-refresh = 새로고침

# Run row button; present only if its outcome names a session.
schedules-history-open-chat = 대화 열기

# Run heading; $at is typed viewer date/time, $number numeric run number.
schedules-history-run-at = 실행 { NUMBER($number, useGrouping: "false") } · { DATETIME($at) }

# Typed skip reason; no diagnostic data.
schedules-history-previous-running = 이전 실행이 아직 진행 중입니다.

# Typed skip reason; no diagnostic data.
schedules-history-previous-waiting = 이전 프롬프트가 아직 대기 중입니다.

# Typed skip reason; a stopped chat cannot restart automatically.
schedules-history-stopped = 대화를 중지한 상태여서 자동으로 다시 시작하지 않았습니다.

# Typed missed slots; $count numeric from node.
schedules-history-missed = 머신에 연결할 수 없는 동안 예약된 실행 { NUMBER($count, useGrouping: "false") }회를 건너뛰었습니다.

# Forward-compatible reason in run history.
schedules-history-reason-unknown = 이 실행의 사유를 보려면 앱을 업데이트하세요.

# Historical InvalidRequest; no wire diagnostic.
schedules-history-run-invalid = 머신에서 이 실행의 입력을 거절했습니다.

# Historical Unauthorized; no wire diagnostic.
schedules-history-run-unauthorized = 이 실행에 사용한 인증 정보를 받아들이지 않았습니다.

# Historical Forbidden, not a claim about current viewer scopes.
schedules-history-run-forbidden = 예약을 만든 주체에게 이 실행을 수행할 권한이 없었습니다.

# Historical ProtocolMismatch.
schedules-history-run-protocol = 이 실행에서 호환되지 않는 통신 규약을 사용했습니다.

# Historical NotFound, not guessed target deletion now.
schedules-history-run-not-found = 이 실행에 필요한 리소스를 찾지 못했습니다.

# Historical Conflict.
schedules-history-run-conflict = 다른 작업 때문에 이 실행을 시작하지 못했습니다.

# Historical Unsupported.
schedules-history-run-unsupported = 머신에서 이 실행에 필요한 작업을 지원하지 않았습니다.

# Historical HarnessNotInstalled.
schedules-history-run-agent-missing = 필요한 에이전트가 설치되어 있지 않았습니다.

# Historical SpawnFailed.
schedules-history-run-spawn = 에이전트를 시작하지 못했습니다.

# Historical Unavailable.
schedules-history-run-unavailable = 이 실행에 필요한 서비스를 사용할 수 없었습니다.

# Historical Internal; no private diagnostic.
schedules-history-run-internal = 머신에서 이 실행을 완료하지 못했습니다.

# History fallback heading when a typed calendar/catalog error prevents prepared rows.
schedules-history-heading = 실행 기록
