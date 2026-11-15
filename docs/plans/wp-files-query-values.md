# File picker query values prerequisite

[Decision Log]
- Purpose: initial empty-query file rows must not fabricate an invalid empty `FindQuery`.
- Existing assets: the existing validated protocol `FindQuery` remains unchanged.
- Choice: one pure `view::workspace::file_query::FileQuery::{All,Matching(FindQuery)}` owner value. All has no protocol query; consumers may parse text in their UI adapter.
- Why: the source-independent value unblocks command-center DTOs without importing unready C5 connection transport, root facts, requests, widgets or callbacks. C5 will implement initial All through an admitted requested-root list and disclose its parent-only difference until suggestions are assigned.
- Tradeoff: this type grants no filesystem authority and performs no I/O. Actual scoped acquisition/opening remains a separate checked producer; these values are not evidence of runtime parity.
- Validation: one pure regression pins absence of All text, the existing protocol rejection of blank input, and valid Matching text. Exact59763344f remote targeted tests completed: wp-files-query-values-20261115T120324Z-59763344, fonts rc0, actual View/Proto compilation and the one file_query regression pass1/0/0 (949 filtered). The earlier048 run stopped before compilation on a stale inherited native lock; this checked head contains the official dd13 lock fix. This is targeted test evidence only, with no standalone all-target Clippy, full gate or runtime claim.
