# Subscription identity

The RPC layer authenticates the node and decodes subscription frames. The
client stream layer additionally joins the returned metadata ID to the exact
requested session or terminal ID before exposing metadata or records. It applies
the same check after lag catch-up, when a new subscription replaces the old one.

[Decision Log]
- Purpose: prevent a valid subscription response for another log from being
  installed under the requested log's identity.
- Constraint: a connected peer and a valid response shape do not prove log
  identity. Checking only the initial open leaves automatic resubscription open.
- Alternatives: callers can inspect metadata, but that duplicates policy and
  cannot protect records consumed by automatic catch-up.
- Choice: `Log::info_matches` performs the typed join at stream establishment.
  Session and terminal logs compare their own IDs; custom log implementations
  without a check fail closed.
- Impact: `follow` and `follow_terminal` return `StreamError`; ordinary RPC
  errors remain available as `StreamError::Call` with their original source.
  `UnverifiedIdentity` is fatal for that stream, without retiring the connection
  or retrying another target. Wire types and protocol numbers are unchanged.

The in-memory RPC tests cover matching and mismatched session and terminal
metadata, a mismatch on resubscription, and continued connection usability after
refusal. Run `cargo test -p vessel-client` and strict all-target client Clippy
through the prescribed remote build runner. A passing client check does not
prove downstream CLI or native consumer integration.

The SDK and its node/CLI consumer adapters are merged onto announced node head
`1c762ccf` (after the `9c150c6e` merge) for current integration checks. Earlier 84 unit tests and two
documentation tests passed on the original SDK source; merged-head checks remain
separate evidence. No native dependency is advanced outside its shared base.

The identity correction's framed tests use canonical `vessel_testkit::within`
and named shorter waits for authentication, subscribe, record/catch-up and
post-refusal connection use. This test-only A7 correction preserves the five
identity cases and all SDK behaviour; the receiving union must execute the
corrected fixtures rather than borrow the predecessor's passing run.
