# ADR 0107: Page usage preferences in the bridge preference document

Status: accepted by the coordinator on 2026-11-16 (option (a)); impl U, TRACKER 771 P2.
Extends ADR 0028 (the bridge keeps the page's preferences) for Paseo's usage choices.

## Context

Paseo keeps three usage choices: how percentages read (Used or Remaining), which host the sidebar summary
and modal follow, and which windows are pinned to the summary (usage/preferences.ts). Whether the summary is
shown at all is the sidebar footer item's visibility (usage/in-sidebar.ts), not a usage setting. The native
app copies that split: `view/src/prefs/usage.rs` (display, host by NodeId, pins) and the sidebar navigation's
Usage visibility. The page had nowhere to keep them: each `vessel ui` launch is a new origin, so browser
storage is lost every launch (theme.ts records the same problem), and the bridge document (ADR 0028) has no
usage fields.

## Decision

[Decision Log]
- Purpose: the page's usage choices survive launches and match the native app's meaning.
- Existing assets: the bridge document and its lenient keeper (vessel-web keep/prefs_doc, wire/prefs),
  `PreferenceChange` (one setting per change, so two tabs keep both edits), the native `UsagePreferences`.
- Alternatives: (b) page-local state, lost at every launch; (c) a second file for usage; (d) one combined
  "usage" change carrying all three values, which would let two tabs overwrite each other's choices.
- Choice: a `usage` section in the bridge document shaped exactly like the native one:
  - `display`: `"used"` | `"remaining"`, default used;
  - `host`: a node id or null, default null (follow the active machine, then the first that reports);
  - `pins`: null (follow the sources' defaults) or an explicit list of `{ "source", "window" }`
    (an empty list pins nothing), bounded like the other lists.
  Changes `UsageDisplay`, `UsageHost`, `UsagePins`, one per field. The summary's visibility is
  `SidebarUsage(bool)` in the existing `sidebar` section (default off, Paseo's), beside group/sort/ended.
- Compatibility: a document without the section, or with a field this build cannot read, loads as the
  defaults for exactly that field (the keeper's lenient rule); only fields that were set are written. No
  version change: an older bridge ignores the section and keeps it as written.
- Native: the native test's copy of the page fixture gains the same section, so both documents keep one
  meaning for the shared fields.

## Consequences

- vessel-web Rust (wire types, keeper, tests) and the generated `web/src/wire/bridge.ts` change; the page
  reads display, pins and host from preferences instead of component state.
- The page then draws the sidebar summary (Paseo sidebar-item.tsx, N2's quotaPinnedLayout) and the usage
  modal (usage-modal.tsx) from these values.
- Pins name source and window ids from the node's reports, never labels, so a renamed account keeps its pins.
