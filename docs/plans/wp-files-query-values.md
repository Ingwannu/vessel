# File picker query values prerequisite

[Decision Log]
- Purpose: initial empty-query file rows must not fabricate an invalid empty `FindQuery`.
- Existing assets: the existing validated protocol `FindQuery` remains unchanged.
- Choice: one pure `view::workspace::file_query::FileQuery::{All,Matching(FindQuery)}` owner value. All has no protocol query; consumers may parse text in their UI adapter.
- Why: the source-independent value unblocks command-center DTOs without importing unready C5 connection transport, root facts, requests, widgets or callbacks. C5 will implement initial All through an admitted requested-root list and disclose its parent-only difference until suggestions are assigned.
- Tradeoff: this type grants no filesystem authority and performs no I/O. Actual scoped acquisition/opening remains a separate checked producer; these values are not evidence of runtime parity.
- Validation: one pure regression pins absence of All text, the existing protocol rejection of blank input, and valid Matching text. Remote tests pending; no full gate or runtime claim.

Consumer receipt (W22b): producer59763344 test run
wp-files-query-values-20261115T120324Z-59763344 executed the regression1/0/0,
native-fonts/native-cargo rc0. Only the values/module export and doclint were
adopted here; F4 current-head union checks and runtime integration remain owed.
