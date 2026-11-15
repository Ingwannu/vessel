# The first-run walkthrough

> Historical proposal retained for reference. The current W15 implementation
> follows the desktop amendment in ADR 0071 and
> [wp-desktop.md](wp-desktop.md). The Machine step, no-host opening, six-step
> model, `setup` record name and phone-width rules below are superseded. They
> must not be used as the current implementation contract.

Current source uses five steps (Appearance, Providers, Projects, Orchestration,
Phone), a fixed Wide/Compact layout per opening, client-local
`Prefs.onboarding`, canonical ConnectionContext plus monotonic Visit, and the
existing settings owners. Compact omits Phone; native mobile suppresses this
walkthrough. Missing/offline/unhydrated host facts stay Deciding; unreadable
preferences hide automatic setup. The managed local target is preferred while
its controller/admission is ready; pairing remains a separate no-host route.

Completion/visit models have scoped tests. Canonical project inventory and
checked-suggestion acknowledgements, the real queued DeliveryGuard consumer,
MCP/skills owner integration and the combined app gate/captures remain pending.
A raw ProjectsView::Ready cache is not authoritative proof across physical
retirement; the W11 owner must supply its guarded query. Existing/Empty and
step-advance behavior are not approved from that legacy cache. CLI effects use
the desktop-shell owner API and must settle before advancing; unavailable effects
are not silently claimed as installed.


ADR 0071 is the contract. Its status is proposed until review passes
it. This file is the order of work and the Paseo behaviour it was
taken from. No protocol number is reserved. No `node.settings` key
is added.

Paseo 0.15.4 is the reference. Paths below are that repository's
packages. Vessel paths are this repository.

## Existing assets, and the choice

- **Theme.** `vessel_app_view::prefs::Theme` and
  `PreferenceChange::Theme`. Settings › Appearance already draws
  `Theme::ALL` (system, light, dark) in
  `spikes/native/app/src/areas/settings/appearance.rs`. The step writes
  that change. It does not grow a second theme store.
  Paseo's picker also offers zinc, midnight, further palettes
  (`packages/app/src/styles/theme.ts` from 783) and contributed
  themes (`appearance/theme-picker.tsx` 31–64). Those themes are not
  in the kit. Decision: do not add them.
- **A machine.** `link::Command::Pair` and `view::pairing`. The shell
  already opens `Route::Pair` when the first fleet has no machine
  (`spikes/native/app/src/shell.rs`, the empty-fleet arm). The
  connect step is that flow, drawn inside the walkthrough. It is not
  a second decoder. If composing the existing pair screen needs a
  change outside `app/src/areas/onboarding/`, the line is reported
  and not edited here.
- **Agents.** `view::agent_status::readiness` and the link's
  `Ask::List` / `Ask::Check`. Sign-in stays the node's command, copied
  (`areas/settings/agents/detail/sign_in.rs`). The step does not invent
  a login. Paseo's providers section also enables, removes and
  installs providers (`screens/settings/providers-section.tsx`). The
  walkthrough shows the checks Vessel's node already returns. It does
  not grow a provider catalogue.
- **A project.** `ProjectAsk::List` and `ProjectAsk::Add`. Paseo's
  suggestions come from each agent's history, pre-checked at three
  sessions in 30 days (`project-suggestions/internal/suggestions.ts`
  5–6, 52–56). Not read: a node API of that history. A search of
  `spikes/native` found no project-suggestion list. Decision: no
  suggestion list, because inventing one would not be Paseo's data.
  Add is the existing add. Continue with nothing in flight moves on,
  as Paseo's Continue does when nothing is checked.
- **The record.** The app's `app-prefs.json` (`link` `prefs.rs`), a
  new `setup` field, not a field of `Preferences`. `view` still does
  no I/O. No new crate, no new dependency (`chrono` is already
  `view`'s), no new wire call.
- **External alternatives.** A wizard crate, or Paseo's React screens
  copied into GPUI. Refused: the rules are a closed enum, and Paseo's
  sources are not ours to copy.

Decision: **new** model (`view` `onboarding`) and **new** screens
(`app/src/areas/onboarding/`). **Modify** the prefs file's schema by
one optional field, in the slice that loads it. **Do not modify** the
shell or Settings; their owners mount the lines below.

## What Paseo does, kept

The detail and the line ranges are ADR 0071. The tests in slice 2
match `internal/model.test.ts` case for case, under the names here.

| Paseo case (`model.test.ts`) | Here |
| --- | --- |
| Opens on appearance for a connected host with no projects | Pending, and no machine, or machines whose project lists are empty: open on `Theme` |
| Waits until connected and hydrated | Pending and project lists unknown: stay `Deciding`. No machine does not wait (stated difference) |
| Stays hidden if the host had projects, even after they go | `Hidden::AlreadySetUp` is sticky |
| Stays hidden once completed | A `completed_at` hides, and a later empty fleet does not open |
| Decides only after the record has loaded | Neither note is assumed; completed hides without the machines |
| Stays open if the first project appears during setup | Notes after leaving `Deciding` do nothing |
| Omits the phone step on a phone | `Client::Phone` omits `Phone`. A narrow window does not: `breakpoint::phone` is a width |
| Moves inside the list; back stops at the first | `back` and `go_to` |
| A late advance from a visit already left does nothing | `next` checks `Visit` |
| The last step finishes, unless that visit was left | `next` from `Phone`, or from `Orchestration` when the client is a phone |
| Closing writes the record, so the next launch stays hidden | `complete` |
| Reopen starts at the first step for someone who finished | `reopen` |
| Subscribers see each change | Not ported: the caller notifies. The assertions are on the phase |

Also kept, from the modal rather than the model: Skip is drawn on
every step but the last; close finishes; Finish on the last step runs
that step's add if one is in flight; an add uses the visit it started
with.

## Stated differences

1. **Pairing instead of a local daemon.** Paseo's host is the desktop
   daemon, else the first saved host (`use-setup-host.ts` 9–14), and
   the walkthrough does not start it. `paseo onboard` does, and also
   asks about voice. Vessel's app pairs a node. With no machine the
   walkthrough opens, so the connect step can run. `vessel init` is
   not changed and is not called from the walkthrough.
2. **Skills and the CLI are not switches.** Paseo's three switches
   default on (`orchestration-setup.ts` `ALL_ON`). The tools switch
   is kept, default on, and writes ADR 0065's key through
   `node.set_settings` when the machine carries that key and this
   device holds admin; otherwise it is not offered and the step still
   shows. Skills and the CLI stay out: they are installs, and
   `vessel init` does not install on `--yes` alone. ADR 0065 is not
   in this tree (added at merge); the field name was not read.
3. **No store links on the phone step.** The step is the device
   invitation Settings › Devices already makes (`view::devices`,
   `vessel node pair` in `crates/vessel-cli/src/devices.rs`). A
   search of the native tree and the docs found no store URL, so
   none is invented. The step is omitted when this app is the phone,
   not when the window is narrow. Paseo also never opens setup on a
   native phone (`onboarding-host.tsx` 16–18). Here the other steps
   still open on a phone, because this app is that client (ADR 0036).
4. **The theme list is Settings › Appearance's**, not a list kept
   here. Today that is `Theme::ALL`. ADR 0070 (added at merge) is
   the registry; this step draws whatever Appearance draws after
   that merge. Not a difference of three themes.
5. **No history suggestions.** Add a folder, list what the machine has.
6. **The record lives in `app-prefs.json`**, field `setup.completed_at`,
   snake case, RFC 3339. Paseo's key is `@paseo:onboarding` with
   `completedAt`. An invalid field is omitted on the next save of that
   file rather than deleted by its own write, because the file is
   replaced whole. An unreadable file hides setup and is not saved.
7. **No `subscribe`.** Same transitions; the shell notifies.
8. **Show setup again** uses the same button words. The description
   names these steps, including phone pairing. The row is not removed
   on a narrow window. The button is always enabled: reopen does not
   need a host, because the connect step is how a host appears. Paseo
   disables it with no host (`use-open-onboarding.ts` 6–15).

## Words

English is the source, Korean in the same catalogs, ADR 0039. Taken
from `packages/app/src/i18n/resources/en.ts` 1671–1734 and 2506–2513
where the step exists:

- Rail title: the product name. Step counter "Step {current} of
  {total}". Skip setup. Continue. Finish. Back.
- Theme: label "Appearance", title "Welcome to Vessel", description
  "Let's start by picking a look".
- Agents: label "Providers", title "Your agents", description "The
  machine runs the agent programs it already has". Paseo's sentence
  says "this computer"; the machine may be another one.
- Project: label "Projects", title "Add your projects", description
  "Where do you want your agents to work on", button "Add project",
  badge "Added".
- Orchestration: label "Orchestration", title "Let agents use Vessel",
  description "Let your agents create workspaces and talk to other
  agents". Paseo's title names Paseo. Only the tools switch is drawn.
- Phone: label "Mobile", title "Take your agents with you". Paseo's
  description names a published app and store buttons; those URLs are
  not in this tree, so the description is "Pair a phone with this
  machine."
- Machine, which Paseo does not have: label "Machine", title "Connect
  a machine", description "Pair a node you already run. This app does
  not start one".
- Settings row: "First-run setup", "Walk through appearance, a
  machine, your agents, a project and phone pairing again", "Show
  setup again".

## Layout

Paseo's modal (`onboarding-modal.tsx`): a desktop card, rail at the
full height with the mark and the steps, the step's title and
description above its content, actions at the foot; a compact stack
of title, "Step N of M", content, actions. Controls come from the kit.
Spacing, type, colour and radii come from the kit's tokens. A step
change uses `vessel_app_kit::motion` and is instant when motion is
held. Nothing animates continuously. Captures in both themes, English
and Korean, at 1280, 390, 800×600 and 1024×700, glyphs at scale 1,
1.25, 1.5 and 2, on a fake fleet. Not in the docs slice.

## Lines other owners mount

Not applied in this branch.

Shell, the empty-fleet arm that today calls `route_to(Route::Pair)`:
ask the model with no machine, and if the phase is open, draw the
walkthrough instead of that route. Pairing stays the walkthrough's
machine step. A fleet that already has a machine only notes the
project facts; it does not open the pairing route.

Settings › General, a row in the general card, beside the page's
`ShowSetupAgainRow`: the label and description above, and a button
whose action is `onboarding.reopen()`.

## Slices

1. **ADR and this plan.** Docs only. No code, so the native README's
   layout list is not updated until the area exists.
2. **The model and its tests**, in `view`, one module, tests beside it.
   The record's constructor parses RFC 3339 and rejects anything else.
   No prefs file yet: the model takes a completion value.
3. **The screens** in `app/src/areas/onboarding/`, kit controls only,
   both catalogs. The native README's area list gains the area in that
   commit.
4. **The entry.** Load and save `setup` through the prefs file. Expose
   the calls slice 2's model already has, so the shell and Settings
   can mount the lines above. Do not edit `shell.rs` or
   `areas/settings/`.

## Resume point

- **Done:** ADR 0071 (status: proposed) and this plan, including the
  returned slice's three fixes (tools switch via ADR 0065, phone step
  as device pairing, theme list shared with Appearance). No protocol
  number. No new `node.settings` key in this tree.
- **Half-done:** nothing committed.
- **Next:** slice 2, the model, with `Client` and the six steps. The
  model does not name a machine. Do not open slice 3 before those
  tests pass.
