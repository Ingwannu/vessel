# Private finish-notice values (ADR0084/ADR0033)

This isolated producer is based on the announced node gate50c head. It contains
only proto values, UUID declaration/export, module registration and documentation.
Its admission.rs and notice.rs are byte-identical to the row251 source-passed
values attachment through c58f35602. No rich receipt, recovery RPC, provider,
watcher, store, migration, policy, runtime or app ancestry is imported.

[Decision Log]
- Purpose: unblock the durable-intent owner with one bounded private namespace.
- Existing assets: UUID newtype macro and the retained-admission ADR budget model.
- Alternatives: public ClientMessageId reuse or another receipt database.
- Choice: NoticeKey {owner, intent, checkpoint}, with closed permission/question
  references and actual original EventSeq, plus one shared set of named ceilings.
- Why: a serialized key remains evidence; only a current execution/parent/grant
  atomic fence can authorize delivery. Public submissions cannot select this key.
- Consequences: source values can be consumed without unfinished rich runtime.
  They do not implement reservation, receipt/queue/event atomicity or recovery.

The intent owner must replace the watcher's existing needs-you ceiling with the
shared NEEDS_YOU_MAX when it wires that owner. It reserves one intent and all
MAX_FINISH_CHECKPOINTS worst-case row/byte slots before original work, against the
same finite session/node budgets. Terminal and irrevocable unavailable share the
last slot. Only committed authorized whole-parent-session deletion releases it.
The internal original-request bound applies only to finish registration, never
ordinary generic SubmitPrompt. Logical budgets do not bound SQLite/WAL pages.

Pure fixtures cover key encoded size for all checkpoint variants at maximum
sequence, request namespace distinction and UUID decoding. No real agent or
provider is run. Exact changed-source strict and pure fixture execution are owed;
this document is not a checked consumer or integration-adoption announcement.
