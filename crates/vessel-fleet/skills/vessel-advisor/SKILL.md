---
name: vessel-advisor
description: Request one independent second opinion while retaining responsibility for the user's task.
user-invocable: true
---

# Vessel Advisor

Use the user's question and current conversation. Read the vessel skill before
launching one advisor. Honor an explicitly requested profile; otherwise read
available profile notes and choose one suited to planning, review or diagnosis.
Prefer a different provider family when several suitable choices are available.
Use discovered harness/options and disclose a missing-profile fallback.

The advisor starts without this conversation. Brief it with the exact question,
relevant facts and file paths, options already considered, failed attempts,
constraints and the decision needed. Ask for a recommendation with evidence,
risks and reasoning. Preserve restrictions on reads, networks and live systems.

End its briefing with an explicit analysis boundary:

> Analyze and recommend only. Do not edit, create or delete files, implement
> changes, or change running services.

If the user forwards another skill with `/name`, `/skill:name` or `$name`, tell
the advisor to load that skill and pass its remaining arguments as input.
Exclude this advisor invocation itself. Do not claim the skill ran merely
because it was mentioned; assess the advisor's actual report.

Create one `[Advisor]` session with a completion notification. Continue useful
independent checks, then finish your turn to await the notice. Do not poll or
send hurry-ups. Read the response, verify material claims and explain the
advisor's judgment alongside your own decision. You still own implementation.

Keep the same advisor for follow-ups when the user wants continuing advice.
Archive it when that collaboration is done or a different topic needs a fresh
context. Do not archive an advisor the user asked to retain.
