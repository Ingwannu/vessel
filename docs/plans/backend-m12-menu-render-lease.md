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
state or propagation. The existing key routing moves into menu/input.rs to keep the one draw module
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
Latest source execution and review remain pending. No whole gate, visual or
review pass is claimed. No local Cargo/Rustc is permitted;
all compilation runs through vessel-remote-gate on the build VM.
