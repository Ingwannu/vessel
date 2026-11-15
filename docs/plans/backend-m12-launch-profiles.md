# Named launch profiles (ADR 0076)

## State

V3 accepted ADR source `3f0ae46d` as contract/documents only. The node
integrator allocated store schema 19, after title origin's schema 18;
when its baseline is consumed the migration is appended using the named
predecessor rather than repeating a number. No migration is added here.

Core values now implement name/notes/appearance, bounded existing choices,
stable ids, exact revisions and whole ordered snapshots with count/id/JSON
byte invariants. They share immutable arrays, expose no mutable inner
values, and register no call. Decoder diagnostics redact untrusted input;
canonical JSON is counted without allocating a serialized copy. The list
budgets the maximum revision, including JSON escaping. No catalogue,
method, store migration, CLI, MCP, runtime probe or UI is built. Narrow
checks have passed as recorded below; this is not package completion or
runtime approval.

The first selected Clippy check failed on a getter conflicting with TS's
trait method and test-only lint findings. The getter is now `display_name`
(the serialized field stays `name`); tests use checked division and a
module-level unchecked fixture representation. That failed log is retained.
At source commit time, corrected-source checks were still pending; the
following checkpoint records their actual executions.

### Core-value narrow checks

Source `9375732b` passed TS-enabled, all-target `vessel-proto` Clippy
with warnings denied (`wiregaps-narrow-20261115T055731Z-9375732b`).
Its `vessel-proto --features ts --lib` tests passed 275/0/0, including
all nine new value cases (`wiregaps-narrow-20261115T060103Z-9375732b`).
The latter runner also rebuilt the page and both nested binary prerequisites.
Actual proto recompilation and all step return codes/digests were checked;
nine selected changed Rust inputs equal the exact source commit. No full
package gate or implementation review is claimed by those narrow checks.
The source commit remains a WIP of the feature, rather than a callable API.

The new A10 assignment expands this package to draft/live application and
legacy favorite migration. The accepted contract's running-apply omission
is historical scope, not completion of that expanded assignment. The
survey's migration module/tests and picker/materializer paths are being
read before a contract amendment and implementation of those behaviors.

Quality B's announced pending row remains a wiring prerequisite; the
integrator's row shape is `[Scope, (pending NAMED_LAUNCH_PROFILES), unary]`.
No dummy/current protocol number or second registry is used.

The prerequisite merged baseline is `44cbf0d2`, merging announced green
node head `242076b5` into `f9752ef7`. Its only conflict was architecture
prose: incoming paragraphs remain, and the staged project-list paragraphs
are a separate section. No head test, protocol/schema or generated wire
file was rewritten. Inherited generated TypeScript whitespace remains
byte-identical to the incoming head; regeneration freshness passed.

Complete root gate on that exact baseline:
`wiregaps-20261115T044022Z-44cbf0d2`. All eighteen steps exited zero:
workspace 2,252 passed / zero failed / twelve existing ignores;
TypeScript four tests and no declaration difference; web unit tests 442,
E2E 65; forge feature four. Both nested prebuilds ran. Execution used eight
build jobs/test threads and private SSD test storage; beginning/end
one-minute load was 9.68/8.92. The pre/post selection of 1,642 tracked
Rust/Cargo and root policy/gate inputs matched the exact commit. This is
selected source evidence, not ignored-config/toolchain/machine attestation.
Raw step logs, hashes and summary are supplied outside the public tree.

The earlier ADR 0061 correction `ed34a92b`/results `f9752ef7` received
source-reviewed/staged acceptance. It remains private cfg(test) staging:
there is still no callable project-wide pull-request listing or permission
to merge that staged service onto the product node line. Its original
short timeout and all earlier red runs remain retained.

## Package and order

1. Review ADR 0076, including account visibility, whole-list CAS, bounded
   draft probe ownership and the explicit running-apply omission; allocate
   the store schema through its owner. No implementation precedes review.
2. Add bounded values with constructor/decoding parity and reference
   materialization/clear/order cases. Use current existing typed options;
   do not add a provider capability table or arbitrary JSON.
3. Consume announced Quality B at a clean boundary under the merge rules.
   Add the allocated store migration and its transaction tests, node
   service/probe and method plumbing as one feature package. The feature's
   protocol remains unassigned until the integrator sets it. Staging before
   product wiring follows the existing cfg(test) rule, without public or
   lint workarounds.
4. Add the shared client materializer, typed client methods, CLI whole-list
   read/replace/options commands and MCP list read. Add fixture data and
   actual contract checks to the existing fake fleet. No native/page
   implementation is part of this package.
5. Run narrow changed-crate checks per commit, then one complete root gate
   at package end. Submit the range and actual evidence once. Unassigned
   refusal and integrator-assigned happy-path/combined gate remain distinct
   evidence; no local test version is invented to bypass admission.

Expected module owners are named in the ADR. Storage extends the existing
writer and reader; probe changes extend the existing driver-neutral seam.
There is no new external dependency or crate edge.

## Review fixtures

- Stable ids, duplicate display names, editing replacement/removal of
  optional fields, exact reorder and successful empty list versus errors.
- Each named bound at/beyond its edge, escaped JSON byte budget, UTF-8,
  invalid controls, duplicate ids/choices, unknown replacement fields and
  an invalid later record failing the entire stored read.
- Matching/stale/no-op revisions, two concurrent writers, checked revision
  exhaustion, per-operation rollback, batch commit failure, lost caller
  acknowledgement, restart and unchanged ownership/private modes.
- No account/environment values in replies or validation errors; a missing
  saved account remains editable, but cannot silently launch another.
- Real existing fake ACP/direct driver behavior, model-dependent options,
  current defaults and stale/reload isolation. No probe prompt or persistent
  Vessel session; finite admission retained through cancellation/cleanup.
- Scope/origin intersections and every direct/nested unassigned/older hop,
  generated TypeScript, CLI/MCP and isolated seeded fleet consumers.

## Remaining boundaries

Paseo's fixed fields are represented by agent-reported choices; only
features the active driver actually reports are offered. Null/reset and
running bundle application are not fabricated. The screen must distinguish
unsupported/disconnected/unread from success-empty, and must not infer a
running Apply capability from CRUD. Full differences and why are in the ADR.

Next outside this package: callable ADR 0061 project listing still needs
Quality B/wiring/authority/older-peer/client evidence; registry/version
row 6 and webhook row 7 remain incomplete. Existing account-profile CRUD
is not supplied by named launch profiles. No live service, production
permission, real credential, real forge or other-OS runtime was changed
or verified during this contract work.
