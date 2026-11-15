# Registry metadata activation boundary

Status: source correction for review; no protocol assignment or supported-wire
execution is claimed. The registry remains pending behind its existing optional
feature constant. The complete metadata dependency and actual resource effects
are deliberately separate scopes.

[Decision Log]
- Purpose: allow eventual activation of the six complete registry metadata
  methods without implying readiness of filesystem provisioning or lifecycle.
- Existing constraints: the generated guard rejects agents, but delegated
  person origins are not agents. A forwardable method can therefore reach the
  handler with delegated people despite the direct-person activation boundary.
- Alternatives: treat every delegated caller as an agent globally; change the
  forwarding table for unrelated methods; or infer identity from origin text.
- Choice: one exhaustive principal check at the beginning of each of the six
  registry handlers, before Store read/watch/mutation. Owner and paired Device
  continue through the existing generated scope check; Agent and Delegated fail.
- Reason: this keeps the narrower registry boundary local, uses the existing
  authenticated Principal and leaves all unrelated fleet policy untouched.
- Impact: registry metadata cannot be read or changed through another node's
  delegated delivery, including delegated owner/device origins. Self-forwarding
  retains the original principal and still follows ordinary scope checks.

The six methods are workspace.registry, workspace.list, workspace.get (Read),
and workspace.rename, workspace.pin, workspace.release_automation (Operate).
No request/response value, method row, scope, feature value, advertised version,
store schema, generation, resource lease or dependency changes here. The old
special case disabling long-poll for Delegated is unreachable and removed.

Two authored unit tests cover direct people and direct Agent plus every
Delegated origin with all scopes. They verify the local class boundary, not
actual device pairing, guarded transport, old-version refusal or supported-wire
success. Those must execute against the integrator's announced assigned head.
The approved registry dependency's historical gate qualifications stay intact;
this correction neither reruns nor relabels its original evidence.

Checks for this source are pending. All compilation and tests run through the
remote build helper; local checks are formatting, size and diff only. Protocol
assignment remains owned by the integration line after the assignment packet
review and scope approval. Provisioning, generation, archive/restore, config IO,
editor launch and D2 effects remain excluded, and full C1 is incomplete.
