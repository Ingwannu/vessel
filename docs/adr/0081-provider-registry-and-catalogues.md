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

## Provider and legacy name admission increment

[Decision Log]
- Purpose: prevent new legacy agent.add aliases from shadowing the modern
  provider manifest's copilot/pi/omp identities, including replace requests.
- Existing constraint: AgentName decoding reserves only the older HarnessKind
  spellings. Tightening that decoder would make saved legacy records unreadable
  and could prevent an owner from removing or migrating a collision.
- Alternatives: silently map an Added agent to a built-in account, or duplicate
  the six reserved names in node policy. Both can drift or invent identity.
- Choice: node admission consults the one pure built-in manifest, case-insensitive,
  before command/catalogue lookup or file mutation. A reserved name returns the
  existing typed Conflict code; a broken manifest fails closed. Decoding and
  removal of an existing legacy record stay available, without reclassification.
- Reason: preserve old persisted identity while preventing new ambiguities.
- Effects: exact names are reserved, prefixes remain available. Existing saved
  collisions remain distinct legacy rows until explicitly migrated/removed;
  they confer no modern binding or authority. Modern creation must also refuse
  ambiguous launch binding until the actor can attest the actual association.

The snapshot binding/sign-in increment is still awaiting the actual lifecycle
actor capture producer. No mapping, credential existence or successful catalogue
handshake is used to manufacture a known account or sign-in verdict. This
preparation has no new wire field and does not merge the two UI row sets.
