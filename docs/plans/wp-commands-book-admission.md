# Shared client-book UI admission (bounded producer)

Status: changed-source App narrow checks passed on announced package base14.1
b54abbdc; bounded source review and lead receiving adoption remain owed. This is not
whole F4, onboarding chrome, node/connection authority or a UI/OS acceptance.

[Decision Log]
- Purpose: pinned Paseo1c00 app/_layout.tsx398/416 enables global shortcuts only
  when paired daemons exist, with an explicit Settings exception. The coordinator
  assigns this one UI predicate to W22; S2 adds no parallel gate.
- Existing fact: native link/refresh.rs awaits saved_entries successfully before
  publishing Fleet. Book failures emit a failure and return. Every configured
  entry is retained, including Machine::unreachable. shell/events.rs sets the
  sticky loaded flag after that first successfully installed snapshot. Publication
  may be delayed by connection refresh; the fact is not online readiness.
- Choice: Shell::client_book_read(&self)->bool exposes that existing fact without
  another cache/writer. Shell::global_shortcuts_enabled(&self)->bool is Settings
  OR (client_book_read AND at least one configured entry). Viewport600/mobile
  availability remains the existing separate rule.
- Consumers: the sole native interceptor, dispatcher candidate availability and
  activation, and badge eligibility. One registration line on Fleet installation
  retires disabled chord/held/badge/capture state before later re-admission.
- Alternatives: selected-session or welcome inference, an online-host gate, a
  second book cache, or separate S2/W15 predicates. Each changes the reference
  condition or creates inconsistent readiness. W15 consumes only the read fact
  for its independently owned onboarding/chrome presentation.
- Costs and limits: this adds no RPC, protocol, persistence or resource lease.
  Configured offline entries enable UI keys; feature actions still require their
  exact owner admission. No glyph/menu/renderer/global keyboard listener is added.

Existing enabled keyboard fixtures explicitly seed a configured offline book,
with no NodeId/protocol/connection lease. New actual GPUI key cases exercise
unread/empty/offline and Settings, owner-dispatch bypass refusal, and pending
chord retirement on book loss. The test harness change preserves the intent of
existing IME, local menu, capture and sidebar/desktop stand-in regressions.

The full F4 command center separately retains native nested-page Back while
only global keys are disabled. This portable base's older palette has no such
page operation; no newer palette/controller/renderer is copied into this leaf.

At exact159f685660a4d0fc9f4a4ce044811c1d99156f78, source preflight reports no
findings and the native size check reports769 files/13 inherited oversized
files, with no budget increase. Run
wp-commands-narrow-20261116T030513Z-159f6856 passed App all-target Clippy with
warnings denied. Run wp-commands-narrow-20261116T031122Z-159f6856 recompiled
the actual App binary and passed19/0/0,148filtered, including all three new
book cases and the desktop canonical-binding/recorder stand-in. This final
document-only successor has identical native source; no unchanged suite is
repeated. These are narrow results, not full F4 or live book-I/O/OS evidence. Take
only this bounded leaf after review and the lead's announced receiving green;
never use a moving full F4 branch for this getter.
