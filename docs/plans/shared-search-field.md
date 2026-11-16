# Shared standalone search field

[Decision Log]
- Purpose: Providers catalog and model filtering need Paseo's standalone search
  surface, whose clear affordance belongs to the control.
- Existing assets: TextField, its unstyled bare renderer, identified editing
  engine, focus/listen, and original Search and Remove (Lucide X) vectors.
- Constraints: one input engine and one shared surface; no area chrome or
  competing field writer. W4 identity then G2 multiline were reviewed before
  this additive control.
- Alternatives: style an ordinary form in each area, create a separate editor,
  or accept a multiline field and silently render it as a search box.
- Choice: SearchField privately constructs only the existing one-line TextField,
  renders its bare editor and owns Search/X drawing and clearing.
- Why: input methods, selection, clipboard and undo remain the existing engine's
  responsibility. Ordinary field, composer, multiline/identified callbacks and
  minimum-height APIs remain unchanged.
- Impact: no new dependency, RPC, preferences or OS action. Programmatic clear
  keeps its original silent reset; the person's X uses replace_all so the
  owning query receives the ordinary Change and the edit remains undoable.

## Source and API

Pinned Paseo components/ui/search-field.tsx, whole file (field/input 92–122):
Search14/X14 with inherited1.5 stroke; horizontal12/vertical6/gap8; max420;
input20; radius.md6; surface1/border at rest, surface2/borderAccent focused.
X has hitSlop8 with no new hover/pressed fill. One source geometry in both
layouts: no invented44-point wrapper. The existing native bare input uses
Type::Body metrics; text/control scaling follows the shared font policy.

kit::SearchField::new(placeholder,on_change,window,cx), text(cx),
set_placeholder(placeholder,window,cx), clear(window,cx), and
element(id,tokens,cx). Enter is not a submission API. Callers provide their
localized placeholder and only ordinary state-change callbacks; no external
clear icon or second editing state. Element identity is caller supplied.

## Verification status

Source installed after the checked G2 baton; current timestamps are used.
Both-theme literal returned-style tests and actual engine keyboard/clear/focus/
wide/narrow bounds tests are added. They are uncompiled until the exact changed
head remote narrow completes. Expected constants are source literals, not the
implementation constants alone. X must emit one empty Change, keep focus, remove
its target after clearing and remain inside the narrow field. Locale placeholder
changes preserve editor identity. No native hover pixels, OS accessibility,
keyboard access to X, actual OS IME or consumer UI pass is claimed. Existing
kit-wide keyboard/accessibility gaps are not solved by this drawing control.
No unchanged whole gate or screenshot matrix is requested for the producer;
consumer first-use and final A9 evidence remain separate.


The first exact2e3 Kit Clippy returned101 solely for a missing unit-statement
semicolon in the last test assertion. The sequential runtime stage did not
start. Add that semicolon only; no product body, assertion, expected geometry
or deadline changes. Old red evidence remains. Corrected exact-head Clippy
and focused tests are still owed, without priority or a whole-gate claim.


Exact2b999 strict Kit all-target Clippy completed0 at
agents-control-20261115T143804Z-2b9994ac. The focused runtime at
agents-control-20261115T150042Z-2b9994ac returned1 passed/1 failed.
The real engine emitted one Change for each grapheme typed by simulate_input,
then exactly one empty Change from X. The test incorrectly expected one
aggregate typing event. Assert the entire literal prefix history and the final
empty event; do not suppress callbacks or change the product. All focus, target,
geometry, localization-identity and Enter assertions remain. The later checks
were not reached in that failed test, so their execution remains unverified.
Old red evidence is retained; corrected focused checks and adoption remain owed.


## Exact corrected producer results

Exact13c46693 strict all-target Kit Clippy0 at
agents-control-20261115T152445Z-13c46693; focused runtime2 passed/0 failed/0 ignored
(184 unselected tests) at agents-control-20261115T155040Z-13c46693. Both runs
completed native-fonts0/cargo0 without skips and actually checked/compiled Kit.
The latter exercises both themes and the later narrow/Enter/placeholder-identity
checks that the old red never reached. The original queued focused run was
cancelled with verified143 only after the coordinator approved one same-source
priority replacement for G2's blocked shared-field writer. No priority is carried
to another head or feature. Old2e3 and2b999 failures remain separate evidence.

Clippy steps SHA256:
5e92d8cf468f6301d0506a6c6ce3c4bd09cc937077e879832e37800a1c02b848.
Runtime steps SHA256:
18d3f4b5fa74d128f245ba8f73eb9a626ec420db38a30efe347f0e42d1cd22ec.

V1 disposition and selected-producer adoption remain the next boundary; this is
not a full Kit suite, consumer UI, actual OS IME/accessibility, hover-pixel or
whole A9 result. Only selected2e3f05bf +2b9994ac +13c46693 belong to the control;
never consume intervening A9 changes or the complete branch as its producer.
After checked handoff, G2 becomes the sole next field writer for Code form text.


## Additive identified-search leaf

The retained label-manager consumer requires the existing engine's FieldId,
because an old queued editor event may arrive after close/reopen. Add only
SearchField::with_source, id, focus and set_text, delegating to that same
TextField. Plain new/clear/render stay unchanged. Actual-engine tests cover both
themes' Search/X geometry, silent restoration, focus and source-tagged clear,
and a forced old-engine event delivered after replacement with an unchanged
current query. No consumer transition, new renderer or other field module is
included. This leaf remains uncompiled/unreviewed until its remote checks finish.
