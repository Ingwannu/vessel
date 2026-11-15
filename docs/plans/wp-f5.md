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
Vessel-specific persistence row. Setup is W20's registration. Appearance
currently has three themes and closed size/width selects; K's reviewed
eight-theme package is not assumed merged. Numeric width/reset must
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
to take the translator. Preserve that minimal caller seam when it arrives
through the validated lead base. E2 does not parallel-migrate those text
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
