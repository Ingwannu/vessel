## Schedule-list labels and feedback.
## Existing cadence, state/action and editor-error words remain in the legacy
## schedule table until its separately owned migration. Never duplicate them here.

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


## Prepared relative time and metadata

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

# Older date in explicit UTC fallback; $month/$day numeric.
schedules-age-date-utc = { formats-month } { NUMBER($day, useGrouping: "false") } UTC

# One complete creation clause; $age is prepared localized elapsed time/date.
schedules-created-age = Created { $age }

# One history clause. $age localized distance, $outcome from the accepted result table; node result only.
schedules-last-result-age = Last run { $age } ({ $outcome })

# Next-run relative distance below one minute; short.
schedules-countdown-soon = soon

# Rounded next-run minutes, $count numeric; short.
schedules-countdown-minutes = in { NUMBER($count, useGrouping: "false") }m

# Rounded next-run hours, $count numeric; short.
schedules-countdown-hours = in { NUMBER($count, useGrouping: "false") }h

# Rounded next-run days, $count numeric; short.
schedules-countdown-days = in { NUMBER($count, useGrouping: "false") }d

# Next clause. $at typed date/time already in viewer zone, $countdown localized distance. DATETIME names its zone.
schedules-next-known = Next run { $countdown } · Your time: { DATETIME($at) }

# Next clause when platform zone is unavailable; typed $at is explicitly UTC.
schedules-next-utc = Next run { $countdown } · { DATETIME($at) } · Your time zone is unavailable

# Adjacent rule-zone identity when it differs; $zone is a validated rule IANA name, never machine zone metadata.
schedules-rule-zone = Rule’s zone: { $zone }

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

# Bundled zone lookup failed; no guessed offset or diagnostic.
schedules-clock-zone = This time zone is unavailable. Update the app to display times in it.

# A platform zone identity cannot be represented by the app.
schedules-clock-name = The time zone name cannot be displayed. Update the app.

# Typed display-calendar conversion failure.
schedules-clock-range = This date is outside the calendar’s display range.

# Machine selector with no match, distinct from an empty schedule filter.
schedules-search-empty = No matching machines

# Short menu trigger tooltip; never the row status or feature title.
schedules-more = Schedule actions

# Complete partial-host error line; the reason is already localized, never a transport diagnostic.
schedules-host-error = { $host }: { $reason }

# Next-run clause with a different rule zone; references retain their typed arguments.
schedules-next-known-rule = { schedules-next-known } · { schedules-rule-zone }

# Explicit UTC viewer fallback plus the separate rule zone.
schedules-next-utc-rule = { schedules-next-utc } · { schedules-rule-zone }

# Paused cron metadata still names the rule zone without promising a next run.
schedules-metadata-rule = { schedules-metadata } · { schedules-rule-zone }

# Browser build has no node transport; no fabricated schedule data or retry.
schedules-failed-no-transport = Schedules are unavailable in this browser build.

# `HostFilter` picker title; not searchable, as the reference, keep short.
schedules-filter-by-machine = Filter by machine

# Unassigned local feature: no call sent; updating the machine is not promised as a fix.
schedules-failed-not-enabled = This schedule feature is not available in this build.

# Cadence preset picker labels; not the saved row’s full cadence description.
schedules-preset-minute = Every minute

# Hourly cadence preset picker label; not a saved cadence description.
schedules-preset-hour = Every hour

# Daily-at-nine cadence preset picker label; keep short.
schedules-preset-daily = Daily 9:00

# Monday-to-Friday-at-nine cadence preset picker label; keep short.
schedules-preset-weekdays = Weekdays 9:00

# Monday-at-nine cadence preset picker label; keep short.
schedules-preset-mondays = Mondays 9:00

# Original selected physical admission retired before an operation; no retarget/retry.
schedules-failed-connection-changed = Connection to this machine changed. Refresh before trying again.

# Safe read deadline; a lost write uses the existing unconfirmed whole sentence.
schedules-failed-timeout = The machine did not answer in time. Check its connection, then try again.

## Form validation, separate from accepted cadence/error tables

# Whole field error; numeric maximum comes from the typed node bound.
schedules-form-name-invalid = Use a short name on one line, without control characters.

# Whole field error; numeric maximum comes from the typed node bound.
schedules-form-prompt-invalid = Enter a prompt of at most { NUMBER($maximum, useGrouping: "false") } UTF-8 bytes, without control characters except tabs and line breaks.

# Whole field error; numeric maximum comes from the typed node bound.
schedules-form-max-runs-invalid = Enter a whole number from 1 to { NUMBER($maximum, useGrouping: "false") }, or leave it blank for unlimited runs.

# Required structured chat target; a missing selection blocks saving.
schedules-form-target-required = Choose an agent and a folder for this schedule.

## Node-only editor preview failures

# Whole typed preview failure; never show wire message/Display.
schedules-preview-mismatched = The machine returned a different or incomplete preview. No run times are shown.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-unknown-zone = This machine does not know that time zone. Choose another zone or update the machine.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-unknown-cadence = Update this machine before previewing this kind of schedule.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-no-future-run = This rule has no three future runs within the machine’s supported range.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-out-of-range = The selected time is outside this machine’s supported range.

# Whole typed preview failure; never show wire message/Display.
schedules-preview-unknown = Update the app to read this machine’s preview refusal.

# Save waits for the exact current node preview, except an explicit older-node refusal.
schedules-form-preview-required = Preview the current rule before saving.

## Read-only schedule history

# Read-only row-menu entry opening past runs; short.
schedules-history-menu = View runs

# History dialog title; $title is the original displayed schedule title.
schedules-history-title = Run history · { $title }

# History has loaded successfully but no run is retained.
schedules-history-empty = No runs yet

# Close the read-only history dialog; short.
schedules-history-close = Close

# Explicit history reload; short.
schedules-history-refresh = Refresh

# Run row button; present only if its outcome names a session.
schedules-history-open-chat = Open chat

# Run heading; $at is typed viewer date/time, $number numeric run number.
schedules-history-run-at = Run { NUMBER($number, useGrouping: "false") } · { DATETIME($at) }

# Typed skip reason; no diagnostic data.
schedules-history-previous-running = The previous run is still active.

# Typed skip reason; no diagnostic data.
schedules-history-previous-waiting = The previous prompt is still queued.

# Typed skip reason; a stopped chat cannot restart automatically.
schedules-history-stopped = The chat was stopped and was not restarted automatically.

# Typed missed slots; $count numeric from node.
schedules-history-missed = Missed { NUMBER($count, useGrouping: "false") } scheduled times while the machine was unavailable.

# Forward-compatible reason in run history.
schedules-history-reason-unknown = Update the app to read this run’s reason.

# Historical `InvalidRequest`; no wire diagnostic.
schedules-history-run-invalid = The machine rejected the run’s input.

# Historical Unauthorized; no wire diagnostic.
schedules-history-run-unauthorized = The credential used for this run was not accepted.

# Historical Forbidden, not a claim about current viewer scopes.
schedules-history-run-forbidden = The identity that created this schedule could not perform the run.

# Historical `ProtocolMismatch`.
schedules-history-run-protocol = The run used an incompatible protocol.

# Historical `NotFound`, not guessed target deletion now.
schedules-history-run-not-found = A resource needed by this run could not be found.

# Historical Conflict.
schedules-history-run-conflict = Another operation prevented this run from starting.

# Historical Unsupported.
schedules-history-run-unsupported = The machine did not support an operation needed by this run.

# Historical `HarnessNotInstalled`.
schedules-history-run-agent-missing = The required agent was not installed.

# Historical `SpawnFailed`.
schedules-history-run-spawn = The agent could not start.

# Historical Unavailable.
schedules-history-run-unavailable = A service needed by this run was unavailable.

# Historical Internal; no private diagnostic.
schedules-history-run-internal = The machine could not complete this run.

# History fallback heading when a typed calendar/catalog error prevents prepared rows.
schedules-history-heading = Run history

# A machine supports schedule create/update but predates the read-only preview call.
schedules-preview-unavailable-older = This machine cannot preview this rule. You can save it without a preview.

# Schedule form labels follow schedule-form-sheet.tsx409–537/706–813 and cadence-editor.tsx.

# Schedule editor: form-new-title control/feedback, never a node diagnostic.
schedules-form-new-title = New schedule

# Schedule editor: form-edit-title control/feedback, never a node diagnostic.
schedules-form-edit-title = Edit schedule

# Schedule editor: form-heartbeat-title control/feedback, never a node diagnostic.
schedules-form-heartbeat-title = Edit heartbeat

# Schedule editor: form-create control/feedback, never a node diagnostic.
schedules-form-create = Create schedule

# Schedule editor: form-save control/feedback, never a node diagnostic.
schedules-form-save = Save changes

# Schedule editor: form-name control/feedback, never a node diagnostic.
schedules-form-name = Name

# Schedule editor: form-prompt control/feedback, never a node diagnostic.
schedules-form-prompt = Prompt

# Schedule editor: form-host control/feedback, never a node diagnostic.
schedules-form-host = Host

# Schedule editor: form-project control/feedback, never a node diagnostic.
schedules-form-project = Project

# Schedule editor: form-model control/feedback, never a node diagnostic.
schedules-form-model = Model

# Schedule editor: form-thinking control/feedback, never a node diagnostic.
schedules-form-thinking = Thinking

# Schedule editor: form-mode control/feedback, never a node diagnostic.
schedules-form-mode = Mode

# Schedule editor: form-archive control/feedback, never a node diagnostic.
schedules-form-archive = Archive on finish

# Schedule editor: form-limit control/feedback, never a node diagnostic.
schedules-form-limit = Max runs

# Schedule editor: form-optional control/feedback, never a node diagnostic.
schedules-form-optional = Optional

# Schedule editor: form-prompt-placeholder control/feedback, never a node diagnostic.
schedules-form-prompt-placeholder = What should the agent do each run?

# Schedule editor: form-unlimited control/feedback, never a node diagnostic.
schedules-form-unlimited = Unlimited

# Schedule editor: form-cadence control/feedback, never a node diagnostic.
schedules-form-cadence = Cadence

# Schedule editor: form-zone control/feedback, never a node diagnostic.
schedules-form-zone = Time zone

# Schedule editor: form-next-runs control/feedback, never a node diagnostic.
schedules-form-next-runs = Next runs

# Schedule editor: form-host-placeholder control/feedback, never a node diagnostic.
schedules-form-host-placeholder = Select host

# Schedule editor: form-project-placeholder control/feedback, never a node diagnostic.
schedules-form-project-placeholder = Select project

# Schedule editor: form-model-placeholder control/feedback, never a node diagnostic.
schedules-form-model-placeholder = Select model

# Schedule editor: form-zone-placeholder control/feedback, never a node diagnostic.
schedules-form-zone-placeholder = Choose a time zone

# Schedule editor: form-custom control/feedback, never a node diagnostic.
schedules-form-custom = Custom

# Schedule editor: form-interval control/feedback, never a node diagnostic.
schedules-form-interval = Interval

# One projected run; $at is the typed viewer-calendar date/time, with its explicit zone.
schedules-form-next-at = { DATETIME($at) }

# New-rule bootstrap when the viewer cannot report its own zone; never silently infer UTC.
schedules-form-viewer-unknown = Your time zone is unavailable. Choose a time zone for this rule.

# A rule zone has been explicitly selected but the display viewer zone is still unknown.
schedules-form-viewer-display-utc = Your time zone is unavailable. Preview times are shown in UTC.

# Read-only existing-session target, schedule-form-sheet.tsx909.
schedules-form-target-label = Target

# Project identity plus its node-authored directory, if a form target uses both; $dir/$project are display text, never local path joins.
schedules-form-target-project = { $project } · { $dir }

# Local typed preview configuration failure; no node call was sent.
schedules-form-preview-configuration = The app could not prepare this schedule preview. Update the app and try again.

# Hidden until the single typed Vessel schedules documentation destination exists.
schedules-see-docs = See docs

## Schedule form choice search and empty feedback

# Placeholder for searching project choices on the selected machine; short.
schedules-form-search-projects = Search projects...
# Placeholder for filtering the current provider/model browser; short.
schedules-form-search-models = Search models...
# Placeholder for filtering offered thinking-option choices; short.
schedules-form-search-thinking = Search thinking options...
# Placeholder for filtering offered session modes; short.
schedules-form-search-modes = Search modes...
# Placeholder for filtering IANA time-zone names; short.
schedules-form-search-zones = Search time zones...
# Project choice list has no matching offered project; not a transport diagnostic.
schedules-form-empty-projects = No projects found
# Model choice list has no offered model; provider read errors use separate feedback.
schedules-form-empty-models = No models found
# Thinking choice list has no matching offered option; short.
schedules-form-empty-thinking = No thinking options found
# Session-mode choice list has no matching offered mode; short.
schedules-form-empty-modes = No modes found
# Time-zone choice list has no matching IANA name; short.
schedules-form-empty-zones = No time zones found
# Fallback empty filtered choice list; short, not a failed node read.
schedules-form-empty-choices = No matching options
# Example five-field cron expression in the form; preserve digits, spaces and operators.
schedules-form-cron-placeholder = 0 9 * * *

# Source model-browser feedback; short loading/retry/no-matching-model labels.
schedules-form-model-loading = Loading
# Retry only the failed provider-model read; short button label, never repeats a write.
schedules-form-model-retry = Retry
# Disabled retry button while that provider-model read is pending; short.
schedules-form-model-retrying = Retrying...
# Successful model catalog/search with no matching models; not a provider error.
schedules-form-model-no-matches = No models match your search

# Local preference persistence failed before the schedule RPC; no node action occurred.
schedules-failed-preferences = Could not save these model choices. No schedule was sent.
