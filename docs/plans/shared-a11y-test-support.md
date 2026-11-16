# Change 11: real accessibility activation in the test platform

Coordinator6bafe9e8 assigns G2 the two GPUI test-support files; lead6d5654b4
reserved change11 after its change10. Isolated directly on announced2ebeedf9.
No Code/MD, secure-field, F1 area, node or production backend delta is included.

[Decision Log]
- Purpose: ADR0102 requires an observed PasswordInput node without its value;
  policy data alone cannot prove what the real frame emitted.
- Existing constraint: TestWindow discarded A11yCallbacks through the no-op
  PlatformWindow default; the existing Window tree could never activate in tests.
- Alternatives: fake nodes, a second accessibility renderer, or configuration-only
  assertions. These bypass the real callback/frame and cannot establish that proof.
- Choice: retain the real callback, explicitly invoke activation outside the window
  lock, and require its true result before reading debug_a11y_tree_json after a
  real frame. Ordinary tests stay inactive unless they opt in.
- Why/cost: one test-only simulation API, no backend/renderer/default change. Two
  direct Kit dev edges use already-resolved serde/serde_json workspace versions;
  coordinator7f0c2479 explicitly accepts them. The lock delta is only those two
  edges, no other package metadata or coordinate changes; Python pin proof and
  exact remote --locked check validate that statement.

The typed SDK snapshot decoder retains typed aria fields and discards the
meaning of transient SDK map keys; it never prints raw snapshots or derives
Debug for captured node data. A real existing ordinary editor with a nonsecret
reference value is the negative control: inactive before activation, then a real
TextInput role/label/value after it. Future secure producer tests must separately
observe their actual PasswordInput/no-value state after this producer is reviewed
and adopted. This is framework-tree proof, not OS AT bridge or secret-store proof.

Source fmt/size/token/diff/preflight are clean. At the source checkpoint exact
Kit all-target strict and full lib tests were owed; completed results follow
below. No new label/priority/extra whole was requested. The
vendored GPUI's own unit suite is not executed by the Kit suite and is not claimed.
Source headers and every changed block carry Vessel patch/change11 markers.
Base13 lacks later README9/10; lead integration unions them and inserts11 after10.

## Exact first check and complete correction batch

The first immutable31ed5ec0 locked Kit all-target strict run
`schedules-20261116T012338Z-31ed5ec0` ended fonts0/cargo101 (helper1).
It compiled/checked the changed Kit but reported seven test-decoder lints:
five redundant qualifications, one unused trait import and one unit semicolon.
No tests ran. The correction changes only those test spellings; the two GPUI
files, callback semantics, dev-edge/lock proof and assertions remain unchanged.
The original red evidence is retained; corrected strict/full Kit lib results
are still owed on their own immutable head.

## Exact corrected execution receipt

Product/test head87aadffd5cbb61d93484712f7b79872e44464ae1:
- schedules-20261116T013649Z-87aadffd: locked Kit all-target strict0; fonts/cargo0,
  no skip, actual Checking Kit. steps.log SHA
  e82e580dea17b8295953b72c8ad53e0c32bce1d2b520afa1b88e8e1ddb173c79.
- schedules-20261116T014515Z-87aadffd: locked full Kit lib244/0/0; fonts/cargo0,
  no skip, actual Compiling Kit. New ordinary-editor accessibility case explicitly
  executed ok. steps.log SHA
  907cc61793fdaaea0397602e004ba40b9c8e54bbc85762386b1ec96b33a24596.
- Ordinary SHORT, priority0, same schedules label. All30 completed run files,
  exact checkout/specs and four step-log digest prefixes verified; immutable
  Cargo.lock package/coordinate comparison confirms ONLY the two existing Kit
  dev edges. No new package/version, production edge or other package change.

This proves the opt-in real callback/frame/debug-tree path for the ordinary
negative control. It does NOT prove the child secure field, Linux pixels, OS
AT/IME, secret store, F1 or Host Voice. Vendor's own tests were not executed.
Original31ed lint-red evidence stays separate. This docs-only receipt changes
no Rust; bounded V1 source/narrow disposition and lead line adoption remain owed.
