# F5: Settings pages

## Specification and ownership

Follow the complete Paseo Settings survey (area 6, part D), in the two
groups and source page order. Reuse the existing Settings frame, typed
`Section` table, kit section/card/row/select/dialog controls and existing
preferences store. Every shown row must change the behavior it names.

| Surface | Owner/integration contract |
|---|---|
| Settings frame and remaining pages | E2; no duplicate frame or store |
| Appearance: themes, fonts, sizes, width/reset, syntax and preview | K; canonical Theme list/names and sole preference-to-palette adapter |
| English/Korean words | L; FTL and typed generated getters, call-expression seams agreed before edits |
| Usage | W5; one page registered through the frame |
| Diagnostics and onboarding | W20 app + N1 node; reuse existing local report until a real node call exists |
| Pairing/devices | G4; admitted connection identity is also reused by other host pages |
| Chat outline | W1 A3 minimal typed preference/change and switch; existing store, no duplicate rail setting |
| Shortcuts/command center/sidebar behavior | their F4 owners; Settings hosts their pages |
| Node update | nodectl ADR and real call first; E2 draws the Settings consumer only afterwards |
| Plugins | separate F6 owner and ADR; E2 integrates its reviewed page contribution |

## Source page order and existing differences

App: General (Language and setup), Appearance, Sidebar, Chat, Terminal,
Browser (desktop), Editor, Files, Shortcuts, Integrations, Notifications,
Permissions, Diagnostics, About. Machine: Overview, Projects, Connections,
Pair device, Agents, Metadata, Voice, Workspaces, Providers, Usage,
Terminals, Plugins. Vessel-specific Models/Sync keep their explicitly
documented status; they do not replace a Paseo page.

Current General has language and send controls plus the explicitly
Vessel-specific persistence row. Setup is W20's registration. The frozen source0a0a5a87 contains K's eight System-first Theme::ALL
choices. Its newer font/numeric/tile consumers are not assumed merged;
this tree still uses the inherited closed size/width selects. Numeric width/reset must
replace presets only through the existing validated preference model;
loading a stored preset must not rewrite it. Chat currently has display
only; W1 owns the outline addition and L its words. Connections currently
shows safe read-only routes, without latency or removal. Hidden pages
are not evidence that their behavior exists.

## Architecture decision

[Decision Log]
- Purpose: one coherent settings surface despite several feature owners.
- Constraints: source page order, no inert settings and no screen-owned
  renderer; each behavior has an existing feature/store owner.
- Alternatives: independently rebuilding specialist pages or adding
  placeholder controls for unavailable calls.
- Choice: E2 owns composition and remaining Settings rules, consumes the
  feature owners' reviewed typed APIs and registers their pages with
  minimal seams. Pure preference/view code owns parsing and transitions;
  link owns node requests; app wires the existing kit.
- Why: settings must describe actual effective behavior, and duplicate
  stores/collectors would diverge from the feature they configure.
- Costs: some pages await their producer. Their missing states remain
  recorded, rather than drawn as a successful no-op.

L's accepted Diagnostics/Chat text package changes `SettingsText::section`
to take the translator; it is present through the validated lead base.
Preserve that minimal caller seam. E2 does not parallel-migrate those text
tables. N1 currently has bounded diagnostic values but no public request,
client API or assigned protocol; W20 must not receive an invented report
producer from E2.

## Package process

After the current rename/field work: merge the announced package base,
audit every page against its actual source component, establish the
typed skeleton and fixtures, then fill the working flows in source order.
Record each deliberate platform or node gap with its owner and reason.
Update only E2-owned tracker rows when their state changes; specialist
passes do not imply a whole-page pass.

One complete gate/review/capture package under A14, narrow changed-crate
checks per commit. Capture changed states at 1280×900/390×844, 800×600
where needed, both languages/themes, empty/error/long values. Document
source, runtime, platform and review limits separately. Current state is
audit/preparation, not a new Settings parity pass.


## Effective frame and control-density boundary

Consume S2's checked shared SidebarLayout/Sharing decision after its
announced base handoff, with no new width constant or PHONE change.
Sheet, Hidden and TooNarrow require a typed stacked Settings form with
list-to-section navigation/back; Docked+Fits permits navigation and body
beside each other. Derive this from the actual root adapter decision,
not an independently recomputed width formula or a second cached policy.

Keep that effective frame separate from the existing controls' phone/
touch sizing: an800-wide window can require stacked navigation without
changing every input/button to touch geometry. Reuse the existing page
and shared header renderer. Remove the root's temporary Settings PHONE
and auto-hide exceptions in the same integration as this consumer, so
navigation is neither missing nor duplicated. This is planning only:
no frame/policy source has been changed while the input gate is frozen.

## Future canonical preference refusal

W19/S2's future bounded dismissal-set union can exceed its canonical cap
even when each input is independently valid. They own a new typed
Refused::SidebarCalloutDismissals only after the checked producer; it must
refuse before projection/store/admission changes and never masquerade as
Actions/Machines or silently clip. L reserves its single exhaustive
prepared/translated refusal arm and uses the owner's canonical limit.
E2 does not parallel-edit the prepared field, Err branch, render or
language refresh. The canonical writer/hook is still unverified, and no
new variant/row/store/watcher is introduced by this reservation.
