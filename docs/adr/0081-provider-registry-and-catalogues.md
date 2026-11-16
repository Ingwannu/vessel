# Provider registry and catalogues

Status: proposed

## Decision Log

- Purpose and intent: retain provider identity, permission modes and catalogue
  state independently of the transport that happens to run an agent.
- Existing implementation and constraints: four harness descriptors, added ACP
  agents, profiles, a shared options/readiness probe and a Codex direct driver
  already exist. They do not form a provider registry. The Claude and OpenCode
  direct drivers are refused; Pi and Oh My Pi have no drivers. New RPC methods
  wait for the shared method-table change and an assigned feature version.
- Alternatives considered: expanding HarnessKind for every configured provider;
  silently mapping provider mode names to ACP values; replacing existing launch
  and probe paths; a separate provider framework dependency.
- Choice: extend the existing proto, harness and agent boundaries. Proto owns
  validated provider values and unregistered request/reply shapes; harness owns
  the manifest, launch overlays, registry and create-mode policy; the node owns
  catalogue refresh state. Reuse CommandSpec, Lookup and the ACP probe. No new
  external dependency or SQLite schema is required for the first stage.
- Why: configured and plugin providers are an open set, unlike HarnessKind.
  Provider policy must not imply that a missing driver works. Existing ACP
  choices retain their native names until an explicitly tested adapter exists.
- Advantages, costs and effects: source-only policy can be tested without any
  installed agent. Product calls remain unavailable until wiring is reviewed;
  complete native provider execution requires additional driver work.

## Security and compatibility

Provider commands are argument vectors, never shell text. Environment values
remain redacted; inherited Claude parent-session markers are removed after all
overlays. Configured providers retain Vessel's explicit-inheritance security
floor instead of copying the daemon environment. Denied tools are retained as
typed policy and must be refused by a transport that cannot enforce them.
Plugin registration supplies inert definitions only; executing plugin code or
trusting a plugin is the plugin runtime's responsibility (ADR 0068).

The feature protocol is unassigned. These values do not add fields to existing
replies, register calls or change the current protocol. Older peers see exactly
the existing replies. A later wiring commit must gate both directions through
the shared request accessor, and test nested/forwarded replies as well.

No platform-specific service or signal operation is added by the policy layer.
Command resolution uses the existing cross-platform Lookup. No provider is
started, installed, authenticated or contacted by the manifest or resolver.

The common session preparation seam also admits native version helpers before
starting them. Its caller-owned non-serializable gate delegates fresh decisions
to the host's existing arbiter and can wait on that host's existing policy watch.
It preserves the original guard/token/context through actual lookup and version
root settlement, including caller drop. This extends the existing runtime
tracker; it adds no policy registry, quota, persistent identity or reaper.

## One native read transport

[Decision Log]
- Purpose: native catalogue and history reads must use the same bounded process,
  initialization, ID correlation and response decoder, while keeping their own
  typed request/response projection.
- Existing asset: the native history owner extracted `codex::read_probe` from
  catalogue transport. Maintaining a second runner here would let cancellation,
  notification limits or protocol refusal diverge between read operations.
- Choice: catalogue delegates to the same `Transaction` through `ReadRequest`
  associated response types. Only a method-not-found response is optional;
  malformed successful data and unsolicited execution requests still fail.
  Catalogue retains its version parser, model/config interpretation, pagination,
  metadata budget and current mode policy.
- Tradeoff: a real public catalogue-error consumer requires the shared safe error
  class to be publicly reachable; native content remains outside that error.
  The unused history-only in-memory test constructor stays with its receiving
  tests until their union. No second runner, new dependency, wire method or
  newly advertised history capability is introduced by this delegation.
- Cleanup scope: the existing terminate/wait/kill request sequence is preserved.
  This refactor does not prove descendant death or settle the separate process
  cleanup audit. Exact joined runtime verification remains required.
