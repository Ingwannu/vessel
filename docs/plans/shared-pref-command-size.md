# Box the native preference-save command

[Decision Log]
- Purpose: growing typed preferences must not enlarge every native command.
- Constraint: Sidebar preference bands made inline SavePrefs472 bytes versus
  the next variant249, crossing strict large_enum_variant. The same limit
  affects other preference consumers.
- Alternatives: a lint exemption would leave the cost; boxing individual
  preference fields solely for this threshold would change their public
  representation and fail again as more fields arrive.
- Choice: Command::SavePrefs(Box<Prefs>) and ten mechanical constructor changes.
  Standard Box supplies one owned allocation per infrequent save.
- Why: preserve the canonical Prefs value, all keys, JSON, validation and sole
  store/save path. No wire/RPC/schema/policy changes or new dependency.
- Impact: native Rust constructor API now owns Box<Prefs>. The existing
  spawn_blocking dispatch calls prefs.save through ordinary dereference;
  web/unsent match arms retain their existing behavior.

This isolated producer starts at announced c5263270, outside the F4 branch.
Only command.rs and seven app caller files change, plus this decision note.
Coordinator explicitly assigned S2 the sole mechanical writer and requested
lead-first handoff. Non-cargo format/size/boundary/token/import checks are the
available source checks; Rust execution remains owed to the lead integration.
No local Cargo/Rustc, runtime, persistence or full feature pass is claimed.

Integration result: the lead included exact b8390c60 in announced c3429952 and
reports native1450 passed/0 failed/2 existing ignored. The earlier41eee587
candidate was superseded to remove an unrelated withdrawn registry carrier.
This is the lead's combined native evidence, not an independent execution on
the isolated producer branch. The command/call-site change remains mechanical;
JSON, store and policy are unchanged.
