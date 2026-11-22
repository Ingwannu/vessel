# Host system prompt supplement

M12 F-L23. The host Agents card places System prompt after the browser-tools
position and before restart recovery. Its Edit button opens an Append system
prompt sheet with Reset and Save. Paseo 0.15.4 uses this order in
`packages/app/src/screens/settings/host-page.tsx:281–308` and this editor
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

The node stores the setting with its other owner-only settings. The saved
type masks Debug output and the Codex handshake masks provider refusals that
could quote the supplement. The web sheet reads text on Edit, keeps the draft
through a failed save, and shows a capability-specific hint. Its hint names
Codex app-server and ACP because Paseo's “all agents” wording would promise
more than this node serves.

The wire tests exercise the shared boundary and older replies. The settings
fixture covers Admin scope, saved value across reboot, explicit clearing and
ordinary settings projection. With the protocol assigned, it also starts a
Codex session through the node, clears the setting and reloads the same
conversation. The fake checks the native parameters on both physical
launches and echoes a synthetic rejection in a driver test to exercise
redaction.

Provider reference at Paseo 0.15.4 is
`packages/server/src/server/agent/providers/codex-app-server-agent.ts:3838–3850,5386–5401`.
