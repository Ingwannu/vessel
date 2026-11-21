## Native schedule list and action labels.

# Machine filter choice; keep short.
schedules-all-machines = All machines

# Active filter empty title; centred, short.
schedules-empty-active = No active schedules

# Ended filter empty title; centred, short.
schedules-empty-ended = No ended schedules

# Forward-compatible filter empty title; centred, short.
schedules-empty-unknown = No schedules in other states

# Active empty description below the calendar-clock glyph.
schedules-empty-description = Schedules run agents on a cadence.

# Failed initial load, above its retry; no transport diagnostic.
schedules-unable = Unable to load schedules

# Ghost load-error retry and safe host retry action; short.
schedules-retry = Try again

# Cancel schedule deletion; short.
schedules-cancel = Cancel

# Nonselectable pending menu item, replacing Pause; short.
schedules-pausing = Pausing...

# Nonselectable pending menu item, replacing Resume; short.
schedules-resuming = Resuming...

# Nonselectable pending menu item, replacing Run now; short.
schedules-starting = Starting...

# Nonselectable pending menu item, replacing Delete; short.
schedules-deleting = Deleting...

# Existing chat target whose cached title is empty; one target line.
schedules-target-chat-untitled = Untitled chat

# No cached target chat; never a claim it was deleted.
schedules-target-chat-unavailable = Chat details unavailable

# Explicit project target whose cached name is unavailable.
schedules-target-project-unavailable = Project details unavailable

# Explicit worktree target whose node path is unavailable.
schedules-target-worktree-unavailable = Worktree details unavailable

# New chat using its agent default folder; no fabricated path.
schedules-target-default-folder = Agent’s default folder

# History clause when the node reports no past run; short.
schedules-never-run = Never run

# List unavailable because the peer has no schedule method.
schedules-failed-older = Update this machine to use schedules.

# Lost connection; safe read retry only, never an automatic write retry.
schedules-failed-offline = Cannot reach this machine. Check its connection, then try again.

# Bounded app request pool is occupied; safe read may be retried.
schedules-failed-busy = Other schedule requests are still running. Try again shortly.

# Reply identity/shape differs from the requested resource.
schedules-failed-unexpected = The machine returned an unexpected answer. Refresh before trying again.

# Local request ids exhausted; no retry button for this failure.
schedules-failed-exhausted = Reopen the app to send more schedule requests.

# Read refusal; distinct from the existing read-only mutation message.
schedules-failed-view-forbidden = This device cannot view this machine’s schedules.

# Node rejected the request; diagnostics stay at the link edge.
schedules-failed-invalid = The machine could not accept this schedule. Refresh and check its settings.

# Device credential no longer accepted.
schedules-failed-unauthorized = Pair this device with the machine again.

# Negotiated peer protocol cannot carry the requested operation.
schedules-failed-protocol = Update the app and this machine so they can connect.

# Node says schedule or target absent; do not guess which one.
schedules-failed-not-found = This schedule or its target is no longer available. Refresh the list.

# Node reports changed state; no automatic mutation retry.
schedules-failed-conflict = This schedule changed or cannot run in its current state. Refresh the list.

# Node explicitly refuses this operation as unsupported.
schedules-failed-unsupported = This machine cannot perform that schedule action.

# Node could not find the scheduled agent installation.
schedules-failed-agent-missing = Install this schedule’s agent on the machine before running it.

# Node could not start the scheduled agent.
schedules-failed-spawn = The machine could not start the agent. Check its installation and folder.

# Node busy/shutting down; explicit retry is possible.
schedules-failed-unavailable = The machine is busy or shutting down. Try again shortly.

# Node failed internally; no raw diagnostic or hidden write retry.
schedules-failed-internal = The machine could not finish that request. Refresh before trying again.

# Typed calendar/catalog formatting failure in a row; no Display text.
schedules-display-unavailable = Some schedule details cannot be displayed. Update the app and try again.

# Elapsed time below one minute, including future timestamps; short.
schedules-age-now = just now

# Floor elapsed minutes, $count numeric; short.
schedules-age-minutes = { NUMBER($count, useGrouping: "false") }m ago

# Floor elapsed hours, $count numeric; short.
schedules-age-hours = { NUMBER($count, useGrouping: "false") }h ago

# Floor elapsed days below one week, $count numeric; short.
schedules-age-days = { NUMBER($count, useGrouping: "false") }d ago

# Older viewer calendar date. $month and $day are numeric date parts; global month names are shared.
schedules-age-date = { formats-month } { NUMBER($day, useGrouping: "false") }

# One complete creation clause; $age is prepared localized elapsed time/date.
schedules-created-age = Created { $age }

# Next-run relative distance below one minute; short.
schedules-countdown-soon = soon

# Rounded next-run minutes, $count numeric; short.
schedules-countdown-minutes = in { NUMBER($count, useGrouping: "false") }m

# Rounded next-run hours, $count numeric; short.
schedules-countdown-hours = in { NUMBER($count, useGrouping: "false") }h

# Rounded next-run days, $count numeric; short.
schedules-countdown-days = in { NUMBER($count, useGrouping: "false") }d

# Complete metadata without next run. Inputs are independently prepared cadence/creation/history phrases; translators may reorder.
schedules-metadata = { $cadence } · { $created } · { $last }

# Complete metadata with next run; $next is the prepared next-run clause.
schedules-metadata-next = { $cadence } · { $created } · { $last } · { $next }

# Multi-machine metadata prefix only. $host is its book display name, $metadata the prepared row phrases.
schedules-metadata-host = { $host } · { $metadata }

# Destructive confirmation body. $title is the displayed schedule title; keep the whole warning together.
schedules-delete-confirm = Delete "{ $title }"? This cannot be undone.

# Destructive confirmation button only; short, distinct from the existing product-specific menu label.
schedules-delete-confirm-action = Delete

# Lost write reply: the node might already have acted. No automatic retry, and no completed result is fabricated.
schedules-write-unconfirmed = This action could not be confirmed. It may have completed. Check the latest state before trying again.

# Machine selector with no match, distinct from an empty schedule filter.
schedules-search-empty = No matching machines

# Short menu trigger tooltip; never the row status or feature title.
schedules-more = Schedule actions

# Complete partial-host error line; the reason is already localized, never a transport diagnostic.
schedules-host-error = { $host }: { $reason }

# Browser build has no node transport; no fabricated schedule data or retry.
schedules-failed-no-transport = Schedules are unavailable in this browser build.

# `HostFilter` picker title; not searchable, as the reference, keep short.
schedules-filter-by-machine = Filter by machine

# Unassigned local feature: no call sent; updating the machine is not promised as a fix.
schedules-failed-not-enabled = This schedule feature is not available in this build.

# Original selected physical admission retired before an operation; no retarget/retry.
schedules-failed-connection-changed = Connection to this machine changed. Refresh before trying again.

# Safe read deadline; a lost write uses the existing unconfirmed whole sentence.
schedules-failed-timeout = The machine did not answer in time. Check its connection, then try again.

# Runnable bucket label and active status badge.
schedules-active = Active

# Filter for schedules that have finished.
schedules-ended = Ended

# A newer node reported a status this app does not recognize.
schedules-unknown = Unknown

# Status badge for an explicitly paused schedule.
schedules-paused = Paused

# Status badge when a schedule has used its run allowance.
schedules-finished = Finished

# Status badge when the schedule reached its expiry time.
schedules-expired = Expired

# Ended badge when the schedule’s credential owner no longer exists.
schedules-creator-gone = Creator gone

# Ended badge when the schedule’s destination no longer exists.
schedules-target-gone = Target gone

# Row notice when the paired device cannot change the schedule.
schedules-read-only = This device can only view this machine’s schedules.

# Row menu action to stop future automatic runs.
schedules-pause = Pause schedule

# Row menu action to allow automatic runs again.
schedules-resume = Resume schedule

# Row menu action to request one run outside the cadence.
schedules-run-now = Run now

# Destructive action and confirmation title for an ordinary schedule.
schedules-delete = Delete schedule

# Destructive action and confirmation title for an agent’s self-owned recurring prompt.
schedules-delete-heartbeat = Delete heartbeat

# Fallback target or cadence label for a value from a newer protocol.
schedules-newer-value = Requires a newer app

# Row metadata about the last run. $age is a localized distance or date.
schedules-last-run-age = Last run { $age }

# Row metadata about the next run. $countdown is a localized future distance.
schedules-next-run = Next run { $countdown }

# Target line naming project $name and its relative folder $dir.
schedules-project-folder = { $name } · { $dir }

# Capacity notice. $host is the machine name and $count is its numeric user-schedule quota.
schedules-list-limit = { $host }: This machine has the maximum of { NUMBER($count) } person-owned schedules.

# Cadence label for once per minute.
schedules-every-minute = Every minute

# Cadence label for once on each hour boundary.
schedules-every-hour = Every hour

# Hourly cron label. $minute is a two-digit minute within each hour.
schedules-hourly-at = Every hour at :{ $minute }

# Fixed-time cron label. $day is a localized day group, $time is HH:MM, and $zone is a node IANA name.
schedules-fixed-at = { $day } at { $time } { $zone }

# Elapsed cadence label. $duration is the full localized interval.
schedules-every-interval = Every { $duration }

# Cadence label for an elapsed interval of one day.
schedules-every-day = Every day

# Day part of an elapsed interval. $count is numeric.
schedules-interval-days = { $count ->
    [one] { NUMBER($count) } day
   *[other] { NUMBER($count) } days
    }

# Hour part of an elapsed interval. $count is numeric.
schedules-interval-hours = { $count ->
    [one] { NUMBER($count) } hour
   *[other] { NUMBER($count) } hours
    }

# Minute part of an elapsed interval. $count is numeric.
schedules-interval-minutes = { $count ->
    [one] { NUMBER($count) } minute
   *[other] { NUMBER($count) } minutes
    }

# Second part of an elapsed interval. $count is numeric.
schedules-interval-seconds = { $count ->
    [one] { NUMBER($count) } second
   *[other] { NUMBER($count) } seconds
    }

# Day group in a fixed-time cron description.
schedules-daily = Daily

# Monday through Friday in a fixed-time cron description.
schedules-weekdays = Weekdays

# Saturday and Sunday in a fixed-time cron description.
schedules-weekends = Weekends

# Weekly Monday recurrence in a fixed-time cron description.
schedules-monday = Mondays

# Weekly Tuesday recurrence in a fixed-time cron description.
schedules-tuesday = Tuesdays

# Weekly Wednesday recurrence in a fixed-time cron description.
schedules-wednesday = Wednesdays

# Weekly Thursday recurrence in a fixed-time cron description.
schedules-thursday = Thursdays

# Weekly Friday recurrence in a fixed-time cron description.
schedules-friday = Fridays

# Weekly Saturday recurrence in a fixed-time cron description.
schedules-saturday = Saturdays

# Weekly Sunday recurrence in a fixed-time cron description.
schedules-sunday = Sundays

# Search aliases for the Schedules destination in Command Center.
schedules-search-keywords = schedules scheduled automation recurring
