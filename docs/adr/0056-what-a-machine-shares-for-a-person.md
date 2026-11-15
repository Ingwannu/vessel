# ADR 0056: What a machine shares, read by a person

Status: accepted, 2026-11-14 (milestone 12's node line, protocol 16).
The wire is `fleet.published` (`crates/vessel-proto/src/fleet_published.rs`);
the node reads it in `crates/vessel-node/src/fleet/published.rs`.

## Context

The native app's Settings › Sync page showed, for a home, what it shares,
and read it with `fleet.manifest`. That call is the member protocol: a
member node fetching its home's bundle, under the `sync` scope (ADR 0012),
which is independent of the others and which no device a person pairs
holds (`vessel node pair` grants read, operate and admin). On a real node
the page was refused `forbidden` and said "use an admin device" to an
admin device. Its captures had not shown it because the fake fleet's app
connected with the node's own token, which holds every scope.

`fleet.status` (protocol 15, ADR 0044) gives a machine's role and last
apply to any device with `read`, but not what a home shares: nothing
person-facing did.

## Decision

[Decision Log]
- Purpose: a person sees what a machine shares through its fleet, or last
  received, without the member protocol.
- Constraint: names and metadata only. No file contents, no secret
  values, no tool server's command, arguments, address or headers: the
  bundle carries all of those and a page has no use for them.
- Alternatives:
  - A field on `fleet.status`: status stays cheap and readable by a
    read-only device, while a published list can be long.
  - `admin` (or `read`) allowed on `fleet.manifest`: it hands out the raw
    manifest with its commands and literal values, and the member protocol
    would change for a page's sake.
  - `read` scope: a read-only device would learn every tool server and
    secret name. Starting at `admin`, as `fleet.members` and `secret.list`
    are, is least privilege; relaxing later is additive, tightening is not.
- Choice: a call of its own, `fleet.published` (`admin`, since 16):
  - A home answers its bundle, `None` before its first one.
  - A member answers the bundle it last received, the home it came from,
    and the version it last applied (older when the last apply failed).
  - A standalone machine answers that it shares nothing, as a value, not
    an error.
  - Each file is given by path, kind (instructions, skill, other, from its
    place in the bundle), size and executable flag. Tool servers are given
    by name, and the secrets they name by name, each once.
- Not forwardable: each machine is read over its own connection, and a
  home already hears its members' applied state (`fleet.members`,
  `fleet.status`). Forwarding can be added later without breaking anyone.
  The browser bridge allows it (ADR 0017), as it does every person's call.
- Bounds, as `fleet.status`'s:
  - `manifest.json` is read up to 16 MiB.
  - The bundle is refused past 4096 files or 256 tool servers.
  - The reply must fit the reply budget (1 MiB less the envelope).
  - Past any bound the whole read is refused, `unavailable` with
    `FleetControlError::PublishedTooLarge`, never cut to a shorter list a
    person would trust.

## Consequences

- The node reads `manifest.json` (written by a home when it publishes and
  by a member when it pulls) and `state.json`, the files `vessel fleet
  status` reads, so the answer holds whether or not the fleet loop runs.
- Agents never call it: they hold no `admin`, and the workspace rule
  refuses it to them as well.
- `vessel fleet published` prints it from the selected machine.
- A device needs `admin` on the machine itself. A read-only device sees
  the role (`fleet.status`) but not the list.
