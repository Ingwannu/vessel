# Opt-in field context requests and original-window delivery

[Decision Log]
- Purpose: reuse the desktop kit menu for actual input right-clicks, retaining
  original recipient identity across deferred input/host callbacks.
- Existing assets: `TextField` owns the same `gpui-base` line/textarea engines;
  their public `on_context_menu` callback runs after releasing the input. The
  desktop already owns one weak window registry and one kit menu renderer.
- Constraints: G2 is editing the field factories/event module. This producer
  must not edit those files, add a clipboard engine, change input methods or
  introduce another renderer/window list.
- Alternatives: intercept root mouse events and guess the editor; copy editing
  behavior; change the dependency fork; or register one global default callback.
- Choice: an explicit `TextField::set_context_menu_handler` implemented in a new
  child module installs the existing callback on that field only. The callback
  emits typed permissions, position and weak original focus; its weak live probe
  rereads mask/selection/editability before use. A separate host adapter retains
  the original registry record and defers until the Window borrow ends.
- Why: actual input engines remain authoritative; neither current focus nor a
  live replacement window can become an unrelated original recipient.
- Consequences: only opted-in fields replace their configured/native input menu
  with the host's kit menu. Unconfigured fields keep their prior callback. This
  does not claim that every field or terminal has opted in. Callers still own
  domain/action authority and the actual menu operation; there is no transport,
  session, document-save or clipboard authority in this UI request.

## APIs and data flow

`kit::field::context_menu::{Request, Facts, CopyPermission, PastePermission,
Editing, Probe, Handler}` crosses the UI boundary without text or clipboard
contents. `TextField::set_context_menu_handler(handler, cx)` works on line,
composer and multiline form presentations through the same generic input state.
`Request::facts(cx)` reads current permissions; `current(window,cx)` requires the
original weak focus to remain focused and the live editor to remain enabled.

`app::host_windows::context_menu::handler(Callbacks)` returns that handler.
Callbacks contain caller admission and the existing menu-opening operation.
The real E3 caller must supply its one `desktop_commands_admitted` recorder
predicate and its existing kit menu, rather than another shortcut listener or
clipboard implementation. Deferred delivery checks the exact original window,
one matching current record, weak Shell, direct/wrapped root, original input
focus/live probe, then caller admission. It never routes to a focused successor
or creates a window. The callback returns true only when its actual popup opened;
the adapter then acknowledges selection focus through the same engine's weak
`set_selection_focus` hook. This matters because the additional host defer occurs
after the engine's automatic synchronous focus capture. A refused open does not
acknowledge selection focus. A host callback must preserve source/domain guards on body
actions; the registry grants UI identity only.
On the menu's admitted matching close/retirement, its owner must call
`Request::release_selection` before reusing the shared popup focus. Otherwise an
old editor can remain highlighted when that same focus handle serves a different
editor's popup. Retained close callbacks must pass the existing menu lease and
caller guards first; this request is not a second popup generation manager.

Paseo reference: pinned `1c00b365c9e0b74c1580eba33d905d693125ae21`,
`packages/desktop/src/features/menu.ts:205-228` terminal-context
Copy/Paste/Select All vocabulary; browser form inputs retain their platform input
behavior. This native field opt-in uses that same edit vocabulary. The
native GPUI input/clipboard semantics are reused instead of Chromium IPC.

## Serialized field hookup and proof boundary

This leaf changes no `field.rs`, `field/events.rs`, factory, presentation, masked
field or accessibility implementation. The exact one-line module registration
is supplied **unapplied** in `field-context-menu-hookup.patch`. The method and
engine wiring already live in the new child; there are no event/factory hooks to
copy. The lead applies this patch after G2's in-flight field producers, preserving
their module/export union, and reruns narrow checks on the receiving line.

The App host module is literally `cfg(test)` until an actual E3 startup/field
consumer promotes it. The leaf's VM evidence must run on a separate scratch
branch with the exact field hookup patch applied. That scratch-only hookup
commit is never delivered/merged. Report the executed scratch hash/tree and the
source leaf separately; do not call the unhooked leaf standalone-compiling.

Tests use actual line/composer/form mouse events and their engine callback,
current readonly/masked facts, unchanged defaults and weak input retirement.
Host tests use actual direct/wrapped Shell roots and original-window focus,
root/record replacement, closed recipient and caller-admission refusal. These
host cases prove generic callback routing, not a real shortcut recorder or menu
consumer. Source/remote tests/V1/lead adoption, actual E3 field opt-ins, captures,
OS behavior and final package review are all separate obligations.

The first ordinary scratch run
`wp-desktop-host-narrow-20261116T021145Z-62812fcd`, with the hookup applied,
passed fonts/fmt/quality, then stopped in Kit Clippy at eleven missing public
variant/field documentation diagnostics. Tests did not run. The correction adds
those descriptions without weakening lints. A later source addition also exposes
the admitted-close selection release and tests it; neither change inherits a
passing runtime result. The next scratch check must include both changes.

The corrected receiving leaf merges announced package base 14.3 before its next
scratch run. The unapplied hookup patch is rebased only around the already-landed
composition module; no field factory or event code is authored by this leaf.

## Quota-stop resume

Corrected leaf source is `b75f0750b` on announced base14.3 `67637eb6`.
Evidence-only scratch `6eaf813fba1791c1c3c8abdda3a3ea19d516d162` applies the
exact included hookup patch. The sole corrected ordinary selected run
`wp-desktop-host-narrow-20261116T033714Z-6eaf813f` was registered before the
quota warning and is queued; no Clippy/runtime result exists yet. Keep its
records and retrieve it before resubmitting anything. This docs checkpoint
changes no native source. The scratch is NEVER delivered or merged.

Next: inspect that result, correct only actual findings, and send V1 a bounded
leaf plus the unapplied patch and explicitly qualified WITH-HOOKUP-APPLIED
scratch evidence. Lead applies the patch after G2's field changes and runs the
receiving narrow. Host product registration, real field opt-ins, stale actual
menu/recorder/selection-release consumer evidence and E3 captures remain owed.
No renderer, clipboard engine, authority or all-fields/default behavior was
changed. No further merge or VM run starts during the quota-stop window.
