# Menu render lease correction (S2)

## Scope and decision

The coordinator and V1 returned the shared menu at `dd13ce378` because a
retained rendered frame A can mutate opening B before the application's
captured-action guard runs. The renderer, not each area, owns admission.
This isolated correction preserves the 8158/6ece public menu APIs, placement,
geometry, QUICK, modal Layer, KeepOpen, menu_right and G4 ReturnFocus policy.
Direct references read: Paseo `components/ui/menu/menu-context.ts:135–177`
closes before actions and preserves closeOnSelect=false; `menu-item.tsx:291–296`
selects through that one context; `menu-root.tsx:34–46` owns one menu state.
The allocation lease is a native retained-GPUI-frame safety correction, not a
claim that Paseo exposes this token API.

Measured Start placement and G1's selected/description rows remain subsequent
hierarchy work, not part of this defect correction.

[Decision Log]
- Purpose: make every retained input callback belong to exactly one explicit
  opening, before highlight, close, focus restoration or action dispatch.
- Existing assets: one MenuState, one renderer and GPUI's retained frame/input
  handlers already provide all required lifecycle and event dispatch; extend
  them rather than add area guards, a new popup, dependency or event listener.
- Alternatives: integer generations can wrap; an address without ownership can
  be reused; an application action check occurs after destructive menu effects.
- Choice: each new state and explicit open/open_end owns a fresh Rc allocation;
  rendered callbacks retain only a Weak lease. Closed state rejects admission.
  Weak allocation identity cannot be recycled while a retained callback holds
  its header, so replacement/opening has no counter wrap or ABA reuse.
- Why: close remains const and keeps its API. Closing invalidates admission
  through is_open; reopening replaces identity. An already-authorized Close
  choice may restore focus and dispatch once using identity after close, with
  another identity check after restoration; a later callback cannot enter.
- Impact: one allocation per explicit opening, bounded by retained frames; no
  polling, new physics, platform behavior or persistence. Opener capture on
  reopening an already-open menu deliberately remains unchanged.

## Data flow and verification

MenuState creates the lease; draw captures it independently for key, outside,
row mouse-down and row click handlers. Each checks admission before changing
state or propagation. The existing key routing moves into menu/input.rs to keep
the one draw module
below its size budget; it is not another renderer. GPUI track_focus installs a
default pointer-focus listener: the same panel uses capture_any_mouse_down to
prevent that default only when its lease is stale, including replaced state.
The key input bundle carries the same captured items,
handlers and lease. No area receives an alternative action authorization path.
Future delayed hierarchy mutations must carry and validate this same lease.

`kit/src/menu/lease_tests.rs` dispatches real GPUI platform input against rendered
frame A inside one outer window update after opening B. A render-count assertion
proves B has not redrawn. Close and KeepOpen cover pointer, outside, down/up/Tab,
Home/End, Enter/Space/Escape, focus ownership, no actions, unchanged highlight,
and valid B action after redraw. Pointer/outside also cover in-place reopening,
close/reopen and measured open_end; key checks cover close/reopen and in-place
reopening. A real kit TextField is the return-focus target;
state replacement exercises old pointer/outside callbacks and allocation identity.
State-only close also dispatches actual retained pointer/key/outside callbacks
and verifies its deliberate no-focus-restoration behavior remains unchanged.
These are headless GPUI regressions, not Linux native F5, OS keyboard or complete
accessibility proof. The historical focused-field native F5 hold remains open.

## Status

First production checkpoint 4919d7cc6 passed exact Kit all-target Clippy;
82a4f5004 Kit test execution was 167 passed / 4 failed. It exposed GPUI's default
pointer-focus listener on state replacement, corrected by the panel capture
guard. Key dispatch also draws a dirty window before interceptors; the corrected
regressions open B inside a real interceptor after A is drawn, then let the actual
A key listener execute. A render-count boundary proves there was no draw between
opening B and that listener. No copied callback or test-only product hook is used.
Exact corrected code 47d74dbe8380138b498257dc508a794b4fcc63be:
- Kit all-target strict Clippy rc0, run
  `shell2-lease-20261115T122708Z-47d74dbe`,
  `/tmp/work/m12-shell-s2/menu-lease-clippy-2`; actual changed Kit Checking.
- Kit --lib tests 171 passed / 0 failed / 0 ignored, run
  `shell2-lease-20261115T122919Z-47d74dbe`,
  `/tmp/work/m12-shell-s2/menu-lease-tests-2`; actual changed Kit Compiling and
  all five new lease tests explicitly ok. Fonts and command steps both rc0.
- Existing tests, item/trigger/public geometry and focus-policy sources are
  unchanged. Size/boundary checks pass with the existing 14 size exceptions.

This documentation-only result update changes no gated native source.
V1 review is pending. No whole gate, capture, OS, or full F4 approval is claimed.
No new captures are made for this unchanged menu geometry; headless input
execution is the correction evidence. No local Cargo/Rustc ran: all compilation
went through vessel-remote-gate on the build VM. Development files remain owned
by ubuntu; no service permissions, production data or process was changed.


## V1 return: retain nested input containment (2026-11-15)

V1 accepted the direct lease/default-focus reasoning but returned47d for one
parent-path hole: GPUI captures the dispatch path before its interceptors.
Opening B there leaves A's parent dialog on that path. A rejected Escape must
therefore remain contained, or it can close the parent underneath B. This is a
shared input-routing correction, never an area action workaround.

The corrected router parses the same recognized menu keys into one private
enum and consumes those keys on a stale lease, without state, focus, action or
notify changes. Unknown keys still propagate. AboveDialog's obsolete outside
callback also retains its existing modal containment while refusing dismissal;
valid callbacks keep their prior close/restore/contain order.

A new nested fixture mounts the actual shared dialog with a real TextField,
measured menu trigger and AboveDialog layer. Its real interceptor opens B before
A's real key listener (both themes, Close/KeepOpen, all eight recognized keys).
Assertions require parent and B/focus to survive, no ancestor key/action, no
redraw between B and A's listener; valid B Escape closes only B and next Escape
reaches the parent. Old pointer/outside presses also preserve both layers.
Latest correction source and these regressions are unverified until the next
remote narrow results. The earlier47d171/0/0 is prior-source evidence only;
V1 cycle-safe handoff HOLD and historical native F5 HOLD remain open.
