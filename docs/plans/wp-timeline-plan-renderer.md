# Shared Plan Markdown renderer prerequisite

Isolated parent: announced app line `074ade16`. This bridge contains no A7
timeline/receipt/turn state or A8 permission/question implementation. The app
file is intentionally unregistered until its first actual caller; that caller
adds `pub(crate) mod plan_card;` to `areas/conversation/mod.rs` in its integration.
No dead-code allowance or alternate renderer is needed.

[Decision Log]
- Purpose: the approval dock and resolved stream must use one Plan Markdown body.
- Constraint: A8 owns request identity, pending state, disclosure and actions;
  displaying a plan must never infer approval or send a mode change.
- Alternatives: duplicate Markdown/card surfaces or make a renderer own actions.
- Choice: caller supplies `PlanCard` data, a typed outcome/expansion, a toggle
  callback and an optional footer slot. The same cached Fluent title adapter and
  kit Markdown renderer serve both callers.
- Why: content/geometry have one source while domain decisions remain with A8/A7.
- Effects: this bridge has no wire dependency or network effect. The caller must
  capture its exact request/resident identity in the callback before mutation.

API: `conversation::plan_card::{PlanCard, card}`. Card fields are `id`, `text`,
optional `title`/`description`, `outcome`, `expansion`, optional `footer`.
`card(card, on_toggle, words: TimelinePlan<'_>, tokens, window, cx) -> AnyElement`.
Use `shell.words.timeline_plan()`, not the legacy `ConversationWords` table.
`view::timeline::{PlanOutcome, PlanExpansion}` are the only shared state values;
the outcome is not an authorization result inferred by this renderer.

Source evidence: renderer file SHA256
`cd27aeb02137d82e683d1b9f4e51d6d31985e085cb1a4bc884d100fc7360122a`
is byte-identical to A7 `f8aaaa1d`, whose app/view/kit all-target strict Clippy
passed in `wp-timeline-narrow-20261115T081535Z-f8aaaa1d`. The Plan title adapter
is byte-identical to A7 `9fbda65af`, whose view all-target tests passed1059/0/0.
Its complete EN/KO title regression is retained here. The isolated split itself
has diff/size checks only; A8 must run its exact first-caller integration check.
This is source-prerequisite readiness, not a V4 package approval or UI verdict.
