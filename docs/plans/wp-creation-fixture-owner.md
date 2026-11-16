# Shared retained fixture owner

[Decision Log]
- Purpose: a deadline around a shutdown future must not discard real cleanup.
- Existing asset: the qualified capture fixture already owns actual runtime,
  task drain, Store and temporary homes in one retained task, with independent
  observed completion signals and named deadlines.
- Alternatives: duplicate its actor or time out direct shutdown calls. The first
  duplicates ownership policy; the second can abandon cleanup continuation.
- Choice: reuse that SAME test-only OwnedFixture from the sibling creation
  fixture. Only sibling visibility and one read-only Arc clone accessor change.
  Existing capture assertions, deadlines and cleanup task remain unchanged.
- Impact: CreationFixture consumes the real owner after its body succeeds or
  unwinds. A timed-out cleanup observation does not abort its task or imply
  physical release. Its registered project root is inside that retained fixture's
  fake-bin TempDir, so the actual cleanup owner keeps it through real drain.

This corrects the earlier direct-drain observer design before a safety claim.
The separately queued constructor source remains attributable to its old SHA;
its result is not correction evidence. No product visibility, control/watch
accessor, new registry or positive creation authority is added. The shared
fixture owner remains cfg(test), and joint current checks are still owed.
