# Move the existing LinkError boundary on the lead0.19 base

[Decision Log]
- Purpose: keep the transport actor smaller as schedule registrations are added.
- Constraint: the public root path, enum body, diagnostic Display, variants and
  actor must remain unchanged; another track owns the separate options move.
- Alternatives: enlarge the file allowance or move the whole command/event bus.
- Choice: move the existing LinkError to error.rs, root pub use unchanged. A
  doc-only Event import resolves its original intradoc reference in the new scope.
- Why and cost: one independently readable error vocabulary, no runtime import
  or API/actor change. No change from the other track's options block is taken.

The entire moved block, including its comments, is byte-identical with SHA256
98601c041f87993948f61fe66180f0ee13e32efc6a7e807c652f8349bb2bb526.
No original doc-link line was changed; the new module adds a cfg(doc) Event
import instead. The root falls988 to944 product lines. No variant, Display,
public root path, call, actor state or matching test expectation changes.

This is a source-only intermediate commit, as explicitly instructed after the
ratchet check was read: retaining988 below the measured count cannot pass that
check by itself. No knowingly failing quality/whole gate is run for this commit.
The immediately following schedule-registration commit lowers the record to its
final measured number without ever increasing it. Only that combined head is
gated. The isolated move must be reviewed as source-only, never as independently
gated. Both hashes travel together, move first, in the0.19 hand-over.
