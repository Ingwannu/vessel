# Vessel tools in Machine settings

M12 F-L21. The machine card follows Paseo 0.15.4
`packages/app/src/screens/settings/paseo-tools-card.tsx:9–53` and the host
Orchestration group in `packages/app/src/screens/settings/host-page.tsx`.
The switch names Vessel and shows the selected machine's confirmed value.

## Decision log

A host needs to choose whether its new agent launches receive the internal
`vessel mcp` server. The node's saved settings writer already serializes
durable choices, and the profile launcher already assembles fleet servers
independently. Add a typed field to those existing paths. Toggling the setting
through a fleet server map would entangle this choice with independently
configured servers and their secret resolution.

`node.settings` includes `vessel_tools` when the peer reaches
`VESSEL_TOOLS_PROTOCOL`. The Admin `node.set_settings` patch changes that
field alone, and both replies project it away for older peers. The dedicated
protocol fact stays separate from `MCP_PARITY`, which also gates the Skills
methods. Integration assigns the version.

Paseo's `packages/server/src/server/config.ts:552–555` defaults new hosts to
Off. Once the feature is assigned, a fresh Vessel event store records Off
before the store is opened, under the node lock. This includes nodes prepared
by `vessel init`, which creates an identity before the first start. An existing
event store with no choice keeps the previous On behavior. Existing explicit
choices and the settings file's preservation and private-write rules apply.

The node samples the choice for a new session, Start again, Reload agent or
profile move. A live process retains its setup. Parking and waking within
that same run reuse its captured setup. The internal credential and the
provider's fleet MCP servers have separate paths. For a replacement launch
the current credential remains accepted during preparation. The node's existing
session task owner settles the new choice with the actual actor result even
if the RPC caller goes away; a failed replacement retains the old launch's
credential. A successful move to Off closes its old tool connections.

## Browser and verification

Machines shows Orchestration for a connected host that reports the field.
The control waits for the write response before changing position and reports
a refusal in the card. Settings are read and saved on that machine.

The fixtures exercise explicit Off with fleet servers still present, credential
transitions in either direction, rollback, the recorded fresh default, and the
old-peer projection. The negotiated toggle can be exercised with a numbered
protocol on a private fake-provider fixture before integration assigns it in
the node line.
