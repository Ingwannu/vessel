## Native schedule list and action labels.

# Machine filter choice; keep short.
schedules-all-machines = 모든 머신

# Active filter empty title; centred, short.
schedules-empty-active = 활성 일정 없음

# Ended filter empty title; centred, short.
schedules-empty-ended = 종료된 일정 없음

# Forward-compatible filter empty title; centred, short.
schedules-empty-unknown = 다른 상태의 일정 없음

# Active empty description below the calendar-clock glyph.
schedules-empty-description = 일정은 정해진 주기로 에이전트를 실행합니다.

# Failed initial load, above its retry; no transport diagnostic.
schedules-unable = 일정을 불러올 수 없습니다

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
schedules-failed-older = 일정을 사용하려면 이 머신을 업데이트하세요.

# Lost connection; safe read retry only, never an automatic write retry.
schedules-failed-offline = 머신에 연결할 수 없습니다. 연결을 확인한 뒤 다시 시도하세요.

# Bounded app request pool is occupied; safe read may be retried.
schedules-failed-busy = 다른 일정 요청을 처리하고 있습니다. 잠시 후 다시 시도하세요.

# Reply identity/shape differs from the requested resource.
schedules-failed-unexpected = 머신에서 예상과 다른 응답을 받았습니다. 새로 고침한 뒤 다시 시도하세요.

# Local request ids exhausted; no retry button for this failure.
schedules-failed-exhausted = 일정 요청을 계속하려면 앱을 다시 여세요.

# Read refusal; distinct from the existing read-only mutation message.
schedules-failed-view-forbidden = 이 기기에서는 이 머신의 일정을 조회할 수 없습니다.

# Node rejected the request; diagnostics stay at the link edge.
schedules-failed-invalid = 머신에서 이 일정을 처리할 수 없습니다. 새로 고침한 뒤 설정을 확인하세요.

# Device credential no longer accepted.
schedules-failed-unauthorized = 이 기기를 머신과 다시 연결하세요.

# Negotiated peer protocol cannot carry the requested operation.
schedules-failed-protocol = 연결할 수 있도록 앱과 머신을 업데이트하세요.

# Node says schedule or target absent; do not guess which one.
schedules-failed-not-found = 이 일정 또는 실행 대상을 찾을 수 없습니다. 목록을 새로 고침하세요.

# Node reports changed state; no automatic mutation retry.
schedules-failed-conflict = 일정이 변경되었거나 현재 상태에서 실행할 수 없습니다. 목록을 새로 고침하세요.

# Node explicitly refuses this operation as unsupported.
schedules-failed-unsupported = 이 머신에서 해당 일정 작업을 지원하지 않습니다.

# Node could not find the scheduled agent installation.
schedules-failed-agent-missing = 실행하기 전에 이 일정의 에이전트를 머신에 설치하세요.

# Node could not start the scheduled agent.
schedules-failed-spawn = 머신에서 에이전트를 시작하지 못했습니다. 설치 상태와 폴더를 확인하세요.

# Node busy/shutting down; explicit retry is possible.
schedules-failed-unavailable = 머신이 다른 작업을 처리하거나 종료 중입니다. 잠시 후 다시 시도하세요.

# Node failed internally; no raw diagnostic or hidden write retry.
schedules-failed-internal = 머신에서 요청을 완료하지 못했습니다. 새로 고침한 뒤 다시 시도하세요.

# Typed calendar/catalog formatting failure in a row; no Display text.
schedules-display-unavailable = 일부 일정 정보를 표시할 수 없습니다. 앱을 업데이트한 뒤 다시 시도하세요.

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

# One complete creation clause; $age is prepared localized elapsed time/date.
schedules-created-age = 생성: { $age }

# Next-run relative distance below one minute; short.
schedules-countdown-soon = 곧

# Rounded next-run minutes, $count numeric; short.
schedules-countdown-minutes = { NUMBER($count, useGrouping: "false") }분 후

# Rounded next-run hours, $count numeric; short.
schedules-countdown-hours = { NUMBER($count, useGrouping: "false") }시간 후

# Rounded next-run days, $count numeric; short.
schedules-countdown-days = { NUMBER($count, useGrouping: "false") }일 후

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

# Machine selector with no match, distinct from an empty schedule filter.
schedules-search-empty = 일치하는 머신 없음

# Short menu trigger tooltip; never the row status or feature title.
schedules-more = 일정 작업

# Complete partial-host error line; the reason is already localized, never a transport diagnostic.
schedules-host-error = { $host }: { $reason }

# Browser build has no node transport; no fabricated schedule data or retry.
schedules-failed-no-transport = 이 브라우저 버전에서는 일정을 사용할 수 없습니다.

# `HostFilter` picker title; not searchable, as the reference, keep short.
schedules-filter-by-machine = 머신별로 보기

# Unassigned local feature: no call sent; updating the machine is not promised as a fix.
schedules-failed-not-enabled = 이 버전에서는 해당 일정 기능을 사용할 수 없습니다.

# Original selected physical admission retired before an operation; no retarget/retry.
schedules-failed-connection-changed = 머신 연결이 변경되었습니다. 새로 고침한 뒤 다시 시도하세요.

# Safe read deadline; a lost write uses the existing unconfirmed whole sentence.
schedules-failed-timeout = 머신에서 제시간에 응답하지 않았습니다. 연결을 확인한 뒤 다시 시도하세요.

# Runnable bucket label and active status badge.
schedules-active = 활성

# Filter for schedules that have finished.
schedules-ended = 종료

# A newer node reported a status this app does not recognize.
schedules-unknown = 알 수 없음

# Status badge for an explicitly paused schedule.
schedules-paused = 일시 정지

# Status badge when a schedule has used its run allowance.
schedules-finished = 완료

# Status badge when the schedule reached its expiry time.
schedules-expired = 만료

# Ended badge when the schedule’s credential owner no longer exists.
schedules-creator-gone = 생성자 없음

# Ended badge when the schedule’s destination no longer exists.
schedules-target-gone = 대상 없음

# Row notice when the paired device cannot change the schedule.
schedules-read-only = 이 머신의 일정은 이 기기에서 조회만 할 수 있습니다.

# Row menu action to stop future automatic runs.
schedules-pause = 일정 일시 정지

# Row menu action to allow automatic runs again.
schedules-resume = 일정 재개

# Row menu action to request one run outside the cadence.
schedules-run-now = 지금 실행

# Destructive action and confirmation title for an ordinary schedule.
schedules-delete = 일정 삭제

# Destructive action and confirmation title for an agent’s self-owned recurring prompt.
schedules-delete-heartbeat = 하트비트 삭제

# Fallback target or cadence label for a value from a newer protocol.
schedules-newer-value = 앱을 업데이트해야 표시할 수 있습니다

# Row metadata about the last run. $age is a localized distance or date.
schedules-last-run-age = 마지막 실행 { $age }

# Row metadata about the next run. $countdown is a localized future distance.
schedules-next-run = 다음 실행 { $countdown }

# Target line naming project $name and its relative folder $dir.
schedules-project-folder = { $name } · { $dir }

# Capacity notice. $host is the machine name and $count is its numeric user-schedule quota.
schedules-list-limit = { $host }: 이 머신의 사용자 소유 일정이 한도인 { NUMBER($count) }개에 도달했습니다.

# Cadence label for once per minute.
schedules-every-minute = 매분

# Cadence label for once on each hour boundary.
schedules-every-hour = 매시 정각

# Hourly cron label. $minute is a two-digit minute within each hour.
schedules-hourly-at = 매시 { $minute }분

# Fixed-time cron label. $day is a localized day group, $time is HH:MM, and $zone is a node IANA name.
schedules-fixed-at = { $day } { $time } ({ $zone })

# Elapsed cadence label. $duration is the full localized interval.
schedules-every-interval = { $duration }마다

# Cadence label for an elapsed interval of one day.
schedules-every-day = 매일

# Day part of an elapsed interval. $count is numeric.
schedules-interval-days = { NUMBER($count) }일

# Hour part of an elapsed interval. $count is numeric.
schedules-interval-hours = { NUMBER($count) }시간

# Minute part of an elapsed interval. $count is numeric.
schedules-interval-minutes = { NUMBER($count) }분

# Second part of an elapsed interval. $count is numeric.
schedules-interval-seconds = { NUMBER($count) }초

# Day group in a fixed-time cron description.
schedules-daily = 매일

# Monday through Friday in a fixed-time cron description.
schedules-weekdays = 평일

# Saturday and Sunday in a fixed-time cron description.
schedules-weekends = 주말

# Weekly Monday recurrence in a fixed-time cron description.
schedules-monday = 월요일

# Weekly Tuesday recurrence in a fixed-time cron description.
schedules-tuesday = 화요일

# Weekly Wednesday recurrence in a fixed-time cron description.
schedules-wednesday = 수요일

# Weekly Thursday recurrence in a fixed-time cron description.
schedules-thursday = 목요일

# Weekly Friday recurrence in a fixed-time cron description.
schedules-friday = 금요일

# Weekly Saturday recurrence in a fixed-time cron description.
schedules-saturday = 토요일

# Weekly Sunday recurrence in a fixed-time cron description.
schedules-sunday = 일요일

# Search aliases for the Schedules destination in Command Center.
schedules-search-keywords = 일정 일정 자동 실행 반복 schedules scheduled automation recurring
