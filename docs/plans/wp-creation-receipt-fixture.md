# Durable receipt fixture dependency (staged)

[Decision Log]
- Purpose: common creation admission tests must use the actual committed Begin,
  not construct a receipt that merely resembles an acknowledgement.
- Existing asset: creation admission/CAS already uses the canonical Store writer,
  one savepoint, retained claims/count/byte budgets and commit acknowledgements.
- Alternatives: synthesize a receipt, introduce a test ledger, or attach that
  existing implementation as a distinct source dependency.
- Choice: retain the existing creation Store implementation and its regressions,
  without the profile mutation API, observer RPC, controller or filesystem driver.
- Reason: test fixtures must exercise the same transaction/commit owner as the
  eventual driver. This is not permission to activate the unfinished controller.
- Impact: the two canonical append-only migrations remain unchanged, including
  their empty profile-settings table. No profile mutation implementation is
  included. The receiving integrator repoints the predecessor constant when this
  package is qualified; these private development entries have no assigned schema
  numbers. Namespace/retention and corruption tests remain part of this dependency.

This attachment is unreviewed implementation source. Earlier whole-tree checks
are predecessor evidence only. Current strict and actual affected tests are owed;
no receipt/launch/production adoption is implied. The launch and placement modules
remain literal cfg(test), and common insertion/stop/settlement remains unavailable.
