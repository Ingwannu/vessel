## Settings General: existing labels and delivery explanations.

# Settings navigation and General page title. Short.
general-title = General

# Heading over the send preference rows. Short.
general-sending = Sending

# Label of the physical send-key picker; the chord is typed data. Short.
general-send-shortcut = Send shortcut

# Label of the default delivery-mode picker. Short.
general-default-send = Default send

# Queue delivery-mode choice. Short.
general-queue = Queue

# Steer delivery-mode choice. Short.
general-steer = Steer

# Interrupt delivery-mode choice. Short.
general-interrupt = Interrupt

# Heading over local preference storage. Short.
general-kept = Kept

# Label of the preference storage row. Short.
general-preferences = Preferences

# Status when preferences last only until this app quits. Short.
general-this-launch = This launch only

# Whole hint for queue delivery, including the permission-wait exception.
general-queue-hint = A message sent while the agent works waits, and goes when its turn ends; while a permission request waits, it stops the turn instead.

# Whole hint for steer delivery, including its unsupported-agent fallback.
general-steer-hint = A message sent while the agent works joins its turn, where the agent can take one; otherwise it stops the turn and starts the next.

# Whole hint for interrupt delivery.
general-interrupt-hint = A message sent while the agent works stops its turn and starts the next.

# Whole storage hint. $path is a display path; keep its own line.
general-kept-file =
    Kept by Vessel on this computer, so every launch opens with them.
    { $path }

# Whole explanation when this surface has no place to keep preferences.
general-kept-no-place = Kept until Vessel quits: this computer gives Vessel no place to keep them.

# Whole safe explanation; an arbitrary path-discovery diagnostic is not UI text.
general-kept-no-path = Kept until Vessel quits: the preferences location could not be determined.

# Whole explanation for a newer preferences schema. $path is a display path.
# $version is a numeric schema version, displayed without grouping.
general-kept-newer = Kept until Vessel quits: { $path } was written by a newer Vessel (version { NUMBER($version, useGrouping: "false") }); it is left as it is.

# Safe fallback if the typed storage message cannot be prepared before paint.
general-storage-unavailable = Preference storage information is unavailable.

# Frame notice after copying the preference file path.
general-path-copied = The path was copied.

# Whole unreadable-file recovery. $problem is closed; $path is a display path.
general-kept-unreadable = Kept until Vessel quits: { $path } could not be read ({ $problem ->
        [not-toml] this is not TOML (look for an unclosed quote or bracket, or a line without `=`)
        [not-json] this is not JSON
        [unknown-key] a key this file does not take
        [duplicate-key] a key given twice
        [missing-key] a key this file needs is missing
        [wrong-token] `token` names the file that holds the token, token = { "{" } file = "…" { "}" }, never the token itself
        [wrong-value] a value of the wrong kind
       *[cannot-open] it cannot be opened (check who owns it and its mode)
    }). Fix or remove it, then restart Vessel.

# Whole wrong-type recovery for a known field. $field names only a reader-approved key.
# $path is a display path.
general-kept-unreadable-field = Kept until Vessel quits: { $path } could not be read (`{ $field }` has a value of the wrong kind). Fix or remove it, then restart Vessel.

# Whole unreadable-file recovery. $problem is closed; $path is a display path.
# $line and $column are numeric source positions, displayed without grouping.
general-kept-unreadable-at = Kept until Vessel quits: { $path } could not be read (line { NUMBER($line, useGrouping: "false") }, column { NUMBER($column, useGrouping: "false") }: { $problem ->
        [not-toml] this is not TOML (look for an unclosed quote or bracket, or a line without `=`)
        [not-json] this is not JSON
        [unknown-key] a key this file does not take
        [duplicate-key] a key given twice
        [missing-key] a key this file needs is missing
        [wrong-token] `token` names the file that holds the token, token = { "{" } file = "…" { "}" }, never the token itself
        [wrong-value] a value of the wrong kind
       *[cannot-open] it cannot be opened (check who owns it and its mode)
    }). Fix or remove it, then restart Vessel.

# Whole wrong-type recovery for a known field. $field names only a reader-approved key.
# $path is a display path.
# $line and $column are numeric source positions, displayed without grouping.
general-kept-unreadable-field-at = Kept until Vessel quits: { $path } could not be read (line { NUMBER($line, useGrouping: "false") }, column { NUMBER($column, useGrouping: "false") }: `{ $field }` has a value of the wrong kind). Fix or remove it, then restart Vessel.
