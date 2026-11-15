# Prepared schedule time and row content

[Decision Log]
- Purpose: preserve Paseo's cadence → creation → history → future order with
  human distances, plus the explicitly required viewer/rule-zone names.
- Existing assets: generated typed Date/Number arguments and accepted cadence,
  status, action and result tables remain the single sources for their words.
  The global Ago helper rounds elapsed values, exposes seconds, and keeps days
  indefinitely; changing it would change unrelated screens.
- Alternatives: raw UTC/Display text, chrono platform zone arithmetic, changing
  shared Ago, or restoring the duplicate local cron preview implementation.
- Choice: pure reference Age/Countdown types; Jiff 0.2.39 with std and
  tzdb-bundle-always exactly as the node's locked rules; typed Gregorian parts
  enter Fluent only after conversion. Unknown viewer identity is labeled UTC.
  The original next_due and state always remain node-authored.
- Why and cost: the same bundled rules avoid OS zone-data drift. The raw bundled
  concatenated-zoneinfo.dat in jiff-tzdb 0.1.9 is 206,788 bytes; that is data
  length, not an executable-size delta (linker/compression/code are separate).
  A regression compares both lockfiles' version and checksum for Jiff/tzdb.

References read whole: Paseo utils/time.ts:1–84 and utils/time.test.ts past-age
cases; utils/schedule-format.ts/its formatNextRun tests; schedule-row.tsx:100–146
for metadata order. English reference inputs/outputs are retained; Korean whole
phrases live in schedules.ftl. Stable-id ages.json can also be used by a future
page. Calendar conversion has gap/fold/midnight and chrono-range edge cases.

Prepared content owns shared Arc text and is compared before asking for a frame.
Formatting and calendar conversion run when data, language or visible relative
text changes, never in paint. Target identities use the separately tested target
resolver; optional provider identity is never guessed from a missing session.

Necessary differences: Vessel shows the node's last result (requested feature),
keeps exact interval remainders, names the viewer's zone beside next_due and the
rule's own zone where different, and explicitly labels unknown-viewer UTC.
A paused cron names its rule zone without promising a next run. Node state
replaces Paseo's local deletion/expiry inference. No second cron evaluator.
New list/metadata/feedback words use generated schedules.ftl; accepted cadence,
product/action/status/editor errors remain in text/schedules.rs pending L's move.

This checkpoint is not a visible screen, a whole native gate, or UI approval.
Narrow source/test results are appended when they finish; final list first-use
captures and its one A13 gate remain package completion requirements.

Fluent keeps its existing directional isolates. Portable age cases now retain
both reference-visible words and exact Fluent byte strings in English/Korean;
raw production output is asserted as well. A test-only visible-text comparison
removes only FSI/PDI for the reference words. Production code never strips them.
Project-relative paths remain a separately styled target value rather than a
body-font fragment glued to the project name. Filter dots derive only from
Fleet connection evidence; missing evidence is muted, with no invented
connecting state (the current Fleet does not report one).

## Immutable check checkpoint

Fast checks on this source: native fmt, size (585 files), boundaries (four
crates/eight external owners), tokens and diff-check passed. The prior changed
View clippy passed before the last test/comment corrections; it is not claimed
for this exact source. The prior selected run was 46 passed / five failed
(reference-visible expectations incorrectly omitted Fluent isolates), retained
as a red run; its expectations now check both exact catalog bytes and visible
words. A later clippy found two doc-markdown issues; both are corrected.
The final local retry was still waiting before Cargo and was cancelled by its
verified owned PIDs under the coordinator's remote-migration instruction.
Neither its selected tests nor clippy ran. The immutable commit is the input
for the remote changed-View checks; their actual result is owed below. No whole
native or visible-feature result is asserted by this checkpoint.
