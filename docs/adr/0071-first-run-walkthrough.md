# ADR 0071: The first-run walkthrough is a model in `view`

Status: proposed, 2026-11-14, until review passes it. The log is not
contiguous; 0052 is the previous ADR in this tree. No wire call is
added, so no protocol number is taken and none is reserved for later.
No `node.settings` key is added: completion is not a node setting.
No crate edge is added, so `docs/ARCHITECTURE.md` is unchanged.
The screen and the call that mounts it are later slices of
`docs/plans/app-onboarding.md`; this decision is the contract they
implement.

## Purpose and intent

Paseo walks a new person through setup once, then remembers that on
that client. Settings › General can show it again. Vessel's app does
not. On the first fleet read with no machine it opens the pairing
route and nothing else (`spikes/native/app/src/shell.rs`). `vessel
init` prepares a node and is safe to repeat; it is not this
walkthrough.

The walkthrough should be Paseo's: a theme, a machine connected by
pairing, the agents the node has already checked, a first project,
the tools switch, and pairing a phone. The rules of when it opens,
when it is skipped, and when it is shown again are Paseo's. A step
is omitted only for a reason that is still true after the sibling
ADRs merge.

## Existing implementation and constraints

Paseo 0.15.4, read for this decision (package paths; line ranges are
that tree):

- The step machine is `packages/app/src/onboarding/internal/model.ts`
  (steps at 1–8, the record at 10–13, the host facts at 15–26, the
  state at 28–42, the operations at 44–68, readiness at 70–72, the
  one decision at 103–116, an unreadable store at 124–128, finish at
  135–139, reopen at 152–154, advance at 155–159). The tests that pin
  it are `internal/model.test.ts`.
- The record is one JSON object `{ completedAt }` under the client key
  `@paseo:onboarding` (`internal/completion-storage.ts` 6–22). Missing
  is not completed. Invalid is deleted and read as missing
  (`packages/app/src/storage/validated-storage.ts` 29–48). A store
  that throws is treated as completed, so a person who already uses
  the app is not kept inside setup (`model.ts` 124–128).
- The host is the desktop's own daemon, otherwise the first saved
  host (`internal/use-setup-host.ts` 9–14). Setup opens only when
  that host is connected, its workspaces have been read, the record
  is not completed, and the host has no project. The decision is once
  per launch: a project added during setup does not close it, and a
  project removed afterwards does not open it (`model.ts` 103–116;
  `onboarding-host.tsx` 16–42). A native phone never opens it
  (`onboarding-host.tsx` 16–18). While the decision is still open and
  the host is not offline, startup stays on its loading screen
  (`use-setup-decision-pending.ts` 8–26).
- Steps, in order, each one's own screen under `internal/steps/`:
  1. Appearance (`appearance-step.tsx`): the shared theme picker.
     Picking a tile applies it at once (`appearance/theme-picker.tsx`
     26–45, the comment at 26–29 and `updateSettings` at 40–44).
     Continue only moves on, because the modal's primary action
     special-cases only projects and orchestration
     (`onboarding-modal.tsx` 160–199) and this step is the plain
     advance. Built-in choices that were read lead with the system
     (`auto`, `theme-picker.tsx` 18–22), then light and dark
     (`styles/theme.ts` 783–798); zinc begins at 799. The rest of
     `THEME_OPTIONS` was not read. Contributed themes are rendered
     after the built-ins (`theme-picker.tsx` 57–63). What
     `useContributedThemes` loads was not read.
  2. Providers (`providers-step.tsx`, the whole file, 10 lines): it
     renders `ProvidersSection` for that host and nothing else.
     Continue only moves on (`onboarding-modal.tsx` 160–199, the
     fall-through). The section's own sign-in, install and removal
     were not read past its start (`providers-section.tsx` 356–419).
  3. Projects (`projects-step.tsx`): add a folder, list folders
     already added, and suggestions from agent history. A folder used
     at least three times in the last 30 days starts checked
     (`project-suggestions/internal/suggestions.ts` 5–6 and 52–56).
     The primary button adds the checked folders and moves on only
     when every one was added (`onboarding-modal.tsx` 160–186). With
     nothing checked, Continue moves on, including from zero
     projects.
  4. Orchestration (`orchestration-setup.ts`, `steps/orchestration-step.tsx`):
     three switches, all on — inject Paseo's tools into agents,
     install skills, install the CLI. Nothing is written until
     Continue, and Continue moves on only when every offered switch
     took. A switch the host cannot do is not offered.
  5. Phone, only when the client is not a compact phone
     (`model.ts` 23–26 and 86–88; `use-open-onboarding.ts` 6–14).
     What was read of `steps/phone-step.tsx`: store links, a relay
     pairing modal, and a Tailscale docs row. The file was read
     through its styles.
- Skip setup is on every step but the last and closes setup
  (`onboarding-modal.tsx` 78–84). The sheet's close does the same.
  The last step's primary is Finish: that step's own work runs, then
  setup completes. Back does not pass the first step. The rail jumps
  to any step, including one not yet reached. Each stay on a step has
  a visit number; an action that finishes after the person has left
  that stay does nothing (`model.ts` 155–159, test "ignores a late
  advance").
- Only completion is stored. The step is not. An unfinished setup
  (the record was never written) is offered again on the next launch,
  from the first step. Skip and close write the record, so they do
  not resume. Show setup again (`use-open-onboarding.ts` 6–15,
  Settings › General at `screens/settings-screen.tsx` 391–411) opens
  the first step even when the record exists or the host has
  projects, and does not clear the record until setup is closed again.
  The button is disabled when there is no host. The row is absent on
  a native phone. The words are `settings.general.setup` in
  `i18n/resources/en.ts` (2506–2513): label "First-run setup",
  description "Walk through appearance, providers, projects and phone
  pairing again", action "Show setup again".
- `paseo onboard` (`packages/cli/src/commands/onboard.ts`) is a
  different door: it writes daemon choices, starts the local daemon,
  asks about voice (default off when nobody is there to answer), and
  prints a pairing link.

What Vessel already has, and must be reused rather than copied:

- Theme is `Theme` (`view` `prefs/appearance.rs`): system, light, dark,
  system first, applied as `PreferenceChange::Theme` and kept in the
  app's prefs. Settings › Appearance already draws the three tiles.
- Pairing is `Command::Pair` and `view::pairing`. The shell sends a
  person with no machine straight to that route.
- Agents are the node's readiness (`view::agent_status::readiness`,
  ADR 0032): found or not, chat support, and `SignedIn`. The link's
  `Ask::List` reads the cache and `Ask::Check` runs a check the person
  asked for. Sign-in help is the node's command, copied, never a
  command the app invents (`app` `areas/settings/agents/detail/sign_in.rs`).
- A project is `ProjectAsk::Add` and `ProjectAsk::List` on the machine
  (`link` `workspace`). Not read: a node API that lists folders from
  agent history. A search of `spikes/native` found no such list (the
  only "suggestions" there are the composer's placeholders). The
  step does not invent one.
- The app's memory between launches is one file, `app-prefs.json`,
  replaced whole, left untouched when it cannot be read (`link`
  `prefs.rs`). The settings document inside it is what will follow
  the person to a node later (ADR 0036). `view` does no I/O
  (`spikes/native/README.md`).

## Alternatives considered

- **Open pairing and stop**, which is the app today. It never offers
  a theme, the agents, or a project, and it cannot be shown again
  from Settings.
- **Persist the step as well as completion**, so a relaunch continues
  mid-walkthrough. Paseo does not. A second fact would be a second
  place for "are we in setup".
- **Put completion in the settings document** that ADR 0036 will move
  to a node. Paseo remembers setup on the client, not as a preference
  that follows the person. A finished setup on one machine would then
  hide it on another.
- **A new file beside the prefs.** Same client-local result, plus a
  second reader and a second crash rule. The prefs file is already
  that store.
- **Start a local daemon from the app**, as `paseo onboard` does.
  Vessel's node is a process the person installs; the app is a client
  of nodes it pairs with (ADR 0035). A daemon the app spawned would be
  a second kind of node.
- **Install skills and the CLI from the orchestration step, default
  on.** The tools switch is kept, default on, because that is not an
  install: it writes one settings field. Skills and the CLI are
  installs. `vessel init` will not install on `--yes` alone
  (`crates/vessel-cli/src/init/mod.rs` 16–21, read). The Windows
  behaviour of `--service` was not read, and this walkthrough does
  not call it.
- **Wait forever for a host before showing setup**, as Paseo waits for
  its daemon. With no daemon, a new person would see the empty app
  and never the walkthrough.

## The choice

The walkthrough is a pure model, `view`'s `onboarding`, with no I/O
and no UI types. The app feeds it facts it has already parsed. The
model returns the phase and, when setup finishes, the record to
write. `link` writes that record. The screens in
`app/src/areas/onboarding/` only draw.

### The steps

These steps, in this order. `Phone` is dropped only for
`Client::Phone`, fixed when the phase leaves `Deciding`.

| Step | What the person does | What is written, and when |
| --- | --- | --- |
| `Theme` | Picks from the same list Settings › Appearance draws, in that order | `PreferenceChange::Theme` at the pick. Continue does not write it again |
| `Machine` | Pairs a node with the existing pairing flow (`Command::Pair`, `view::pairing`) | The node book, when pairing succeeds. Continue does not require a machine |
| `Agents` | Sees each agent as the node's last check left it, and can ask for `Ask::Check` or copy the node's sign-in command | Only what those existing calls already write. Continue does not require a sign-in |
| `Project` | Adds a folder with `ProjectAsk::Add`, and sees folders `ProjectAsk::List` already returned | The project, when the add succeeds. Continue with no add in flight moves on, even at zero projects. An add still in flight moves on only when it succeeds, and only if the person is still on that visit |
| `Orchestration` | The tools switch, on to start, as Paseo's `ALL_ON` | On Continue, `node.set_settings` for the one key ADR 0065 adds (added at merge). Not written before Continue. A failed write does not move on. Skills and the CLI are not switches |
| `Phone` | The device invitation Settings › Devices already makes (`view::devices`, ADR 0010) | The node's pairing link, only while that screen already shows one. Continue does not require a phone to be paired |

`Phone` is in the list on the desktop app at every window width. It is
omitted when this app is itself the phone, because the phone is the
thing being paired (`steps/phone-step.tsx` is that pairing, read
through its styles; Paseo also drops the step when the client is a
phone, `model.ts` 23–26). `breakpoint::phone` is a width
(`kit/src/breakpoint.rs` 20–22) and is not this test. A search of
`spikes/native` found no `target_os = "android"` cfg; the caller passes
`Client::Phone` or `Client::Desktop`. Not read: the phone target's
entry point.

The tools switch is offered only when the machine's settings carry
ADR 0065's key and this device holds `Scope::Admin` (`SetNodeSettings`
needs admin, `crates/vessel-proto/src/settings.rs` 33–36). Otherwise
the step is still shown and Continue only moves on. Paseo always
offers the tools switch (`orchestration-setup.ts`, `offered.tools` is
`true`; read with `ALL_ON`). Offering it only when the key and admin
are there is the same rule as Paseo's "a switch the host cannot do is
not offered", applied to a key this tree does not have yet. The
field's name was not read: ADR 0065 is not in this tree. This step
writes that field and no other key. How the shell learns the device's
scopes was not re-read; the caller passes admin or not.

The model does not name a machine. Which paired machine the later
steps show is the shell's existing selection. Not read: the rule that
picks one when several are paired. The screens must not invent an
order. With none paired, those steps say so and Continue still moves,
except the tools write, which has no machine to take it and so is not
offered.

### The phase

`Deciding`, `Hidden`, or `Open`.

`Open` carries the step and a `Visit` (a newtype over `u64`). A new
model starts at 1. Reopen and each move take the next number; it does
not restart, which is Paseo's `visits += 1` (`model.ts` 82–90) and is
what makes a visit from before a reopen fail the check. Paseo's number
has no saturation rule (not read: any cap in that file; the counter is
a plain `let visits = 0`). Here it grows by `checked_add`. If it cannot
grow, the move does not happen, so a wrapped number cannot equal a
stale visit. That is a difference, and the reason is checked
arithmetic. Back on the first step, and a jump to the step already
showing, do not take a new visit. `next` takes the visit the caller
saw: a mismatch, or a phase that is not open, does nothing. From the
last step in the list, a matching `next` finishes setup even when the
counter cannot grow, because finishing is not a move to another step.

`complete` from any phase hides setup and asks for the record to be
written. That is Skip, the sheet's close, and Finish. The Skip control
is not drawn on the last step; Finish is `next` from there. Hidden means
this launch does not open it again unless `reopen` is called.

`reopen` opens `Theme` on a new visit from any phase, including
`Deciding` and `Hidden`. It does not clear a record already stored.
Notes that arrive after the phase has left `Deciding` do nothing, so a
project that appears during setup cannot close it.

### When a launch opens it

The model starts `Deciding`, with the completion, the client and the
machines not yet noted. Any note may arrive first.

- Completed record, or the prefs file could not be read at all: hide.
  Do not wait for the machines or the client. The unreadable file is
  not written, the rule `link`'s prefs already have; the model only
  refuses to open.
- Pending, and the client not yet noted: stay `Deciding`, so a phone
  build cannot open with the phone step and then drop it.
- Pending, the client noted, and no paired machine: open. This is the
  difference from Paseo, and the reason is below.
- Pending, and every paired machine's project list is still unknown:
  stay `Deciding`.
- Pending, and any paired machine has a project: hide, as already set
  up.
- Pending, and every known list is empty: open.

A hide because of a project is sticky for the launch, as Paseo's is.
Startup may keep its loading screen only while the phase is still
`Deciding` and a paired machine's projects are loading. No machine,
and a file already known completed or unreadable, are decided at once.

### Where "completed" is stored

On the app's `app-prefs.json`, as a field of that client-local file,
not a field of the settings document:

```json
"setup": { "completed_at": "2026-11-14T00:00:00Z" }
```

`completed_at` is an RFC 3339 timestamp, parsed once into a newtype
(`chrono` is already `view`'s). The clock is read by the caller that
asks to finish, not by the model. The field name is snake case because
that file already is.

Missing field: pending. A field this build cannot read: pending, and
omitted on the next save of the same file (Paseo deletes an invalid
value; this file is replaced whole, so there is no separate delete).
The whole file unreadable: do not open, and do not save, as above.
A missing file is a first launch, not an unreadable one: the prefs
loader already treats those differently.

Finishing writes the field through the prefs' existing save. If that
save fails, the phase stays hidden for the launch and the next launch
reads whatever is on disk.

### What is not built

- No listener list. Paseo's `subscribe` wakes React. Here the caller
  has just called, and the shell notifies its window.
- No new wire call, and `vessel init` is unchanged.
- The shell and Settings are not edited by this work. The plan names
  the lines their owners mount.

### What it reaches

Nothing of its own. It does not open a browser, listen for HTTP, spawn
a helper, or run a script at startup. It does not use signals, systemd
or launchd, so there is no Windows path to add and no Windows refusal
to write. The later screens call pairing, agent checks, project add,
`node.set_settings` and the device invitation as those calls already
exist. A device invitation is a credential (`view` `devices.rs` 1–12,
read): the walkthrough uses that screen's rule and does not log the
link, put it in a test failure, or leave it in a capture's name.
Whose credential the other calls use was not re-read (the app README
says the app uses the node book and device credentials the CLI uses).
`node.set_settings` is refused without `Scope::Admin`. The model's
tests are values in the test process. They do not listen, and a
release build has no fake that answers.

## Why this and not the others

Paseo's machine already has the cases that are easy to get wrong: one
decision per launch, a late add that must not move a step the person
left, a record that is the only durable fact, and a reopen that does
not forget a previous finish. Copying that machine into `view` lets
the tests be those cases. Putting it in the shell would mix it with
GPUI, which the crate split forbids, and the tests could not run
without a window.

Pairing stands where Paseo's daemon stood. The rest of Paseo's order
is kept: the theme needs no machine and comes first; agents and a
project need one and come after the connect step. Continue never
blocks on a theme, a machine, or a sign-in, because Paseo's Continue
does not either. An add that the person already started does block,
because Paseo's checked folders do.

## Advantages, costs and effects

- One model decides for every surface that draws the app. The page
  does not gain this walkthrough here.
- A person with a project is not shown it, including someone whose
  prefs file has no `setup` field because they started before this
  decision. A person with no machine is shown it, which today's shell
  only half did by opening pairing.
- A broken prefs file does not trap them in setup, and is not
  overwritten.
- The step they were on is forgotten when the app quits. That is
  Paseo's cost, kept on purpose.
- Theme, pairing, agent checks, project add and the device invitation
  stay the calls they already are. The theme list is Settings ›
  Appearance's, so ADR 0070's registry (added at merge) is drawn here
  without a second list. This tree's Appearance draws `Theme::ALL`
  (system, light, dark), read in `prefs/appearance.rs`.
- Not offered, with reasons in the plan: the skills switch, the CLI
  switch, store links, agent-history suggestions, and `paseo onboard`.
  The tools switch and the phone step are offered.
