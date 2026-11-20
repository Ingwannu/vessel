# ADR 0119 — General browser activation independent of Preview

Accepted 2026-11-20. Amends [ADR 0078](0078-browser-engine-and-native-panel.md).

## Context

The workspace browser and agent tools share a node-owned Chromium runtime. ADR
0078 defines a persistent General profile and a separate ephemeral Preview
profile. It originally required both to qualify before either could ship. The
verified workspace-service lease that Preview needs has no production producer
yet. General already has a separate destination policy and does not need a
workspace-service lease.

## Decision

Qualify and activate the Linux General browser independently. The approved
Chromium for Testing build runs with its sandbox under the service identity,
with a private debugger pipe and a persistent, private General profile. General
uses the public-destination policy of ADR 0078 for all requests, including
subresources, redirects and background targets. The actual target lifecycle,
frame and input path, process shutdown, profile permissions on relaunch, and
network policy must qualify before the native pane is offered. Use controlled
local fixtures for qualification. Windows engine qualification is pending.

Preview stays unavailable until the scripts/services owner supplies the checked
lease and verifies accepted-peer identity, scheme, scope and revocation. A port
observation, PID, URL or health result cannot grant that access. The View service
path remains capability-gated and Preview keeps its separate profile.

Agent tools stay off by default. Discovery waits for the complete 22-tool path,
the setting and the session-start lease, even if General human controls are
available. Control, watch, profile clearing and agent operations retain their
existing scopes. The wire method rows and threshold stay pending until the
integrator assigns a common version. The drafting baseline was protocol 36.

## Alternatives and consequences

Joint activation would ship View service alongside the first pane, while tying
public browsing to the independently owned service lease. An interim system
webview would split the General profile, require another renderer and give
agents a different page from the user. Independent qualification reuses the
same engine and profile while preserving Preview's stricter authority boundary.
The first release will omit in-app service previews and will carry separate
qualification records for General, Preview and the agent tools.
