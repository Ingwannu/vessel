# F5: General words migration

General's settings behavior belongs to the settings owner. This word package
changes generated catalog calls and owned translation data, preserving input,
preference/store, visibility, ordering, controls and layout. The reference is
Paseo's General/Sending settings sections, already described by the native
General renderer. Existing Vessel wording is the migration source; no reference
translation assets are imported.

[Decision Log]
- Purpose: migrate the existing General labels and delivery explanations with
  the English-generated contract, English/Korean together.
- Existing assets: the Fluent catalog and Settings cached-choice adapter.
- Alternative: formatting a selector each frame adds allocation; retaining a
  duplicate Rust word table gives translators two sources of truth.
- Choice: whole static messages plus exhaustive TurnDelivery-to-cached-getter
  adapters, as the existing Chat display choices do.
- Effect: no dependency, model/range/store, physical-key naming or UI behavior
  change. The unused general_lead field has no caller and is removed.

## Plain-word subfeature

`general.ftl` contains 13 messages in each language: title, Sending, send
shortcut, default send, three delivery labels and three whole explanations,
Kept, Preferences, This launch only. Existing values are preserved. General's
renderer changes only those expressions. The delivery trigger/list changes
only its label expression. Section::General uses the prepared title; other
section mappings stay unchanged. The corresponding Rust fields/methods are
removed. Whole English/Korean goldens verify all messages and delivery cases.

## Remaining General scope

This is not a complete General feature. Kept-file/reason text still uses the
legacy kept_note/kept_why path pending event/language prepared-cache seams.
Path-copy notice ownership/refresh and the shared refused banner are separate.
The NoPath raw diagnostic branch has no current producer; its typed/fallback
boundary needs coordination rather than a translation of arbitrary diagnostics.
General retains its existing storage behaviors while these parts are prepared.
The settings owner permits word expressions only in Overview/Connections;
those pages have not been touched. Its terminal-settings domain is separate.

State: plain-word source prepared, narrow checks owed. Full General completion,
its package gate/review and before/after screen proof remain owed. Static quality
and component checks are reported at each committed source checkpoint; the F6
helper's earlier evidence is not used as approval for these new resources.
