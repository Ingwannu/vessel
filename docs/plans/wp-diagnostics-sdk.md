# Selected diagnostics SDK adapter (staged leaf)

This leaf starts at announced app package base 13. It contains only the
SelectedConnection SDK adapter and memory-only framed regressions; E4 view,
actor/receipt/UI adoption and G4 Scenario/export changes are excluded.

[Decision Log]
- Purpose: preserve exact admitted connection identity during the SDK's bounded
  diagnostics operation, including closure before polling and after completion.
- Constraints: generic Unary call cannot retain the SDK's typed identity and
  timeout errors. SelectedConnection already owns retirement, physical-close
  notification and pre/post completion fences.
- Alternatives: use GetDiagnostics directly, duplicate the SDK or add a second
  guard. These would split identity/deadline/admission ownership.
- Choice: invoke NodeClient::diagnostics inside the existing guard. SDK Call
  failures enter the same existing CallError mapping; identity and timeout stay
  typed within its result. No raw-error formatter, resolver or epoch is added.
- Consequences: call, guard, follow, protocol getter, delivery and exports remain
  unchanged. This unused adapter is literally cfg(test), matching staged shared
  producers: E4 removes that staging attribute only with its real consumer and
  subsequent product checks. This leaf does not activate app collection.

The canonical SDK retains its ten-second deadline, welcome/report identity
check, protocol and Read admission. Tests use the existing private framed peer
and selected state fixture, no G4-only Scenario API or live daemon. They cover
successful report, identity mismatch, semantic denial, retirement, physical
closure notification and held-response timeout. Narrow VM checks are owed at
initial submission; whole app, UI/copy/privacy and producer approval are not
claimed by this bounded source packet.
