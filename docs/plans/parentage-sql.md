# Current parentage in notification selection SQL

Paseo agent-manager.ts:5351–5364 excludes current delegated agents; its detached
label patch at726–733 removes that parent relation. Immutable creation provenance
must remain intact while a validated detached child becomes independent.

[Decision Log]
- Purpose: W7's notification eligibility must include a validated detached child
  and exclude an attached child before candidate LIMIT, source claim and final
  push lease admission. Existing Inbox Web Push is a separate active path;
  pending local-attention protocol does not turn its behavior off.
- Existing assets: store owns the serialized Creator and durable DetachReceipt;
  Parentage::of owns their one validation/relationship rule. schema::configure
  is already run for both writer and pooled reader connections.
- Alternatives: SQL receipt-not-null or JSON field comparisons duplicate typed
  receipt validation and can accept malformed/foreign parentage. Post-filtering
  a limited batch starves roots behind internal rows; a new bounded scan would
  change page semantics. A second store lookup loses statement consistency.
- Choice: register vessel_has_current_parent(created_by,detach_receipt) on every
  configured connection. It reuses typed serde decoding and Parentage::of,
  returns true only for Attached, false for Root or validated Detached, and
  raises one fixed safe error for invalid data. Legacy SQL NULL creator is the
  existing Owner default; JSON null/unknown/malformed data is not a fallback.
- Bounds: each fixed-size provenance field is checked before decoding against
  a named1024-byte store-private budget. Rejected values never enter errors.
  Function flags are deterministic and innocuous: the result depends only on
  its two row values, with no database/clock/filesystem/process side read.
- Consequences: a missed connection registration fails SQL closed. The feature
  is enabled on the existing rusqlite dependency only; functions has no new
  dependency. Cargo.lock stays byte-identical; no schema, wire, protocol number,
  peer-capability gate or migration changes. Store truth is not an old peer's
  UI projection, workspace-relative root role or an authority grant.

## Receiving boundary

The new helper registers only the canonical scalar. W7 owns notification_policy
and all three production query consumers, and takes this producer only after
V3 and the integrator announce it. Their source pin is8b246c1a. The isolated
producer tests execute the corresponding SQL join shapes against real migrated
sessions, inbox, devices and push leases. They are query-shape tests, not an
assertion that the still-separate A5 callsites have already adopted the scalar.

Tests cover all canonical parentage cases, malformed/foreign/oversized fields,
missing configure, configured writer/pooled reader, an actual durable detach
behind65 attached candidates with LIMIT1, and real WAL views held across detach
and then source consumption/device revocation. These prove no out-of-band read
and preserve filtering before LIMIT. A5 receiving caller checks remain owed.

No push transport, browser, actual device, credentials, live service or permissions
are used. The private SQL error text is structural and contains no stored values.


## First narrow correction

The first exactad1 all-target strict run stopped on11 test-helper diagnostics:
unwraps inside Result-returning SQL query probes and ignored SQL affected-row
counts. Product scalar/registration had no reported diagnostic. The probes now
propagate the existing StoreError/ToSql errors and assert each intended single
row mutation, without changing SQL, eligibility, assertions or bounds. The red
run is retained; per coordinator rule all correction checks use the ordinary
short lane. No Store test execution or full PASS is inferred from that run.


The next ordinary715 strict check exposed three explicit ToSql type mismatches:
the pinned rusqlite accepts signed SQL integers, not usize/u64 parameters. The
query probes now use checked i64 conversions and preserve the conversion error
inside the existing SQL/StoreError chain. This changes no query predicate or
assertion, adds no panic/default/allow, and remains an ordinary correction.
