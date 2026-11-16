# Import history row parity follow-up

Status: root-free row presentation implemented for narrow validation on accepted
base14.1. The prompt-preview producer remains queued; no wire behavior changes.

The coordinator compared the accepted picker captures with pinned Paseo
components/import-session-sheet.tsx:340–368 and requested this separate follow-up:
provider icon, a one-line title with activity time at its right, a two-line prompt
preview, and a one-line folder ending in an ellipsis. The390-point capture of the
existing row clips the folder at the viewport boundary. Folder ellipsis is the
minimum acceptance requirement for this follow-up.

[Decision Log]
- Existing assets: the sole app agent_mark adapter already maps HarnessRef to
  reviewed embedded artwork. The canonical history Scope supplies that harness;
  Entry supplies title, updated_at and cwd. Shared GPUI text layout supplies
  line_clamp/text_ellipsis; VirtualList owns variable row sizing. Existing captured
  visit/scope callbacks and explicit selection/import behavior must be preserved.
- Constraint: ADR0053 and proto::history::HistoryEntry deliberately expose no
  first/last prompt preview. Native Entry has no such field either. The current
  title must not be relabeled as a fetched prompt or synthesized transcript.
  Paseo getPromptPreview (view-model.ts:238–244) itself has an unavailable fallback.
- Alternatives: a second file/transcript scanner violates the one agent.history
  intake boundary; fetching/loading every native conversation changes the listing
  resource contract and may start expensive provider work. Fabricating preview
  content from the title is misleading. A character-count path crop ignores the
  actual font, viewport and Unicode shaping.
- Choice: reuse the existing artwork and text layout for a bounded, root-free UI
  follow-up first. Constrain the row/content/title/path to the actual list width,
  retain the activity label without allowing a long title to displace it, and use
  actual one-line text ellipsis for the folder. A two-line unavailable preview
  may represent the existing missing metadata honestly. Real previews require a
  separately reviewed extension of the existing history producer, including
  finite byte/read budgets, account binding and old-peer projection; no invented
  wire call or independent scanner is authorized by this UI note.
- Impact: the root-free fix can close the icon/header/path layout differences;
  real prompt-preview parity stays explicitly pending until the canonical reader
  carries it. Provider/source identities, resume/import, account/connection fences,
  selection, permissions and automatic-open parking are unchanged.

Validation must include390x844 and1280x900 with a long folder and long title,
known/unknown activity, and both selection states. Inspect actual rendered
ellipsis rather than infer it from a style method. Run narrow App checks and the
existing import retained-input regressions on the receiving source; do not claim
provider previews or whole A4 from a layout-only result. Capture/review is scoped
to this later row change, not borrowed from the accepted popup captures.

## First presentation implementation

The existing row now fills the list viewport, its flex content is explicitly
constrained, the title shares one header with a non-shrinking activity label,
and the folder uses one-line clamped text with an ellipsis. The shared muted
agent mark comes from the same captured history scope. Selection and existing
restore action metadata remain independent from the provider mark. A localized
unavailable preview occupies the shared two-line-clamped preview area; actual
provider text is still missing at the canonical read boundary, not invented.

This source has not yet passed its receiving narrow checks or visual inspection.
The accepted picker captures do not prove these later row changes.
