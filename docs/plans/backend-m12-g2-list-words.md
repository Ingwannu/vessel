# New list words and retained accepted tables

[Decision Log]
- Purpose: the schedules list follows the generated Fluent API from its first
  new label, including complete metadata sentences and typed count/date values.
- Existing implementation and constraints: cadence, action/state and local
  editor error words were accepted in the Rust schedule table. Their separate
  migration belongs to the translation track; duplicating them is forbidden.
- Alternatives: extend the Rust tables or migrate accepted words in this slice.
- Choice: new schedules.ftl resources and exhaustive view label choices. Existing
  cadence/action/state/editor errors keep their original table and caller path.
- Why and cost: one source per phrase and build-checked generated contracts;
  dynamic messages must be prepared on data/language/clock changes, not painting.

Reference: Paseo schedules-screen.tsx225–249/303–329 for loading, failed/empty
states and Try again; schedule-row.tsx269–327 for pending action labels; shared
schedule-format/time helpers for reference distances. English follows the
reference. Korean is written as Korean; the feature name stays 예약.

New catalog resources contain list filters' empty states, machine selector
labels, pending feedback, truthful unavailable targets, transport/refusal copy,
relative time and whole metadata/deletion clauses. The generated API propagates
numeric month/day arguments through the existing global formats-month reference
and typed Date through DATETIME, rather than passing numeric/date strings.
The Rust locale module only chooses between already prepared plain labels.

Accepted cadence/action/status/editor grammar/error tables remain unchanged in
this commit. Known read-only mutation refusal will call its accepted table;
the separate read denial has its own new catalog message. No page/screen,
route, sidebar control, unimplemented action or duplicate notice is introduced.
The Banner profile is still unassigned and no area override is drawn.

Narrow verification passed: two test functions execute all eight portable
empty/pending choices in both languages plus full typed numeric/date reference
composition; selected View tests and all-target View Clippy both returned zero.
View was actually compiled. Formatting, native size, boundary and token checks
also returned zero. The complete merged native A13 gate remains owed. This is
not a completed list, full native gate, screenshot or product interaction pass.
The corresponding list/transport/cache models remain external preparation.

The first narrow cases compiled the generated catalog and passed empty-state
and typed numeric/date composition. Its pending case rejected the draft Korean
starting caption: it omitted the action noun. The resource now says 실행 시작 중...
as the original expected contract required, without weakening that assertion.
The cases now live in renderer-independent list-words.json with stable kebab-case
ids and exhaustive test-side enum mappings, including absent unwired Edit.
The failed run is retained separately. The changed-resource rerun passed; its
portable cases preserve every earlier empty/pending expectation and add the
explicit absent Edit case. No first-run failure is described as a passing run.
