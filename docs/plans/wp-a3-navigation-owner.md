# Portable navigation owner boundary (staged)

Base: announced package base8e, `82ca07a79`. This bounded producer does not
include full A3, prompt-index values, RPC changes or another Searching value.

[Decision Log]
- Purpose and intent: the sole resident snapshot owner needs one conservative
  navigation vote for the main ConversationArea and every shown Origin area.
- Existing assets and constraints: this base has FindBar's actual TextField,
  query, sliced Finder, generation, matches and at-address; the legacy outline
  Pointer has candidate/attention/wait; ConversationArea has a shared pending
  reveal and recursive Origin::Shown. It has no new prompt metadata/jump models,
  FindPhase or remembered closed query. None is invented here.
- Alternatives: returning Ready until full A3 arrives hides real pending work;
  importing the whole feature or adding empty substitute controllers expands
  ownership and makes the core depend on unexecuted unrelated code.
- Choice: literal test-only navigation_held/navigation_retired methods beside
  existing find ownership. Hold an open find field, any shared pending reveal,
  pending/visible hover, or those states in any shown Origin. Retirement clears
  this exact area's Finder/matches/refresh/at, invalidates its existing generation,
  and uses the same Pointer leave cancellation as the production hover path.
- Why: preserves real input/selection/field identity and original counters. It
  cannot turn a retired open editor or unresolved other-owner reveal into Ready.
- Advantages, costs and effects: no controller, registry, snapshot, follow or
  admission issuer is added. There is no production exposure or global cutover.
  Pointer leave is an extraction of its existing body with identical behavior.

The shared `reveal` also carries inbox/other owners' requests. This hook keeps
it; the corresponding core retirement/settlement must release it. A7 owns
Origin::Wanted/Reading/loading and its other readiness votes. This hook recurses
all actual Origin::Shown areas rather than treating them as empty or Ready.

This producer does not close the base's globally looked-up asynchronous callback
scope. A live resident cutover still needs the sole session context/binding and
owner callback routing. Snapshot Hold is required while real navigation exists;
these methods do not authorize dropping it, defaulting a new controller, or
promoting a live getter. Full A3 must preserve this base vote and extend it with
its actual metadata/jump/hover fields at a checked consumer union. No public
failure enum is invented for the old FindBar: its input remains open and Held.

Authored owner fixtures cover the real input engine's selection and subsequent
replacement after local hidden-resource retirement, active-field isolation,
retained shared reveal, and two nested origins containing actual hover/reveal
state. They are source only until execution. Static fmt, size, tokens and
whitespace checks passed; strict App Clippy and filtered navigation runtime
checks are pending the one-own-build lane. No full A3 or product runtime claim.
