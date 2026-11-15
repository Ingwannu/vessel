## Project operations are bound to the explicitly selected host and registry id.
# Confirmation title, short.
projects-remove-title = Remove this project?
# Whole warning; current legacy node refuses removal while resources still refer to it.
projects-remove-description = Remove the project from Vessel. Its directory stays on the host. Close active sessions and remove its worktrees first.
# Confirmation action, short.
projects-remove = Remove project
# Confirmation cancellation, short.
projects-cancel = Cancel
# A pending node mutation may have completed; this must never promise rollback.
projects-outcome-unknown = The host did not answer in time. Refresh the project list to check whether the change completed before trying again.
# Local dispatch validation, whole sentence.
projects-host-unavailable = The selected host is unavailable. Reconnect it before changing this project.
# Ticket exhaustion is a local refusal, no node action was submitted.
projects-identifiers-exhausted = Project operation identifiers are exhausted. Restart the app before trying again.
# Field label above exact host identity, short.
projects-host = Host
# Refresh after external project changes or an uncertain mutation, short.
projects-refresh = Refresh

# Local bounded tracking refuses new dispatch without forgetting earlier pending work.
projects-reconciliation-full = Too many project changes need reconciliation. Wait for pending requests and retry failed changes before adding more.
