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

## Native Codex permission dialect increment

[Decision Log]
- Purpose: match Paseo's offered native modes while retaining existing native
  callers' explicit policies without reinterpreting actual ACP observations.
- Constraint: auto-review requires observed Codex0.115 or later; unknown/below
  versions must not select a reviewer the native process cannot support.
- Alternatives: always offer the reviewer, or silently fall back/escalate;
  neither preserves the caller's explicit security policy.
- Choice: offer auto/auto-review/full-access with version-gated default; accept
  unoffered read-only and explicit legacy native spellings with identical policy.
- Reason: the typed already-observed version supplies compatibility without a
  second executable probe, invented ACP mapping or shell environment flag.
- Effects: unknown versions default auto and reject auto-review before a thread
  mutation. The same projection drives live and prompt-free catalogue options.
  Goal version support remains only a floor, never command availability.
