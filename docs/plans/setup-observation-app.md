# Setup observation app boundary

The canonical values carrier `8272859f` arrived through announced package base
`db42d9c5`. The app now uses those exact invocation, ordinal, duration and
observation/cancel types. This is a view/link foundation, not an active RPC,
workspace bootstrap grant or completed setup panel.

[Decision Log]
- Purpose: show actual command results without parsing combined terminal text,
  and keep cancellation acknowledgement separate from command completion.
- Existing assets: canonical proto values, Reading/RequestKeys, the shared
  ConnectionContext, the legacy setup log and the kit renderer.
- Alternatives: duplicate setup enums, infer commands from printable log markers,
  or copy an unreviewed runtime into the app tree.
- Choice: use one pure model and a typed backend boundary tested in memory.
  The default backend is Unavailable and selects/polls no transport. It echoes
  the original address, request key and typed operation even when unsupported.
- Why: a backing WorktreeId is historical observation identity. It is neither
  canonical WorkspaceId nor permission to start/release/bootstrap anything.
- Effects: a real adapter requires the reviewed runtime in an announced base,
  negotiated capability, physical admission, authorization, deadline and decoded
  snapshot limits. These are not supplied by this values-only foundation.

Paseo setup-panel.tsx 104–118 supplies the final-carriage-return-segment display;
99–103 supplies whole-unit duration formatting, and 164–265 supplies automatic
expansion with manual collapse. The producer here includes planned Waiting rows,
so automatic expansion chooses the running or last attempted row rather than an
unattempted trailing command. A new invocation resets manual expansion because
the same ordinal can name a different command; the reference retains its Sets
for the mounted component. Within an attempt manual collapse remains stable.

Absence and unknown values never mean success. A cancellation button captures
the displayed invocation and requires a fresh running read plus independent
backend support; Requested is not process death. Controller retirement and
original pending-cancellation retention still belong to the later area adapter.
The canonical scalar DTO by itself does not validate a complete observation;
no network renderer or local DTO validation is claimed by these fake tests.
