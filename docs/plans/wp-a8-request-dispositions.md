# A8 staged Requests failure dispositions

Status: source-only owner extension on announced app base14.2 c6d8df49. No
new build, wire adapter, product staging promotion or authority proof. W12 owns
sole registry/bundle/wrappers; this patch changes only W4 owner files and tests.
The earlier3eaa/d186 bounded pass remains proof of that earlier source only.

[Decision Log]
- Purpose: a correlated request which never dispatched must release only its
  original local Sending flag; a started/ambiguous failure must retain its hold.
- Existing assets and constraints: the installed Requests Vec owns binding,
  ticket and local Stage. Its old Effect retained Request id alone, discarding
  Form's actual FieldId/selected permission option after preparation. settled()
  removes a definitive Sending correlation and is not a failure classifier.
- Alternatives: call settled() for every failure (loses uncertainty), derive
  NoSend from Changed/Closed (pre/post-await share those errors), or reset the
  currently selected form (can revive a replacement or clear another option).
- Choice: retain the SAME original Form in each existing Effect; derive its
  Request rather than keep another id field. Add typed owner-local Disposition
  and Transition, preserving the old definitive settled() boundary unchanged.
- Why: the sole owner can consume the original canonical ticket before the local
  transition, while W4 can compare the exact original editor/option before reset.
  No second map, ticket allocator, cloned area/editor or transport capability is
  introduced. The native field's value, drafts and engine are never rewritten.
- Effects: NoSend removes only its effect and returns Released or Unchanged.
  Release requires the same still-pending request and actual FieldId/selected
  option; replaced/withdrawn/already-cleared forms remain untouched. Uncertain
  retains the complete original Form and Stage::Uncertain with no retry/clear.

## Trusted boundary and limits

Disposition is NOT proof, a guard, a receipt or permission authority. The real
transport/preflight producer must prove that dispatch never started before
choosing NoSend. The original connection/binding/ticket must be correlated by
W12; Changed/Closed alone cannot classify an attempted call as never sent.
Guardless local refusal may install no success or authoritative network data.
Started/ambiguous failures use Uncertain; a guarded rich NoEffect refusal is a
separate future adapter, never guessed from a legacy error or a reread.

The intended order is:

1. Validate original binding and Requests::settling correlation.
2. Consume the sole canonical Request ticket exactly once.
3. Inside that successful settle callback, call Requests::failed with the
   trusted disposition and the SAME SessionResources.conversation.
4. Interpret Released as local Sending release only: server Pending, actual
   eligibility, current Visible/Focused input and a new ticket are still needed.

The helper itself cannot consume or mint the canonical ticket. This uses the
same trusted-callback precondition as settled(); W12 alone owns the production
wrapper. No W12 module, field, vote, minter, wrapper or guard is edited here.
All inherited cfg(test) gates remain. Real never-dispatched proof, rich-state
Pending eligibility, delivery/receipt validation and live transport remain held.

## Authored checks; execution still owed

Eight new GPUI cases use the installed Requests field in actual SessionResources,
the existing opening minter and canonical ticket table, and a real TextField.
They cover retained drafts, explicit retry with a new ticket, duplicate/late
callbacks, uncertain completion refusal, replaced FieldId, withdrawn request,
replaced permission option, preservation of an independent question, foreign
operation counts and automatic retirement uncertainty. The transport phase is
seeded logically; these cases do NOT prove that any wire call never dispatched.
The fixture uses SessionView::new only as an isolated test factory, not the
production scoped composer or a provider. No physical keypress/caret/IME, painted
rows, clipboard, lease/delivery, RPC, native capture or full A8 result is claimed.
The existing four request fixtures keep every assertion; only three helper
visibilities change so the new cases reuse their schema/callback definitions.
