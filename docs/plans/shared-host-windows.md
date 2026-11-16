# Shared native host windows

Paseo references: packages/desktop/src/window/desktop-window-owner.ts:42-125
selects focused, then visible, then first window, and creates one through its
owner when none exists. Native GPUI additionally needs weak Shell/root identity
because the window's content can be replaced without changing the OS handle.

[Decision Log]
- Purpose: desktop menus, lifecycle admission and system notification callbacks must use one window owner.
- Constraint: the first window exists on the package base before the rest of E3; a handle may close or keep a different root while an older Shell remains alive.
- Alternatives: one list per area, importing all E3 persistence/startup, or retaining strong Shell entities.
- Choice: move E3's sole WindowRecord list to app::host_windows and retain weak Shell recipients. The base's actual first-window creation registers once.
- Why: host callbacks can share identity and liveness without node, preferences, updater or CLI dependencies.
- Consequences: source-only producer operations stay staged until their actual W7/E3 callers join; absent factory or live recipient returns explicit Unavailable. The registry conveys UI identity, never connection/session/ticket authority.

The E3 move-only commit0934a674 removes Desktop.records and updates its existing
close/persistence readers; no copy of that persistent list remains. This isolated
leaf starts on announced82ca07a7 and carries only the host module/first-window
registration, documentation and W7's bounded kit root-owner helper993c60b5.
It does not contain E3 preferences, menu, CLI, node startup or session navigation.
E3's existing private close-coalescing field is an owner extension of this same
record, not another live-window list or another close manager.

Structure: register initializes one process Global and one window-close observer,
deduplicates the same handle+Shell, and replaces the record for an explicitly
replaced recipient. Closing a window removes its record. All records are weak.
Routing filters live handles and verifies kit::root_contains_view through W7's
single wrapper boundary, then chooses active, visible, or the first valid hidden
window. It repeats root/weak checks immediately inside the selected handle update
before invoking the caller. Temporary local selection snapshots are not a second
registry. Call only from App level, after any mutable Shell borrow ends.

API under app::host_windows:
- register(&Entity<Shell>, &Window, &mut App), real base first-window caller.
- with_focused_shell(&mut App, impl FnOnce(&mut Shell,&mut Window,&mut Context<Shell>)) -> Delivery.
- open_additional(&mut App) -> Delivery.
- set_factory(fn(&mut App)->Delivery,&mut App), desktop-owner startup only.
- Delivery::{Delivered,Unavailable}; no factory and no valid root are explicit Unavailable.

The dispatcher module/reexport and Registry.factory each have literal cfg(test)
staging. W7 removes the dispatcher/reexport/factory-field attributes with its real
generic fallback caller. If promoting the module for W7 first, split set_factory
out of the grouped reexport, keep that individual reexport behind cfg(test), and
put only the set_factory function behind cfg(test) until the E3 startup consumer
is present. The other three delivery exports and factory field become product
code with W7's actual caller. No dead-code
allowance, fake registration or public-visibility workaround is permitted.
E3 reuses this owner and removes the same staging with its actual desktop menu,
launch, lifecycle and factory consumers; it must not recreate Desktop.records.

Tests are installed for direct/wrapped roots, duplicate registration, retained old
Shell after root replacement, close/dead recipient and successor handle, active /
visible / hidden fallback, absent factory and an actual factory-created window.
All five cases executed successfully in the selected continuation described below.
Root identity is not caller source/context authority;
W7 separately checks its opaque notice route and original source before actions.
No OS notification delivery/permission, native capture, IME/a11y, E3 parity or
historical Linux focused-field F5 claim is made by this control producer.

First selected run328b8bba passed fonts/fmt/quality, then strict Clippy stopped at
four style diagnostics: one missing documentation backtick and three unit-return
callback semicolons. No tests ran. Correcting only those diagnostics does not
constitute a runtime result; the continuation uses the ordinary lane, with the
first priority record retained. No suppression or assertion change is added.

Exact source d8b733972f406c65c42a09758abcba34bc30e0d3 passed ordinary selected
run wp-desktop-host-narrow-20261115T170147Z-d8b73397: fonts, formatting, quality,
all-target strict Clippy and tests all returned zero. The raw aggregate was
1490 passed / 0 failed / 2 ignored, including App 108 / 0 / 2, Kit 204 / 0 / 0
and all five host-window cases plus the root-owner helper. The logs show the
changed Kit/App being checked and compiled. This is selected source evidence;
review, final consumer union and the lead's combined gate remain separate.
