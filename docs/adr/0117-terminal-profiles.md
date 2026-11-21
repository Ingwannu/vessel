# ADR 0117: terminal profiles

Status: accepted by the coordinator on 2026-11-21 with four rulings:
- The integrator assigns the protocol number at merge.
- Icons follow Paseo.
- The native menu groups are part of the native packet.
- The node resolves profiles by id.

Implementer T2. Protocol: `TERMINAL_PROFILES_PROTOCOL`, a placeholder (`None`) until the integrator assigns it.
Rows: catalogue G12, G58; TRACKER 490, 275, 336 (the profile part).
This ADR takes terminal profiles out of ADR 0091. That ADR's value types (`vessel_proto::terminal_profiles`) were
ported earlier and nothing used them.

## Context

Paseo 0.15.4 paths are relative to `packages/`.

**The list.** Paseo keeps a host-wide list of terminal profiles in `daemon.terminalProfiles`.
- A profile is `{id, name, command, args?, icon?}` (`protocol/src/terminal-profile.ts` 3-13).
- An absent list means the four shipped profiles: Claude Code, Codex, OpenCode, Pi
  (`protocol/src/terminal-profiles.ts` 19-30).
- An empty list means the person removed them all (`server/src/server/config.ts` 536-543).
- A settings change replaces the whole list (`app/src/screens/settings/host-page.tsx` 1438-1443).

**The prompt placeholder.**
- `{{{prompt}}}` takes a new workspace's first message. With no message, an argument that is only the placeholder
  is left out (`terminal-profiles.ts` 47-106).
- A profile that runs a shipped agent's program without the placeholder adopts it on read. The rule is keyed on
  the command's base name. A positional argument blocks adoption, after skipping the values of options known to
  take one (136-324).
- Nothing adopted is written back. Settings edits the raw list.

**Launch.**
- The client turns the profile into `command` and `args` on `create_terminal_request`
  (`app/src/screens/workspace/terminals/use-workspace-terminals.ts` 136-144).
- The daemon checks that the program exists ("`<cmd>`: command not found"), then runs it literally, without a
  shell (`server/src/terminal/terminal.ts` 61-69, 706-826).
- The tab takes the profile's name.

**Clients.**
- New tab lists "Terminal profiles", with an "Edit profiles" accessory, after the built-ins
  (`app/src/workspace-tabs/launcher/index.tsx` 222-252).
- Settings › host › Terminals lists, orders, edits, adds and removes profiles
  (`host-page.tsx` 1311-1665; `terminal-profile-edit-modal.tsx`).
- A row's mark is `icon ?? guess(command base name)` (`terminal-profiles.ts` 108-132).

## Decision

1. **Settings.**
   - `NodeSettings.terminal_profiles` and `SetNodeSettings.terminal_profiles` are both `Option<Profiles>`.
   - `None` means never set, and clients show `Profiles::shipped()`. `Some([])` means removed.
   - A set replaces the whole list. The node keeps it in `settings.json` as sent. Adoption is applied at launch
     only.
2. **Launch by id.**
   - `CreateTerminal.profile: Option<ProfileLaunch { id, prompt? }>`.
   - The node finds `id` in its list in force (kept, else shipped), adopts the placeholder, and substitutes the
     prompt, or nothing.
   - It then finds the program:
     - a program with a folder in it is resolved from the terminal's folder;
     - a bare name is looked up on the node's `PATH`, which its terminals inherit.
   - It starts the program exactly as it starts a shell: the same folder, scrubbed environment, activity
     capability and pseudo-terminal.
   - The terminal is `TerminalKind::Shell { program }`, so every client, older ones included, types into it.
   - The title defaults to the profile's name.
   - Refusals:
     - an unknown id gives `not_found` "no terminal profile `<id>`";
     - a missing program gives `spawn_failed` "`<program>`: command not found", Paseo's words;
     - an over-long substitution gives `invalid_request`.
3. **Adoption and marks** live in `vessel_proto::terminal_profiles`, so every Rust client and the node agree.
   - Adoption is Paseo's rule, with its table of options that take a value (claude, codex, opencode, pi).
   - The mark is `ProfileIcon`:
     - `Known(name)` for a catalogue name, set or guessed;
     - `Terminal` when there is no name;
     - `Unknown` for a set name outside the catalogue, drawn as the generic mark (Paseo's Bot).
   - A client without a known name's artwork draws the terminal glyph.
   - Harness marks keep their exact-identity rule.
4. **One protocol constant.** `TERMINAL_PROFILES_PROTOCOL` covers the settings field, its set, and
   `CreateTerminal.profile` (`Call::since`).
   - While it is `None`, the server refuses both fields and no peer is shown the list.
   - Positive tests run below the server's gate.
5. **CLI.** `vessel shell --profile ID` opens a profile's terminal.

## Stated differences

- **Resolved by id, not sent as argv.** The person sees the same thing. The reasons:
  - one resolver, in Rust, serves the native app, the page and the CLI;
  - a profile removed meanwhile is refused, not run from a client's stale copy;
  - the node runs only templates its admin saved.

  An operator who can open a shell gains nothing either way.
- **Limits** (`MAX_PROFILES` 64, 256 arguments, 64 KiB per launch, field sizes). Paseo has none.
- **Windows.**
  - The node skips the lookup there, as Paseo does.
  - It does not route `.cmd` and `.bat` through `cmd.exe` (Paseo `terminal.ts` 309-415). A Windows profile names
    an `.exe` or a full path.
- **Not here:**
  - the "Terminal agents" hooks card (ADR 0089, P6);
  - the New-workspace launch picker (TRACKER 493 and 14), for which `ProfileLaunch.prompt` is the wire.

## Consequences

- An older client keeps working: the new fields are absent from its replies, and it never sends them.
- An older node refuses neither field, because the client checks `Call::since` first and refuses to send.
- The native app's and the page's `NodeSettings` and `CreateTerminal` literals gain the new fields when they merge
  this.
- Adding a CLI that takes a value, or a provider mark, means one more line in the proto tables.
