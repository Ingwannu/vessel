# A8 caller companion for the shared Clone Seat

Status: source-only companion on announced base10,3e7820e3. It requires W10's
new Seat producer and is NOT standalone compile-ready on the old fn-field Seat.
Do not adopt or claim a gate before the exact producer plus companion union is
compiled and reviewed. No full A8 branch, node contract or product cutover is
included here.

[Decision Log]
- Purpose: permission/question callers must retain the complete captured
  session context when the shared Seat changes from Copy fn fields to Clone.
- Existing assets and constraints: W10 owns the ONE Seat declaration/extraction,
  checked ResidentSeat getters and row/origin processors. A8 owns its permission,
  question, dock and Place answer/tool callers. Existing fields and controls
  stay unchanged; no second registry, identity cache or cloned area is needed.
- Alternatives: retaining raw fn fields would lose an admitted scoped context;
  using the selected global on stale/missing data would retarget the callback.
- Choice: modify/reuse existing callers: borrow Seat through get/get_mut and
  clone the captured carrier for each dispatch of the reusable plan toggle.
  This is Rust ownership adaptation, not a new control or external API.
- Why: get/get_mut borrow their result from Shell only, so temporary
  Place::SEAT constants still work. Cloning the carrier preserves its exact
  context without moving it out of a multi-shot callback.
- Benefits/costs: no behavior change for explicit Legacy SESSION/ORIGIN paths;
  admitted stale/missing data still returns None with no global fallback.
  The producer and this companion must be checked together.

## Exact dependency contract

Seat is Clone, NOT Copy. The producer supplies:

- get<'a>(&self, shell: &'a Shell) -> Option<&'a ConversationArea>
- get_mut<'a>(&self, shell: &'a mut Shell) -> Option<&'a mut ConversationArea>
- origin(&self) -> Seat derives ONE level from this exact parent, recursively;
  missing/stale data never becomes global ORIGIN.
- legacy(read_fn,write_fn) const constructor and existing SESSION/ORIGIN
  constants for explicitly unconverted owners; Place::SEAT remains const.
- admitted_main(SessionContext), literal cfg(test), delegates checked staged
  getters. Parent carriers use Arc; no area clone or second manager.

This patch does not edit the producer declaration, parent exports, row/toggle/
origin processors or any staging attributes. Dock has no raw getter on this
base and needs no no-op edit. Only existing calls in permission.rs, question.rs,
question/card.rs and Place::tool change; plan toggle clones per dispatch. The
answer boundary still passes its owned Seat unchanged to the existing actor.

## Verification and limits

Local pinned rustfmt, size, tokens and diff checks are the available static
preflight. No Cargo is run locally. Combined producer/caller strict compilation
and meaningful existing card/clone callback tests remain owed on the build VM.
No new test merely mirrors these mechanical replacements. No scoped input,
request/lease authority, field visit, global/resource cutover, keyboard/IME,
full A8 or whole native acceptance is inferred.
