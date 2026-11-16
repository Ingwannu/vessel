# Typed terminal key and paste encoding (staged T2)

[Decision Log]
- Purpose: native key events and the future CLI frame display must consume the
  same authoritative InputModes without duplicating keyboard negotiation.
- Existing assets: the pure engine owns decoded modes; the old view owns the
  ordered sender but has no key encoder. CLI textual send-keys is a separate
  pinned grammar/static expansion and remains with its existing owner.
- Alternatives: mode checks in each UI or CLI edge, a second escape scanner,
  xterm frontend dependency, or guessing modes before a frame is available.
- Choice: extend vessel-terminal-screen with one pure typed key/paste encoder.
  It consumes explicit existing InputModes, returns bounded bytes only and does
  no clipboard, process, socket, focus, IME, repeat or delivery admission work.
- Why: one lower-level rule can serve both display clients. InputModes come
  from the existing engine, never a string scan or copied state tracker.
- Effects: this adds no parser, runtime/dependency, DTO/call, renderer or
  NodeAuthority activation. Real admission and single ordered PTY writer stay
  with their owners. No actual attach/native input is claimed by pure tests.

## Exact reference behavior

Paseo protocol/src/terminal-key-input.ts1-264 is the actual key encoder.
Printable single UTF16-code-unit keys use shift uppercase, Ctrl mapping and
an Alt ESC prefix. Named keys are a closed native enum after edge parsing;
unknown key names are an explicit edge refusal rather than silent empty data.
Supplementary characters and composition are committed text, not physical
named-key guesses. Text commits preserve UTF8 without applying key modifiers.
Shift uppercase expansion follows the reference Unicode string behavior;
finite non-ASCII oracle coverage does not establish universal Unicode parity.

Unmodified arrows use SS3 only in application-cursor mode, otherwise CSI;
modified arrows use CSI1;modifier. Home/End/Insert/Delete/Page/F1-F12 use the
reference modifiers. Modified Enter uses Win32 only for Shift/Ctrl/Alt when
Win32 is active, ahead of Kitty; Kitty requires any modifier and nonzero flags.
Meta-only with Win32 therefore does not imply Win32 Enter. Without enhanced
admission Enter remains CR. Shift-only Tab is reverse Tab; other Tab and
Backspace use only the Alt prefix; Escape is ESC. The named key path never
infers a terminal shortcut/IME/focus policy from these output bytes.

Paseo app/src/terminal/runtime/terminal-paste.ts1-41 preserves plain paste,
and bracketed paste replaces each embedded ESC[201~ with [201~ before wrapping
with ESC[200~/ESC[201~. This prevents an injected closing marker from escaping
that mode. Clipboard empty handling and async focus/opening receipts stay at
the actual caller; empty encoding itself remains deterministic.

## Bounds, ownership and verification

Keys use small fixed bounded output. Bulk committed text and paste accept an
explicit caller byte budget before allocation; this pure API does not invent a
second transport size ceiling. A typed budget/refusal preserves caller input,
and no prefix is returned for a refused single operation. A caller must encode
against a qualified same-cut frame, retain the original intent on refusal and
apply its existing opening/connection/size/input admission and ordered writer.
No auto-retry or input-delivery claim is provided here.

Next: retain an independent oracle from the actual unmodified functions across
all modifiers/mode combinations, then typed tests and implementation, exact
changed-crate remote tests/strict checks. Current engine whole remains frozen
in its separate tree; no readiness or whole-gate result is borrowed from it.


## Staged exact source receipt

Exact665288f9 passed51 tests with zero failures/ignores (the existing48
engine tests plus three input suites) and strict all-target Clippy with warnings
denied. Runs termscreen-narrow-20261116T024725Z-665288f9 and
termscreen-query-20261116T024725Z-665288f9 actually compiled/checked this crate.
All6016 original-key oracle cases and10 original-paste cases are asserted,
including all16 modifier combinations, app-cursor and Win32 switches, Kitty
on/off, casing expansion, embedded paste-end markers and exact byte budgets.
Committed Unicode text and unsupported physical-key refusal are separate tests.

The firstf600 source already passed51/0/0; strict rejected missing error docs,
unchecked bounded arithmetic, similar names and wildcard error mapping. Those
original logs are preserved. The correction uses checked lengths/code conversion
and bounded modifier bit masks, with unchanged oracle inputs/expectations.
No lint waiver or weakened assertion is introduced. Manifests/dependencies/lock
and engine/vendor interpretation are unchanged. This final follow-up is docs only.

Public staged API: Key::try_from external key names; Modifiers with typed
Modifier states; encode_key(Key,Modifiers,InputModes); encode_text(text,
InputBudget); encode_paste(text,InputModes,InputBudget), returning InputBytes
or typed InputError. Budget is explicit caller byte admission, not another
transport limit. This remains a proposed pure producer for the later D1 union:
no enclosing package gate/review, actual native/CLI caller, clipboard/IME/OS,
PTY delivery or NodeAuthority activation is claimed.
