# Keep dialog dismissal attached to its decision

[Decision Log]
- Purpose: Escape, the outside scrim and the close glyph must dismiss the
  decision their dialog was drawn for, without consulting a mutable current
  selection that may already belong to another connection.
- Existing asset and constraint: the shared kit dialog already owns all
  three paths and the adaptive layout. Its function-pointer callback could
  not retain an immutable caller context and confirmation identity.
- Alternatives: another dialog renderer, a global mutable callback target,
  or an erased callback engine. Each adds a second owner or loses the target.
- Choice: modify the existing `dialog` callback to a clonable closure and
  pass its clones to the same three existing handlers. The private header
  accepts the same callable shape. Function-item callers continue to work.
- Why: the kit remains ignorant of application types and owns one renderer;
  the feature can compare the captured decision before changing its state.
- Costs: the feature must still implement and test stale-decision policy.
  This shared change alone establishes no Devices or pairing security pass.

## Reference and compatibility

Paseo 0.15.4 `hosts/host-confirmation-sheet.tsx:21–26,31` closes through a
callback for the pending request. `runtime/host-confirmation.ts:47–53` only
answers a matching request id, and `:73–80` cancels the older request when
another is asked. Devices applies the same lifetime rule to its existing
confirmation. No reference code or assets are copied.

The shared dialog's geometry, focus, animation, layering, Escape propagation,
mouse occlusion and invocation ordering stay unchanged. Ordinary function
items remain accepted. The close glyph gets a debug selector for measuring
and clicking the actual production element in a regression; it changes no
layout or release behavior. No dependency, public domain type or alternate
control is added.

## Verification and resume point

The regression passes an owned captured token through the actual dialog and
activates Escape, the scrim and the close glyph separately. The existing
short/long body geometry tests remain. Narrow local short-lane Kit Clippy
exited zero; Kit tests compiled the changed crate and passed 113 tests,
zero failed, zero ignored. The three-path captured-decision regression
passed. Logs are `dialog-kit-clippy-01.log` and `dialog-kit-test-01.log` in
the resumed track evidence; the cold test build took 8m32s within the
unchanged 600-second short-lane bound. No whole gate or captures ran here.
The final E2 package owns those. Devices additionally owes stale context
and confirmation tests at its own callback boundary.
