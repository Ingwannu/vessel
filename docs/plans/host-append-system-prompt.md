# Host system prompt supplement

M12 F-L23. The host Agents section stacks its cards. System prompt follows
the browser-tools position and precedes restart recovery. Its outline Edit
button opens an Append system prompt sheet with Reset and Save. Paseo 0.15.4
uses this order in `packages/app/src/screens/settings/host-page.tsx:281–308` and this editor
in `host-page.tsx:983–1089`.

## Decision log

- Purpose. Let a machine owner give supported agent processes a common
  system instruction while retaining provider and session context.
- Constraint. The saved text may contain private instructions. The Admin
  read routes through `node.append_system_prompt`. `node.settings` returns
  the available transport names. Both use one protocol fact with the
  `node.set_settings` patch.
- Alternatives. Returning the text with ordinary settings would expose it
  to Read callers. An ordinary user turn would enter the conversation and
  give the provider a different instruction priority.
- Choice. The existing settings writer keeps the exact validated text and
  publishes the committed value to the runtime. Omission keeps the saved
  value. Blank clears it. The value is trimmed when it reaches the native
  instruction channel. Every physical spawn takes the latest published
  value, so it applies on restart, reload and wake as well as on create.
- Effect. The first supported transport is Codex app-server. Its
  `developerInstructions` field carries the supplement on thread start and
  resume. An explicit blank on resume clears a prior supplement, while a
  fresh thread without a supplement follows the provider's usual defaults.
  ACP cannot carry the setting. The Claude and OpenCode direct drivers
  remain unavailable on this receiving line. Support is added when each
  driver is usable and its native delivery passes the provider fixtures.

Paseo omits `developerInstructions` on resume when its composed prompt is
empty (`codex-app-server-agent.ts:4076–4091`, `agent/system-prompt.ts:1–10`).
Vessel sends an empty value on resume to clear a host supplement that Codex
may retain with the thread. This can also clear the user's configured Codex
`developer_instructions` default for that resumed thread. On a fresh thread,
Vessel omits the field when the supplement is empty, so Codex keeps its
configured default.

The node stores the setting with its other owner-only settings. The saved
type masks Debug output and the Codex handshake masks provider refusals that
could quote the supplement. The web sheet reads text on Edit and keeps the
draft through a failed save. The row says “Adds a system prompt to Codex
agents” because the node supports the Codex app-server launch transport.
The shared Vessel dialog has a 520 px desktop width. Paseo sets 560 px
for its editor, and Vessel's shared kit offers 520 px and 640 px sizes.

The wire tests exercise the shared boundary and older replies. The settings
fixture covers Admin scope, saved value across reboot, explicit clearing and
ordinary settings projection. With the protocol assigned, it also starts a
Codex session through the node, clears the setting and reloads the same
conversation. The fake checks the native parameters on both physical
launches and echoes a synthetic rejection in a driver test to exercise
redaction.

Provider reference at Paseo 0.15.4 is
`packages/server/src/server/agent/providers/codex-app-server-agent.ts:3838–3850,5386–5401`.
