# Typed observation of an SDK log's end

Status: separate additive SDK producer for review, with no wire, method, node
runtime or native-app change. It does not include the stream-identity package.

## Decision Log

- Purpose and intent: distinguish an actual normal subscription end from other
  reasons LogStream stops returning records. Native lifecycle cannot interpret
  generic None as an archive acknowledgement or a normal-end observation.
- Existing assets and constraints: the generic LogStream source already has
  Live, Paging and Done states. Done previously combined a typed SessionEnded,
  an already-reported fatal error, and acceptance of the final EventSeq.
- Alternatives: infer normal end in each caller; add a separate cached flag;
  or retain the typed cause directly in the existing terminal source state.
- Choice: Source::Done(LogEnd) with a public read-only end_reason getter.
- Why: the fact stays with the actual transition that observed it, including
  catch-up and fatal paths. No parallel state or identity authority can drift.
- Benefits, costs and effects: the API is additive and keeps next(), errors,
  cursor, catch-up, held records and existing caller behaviour unchanged. The
  enum is only a local observation; consumers still need their original lease
  and resident/ticket fences and cannot treat it as mutation acknowledgement.

## Contract and data flow

`vessel_client::LogEnd` has SessionEnded, FatalError and SequenceExhausted.
`LogStream<L>::end_reason(&self) -> Option<LogEnd>` returns None while receiving
or paging. Only the actual ordered SubscriptionEnd::SessionEnded transition
sets SessionEnded. Existing fail() retains the original error and records
FatalError. An accepted record with no successor sequence records
SequenceExhausted without pretending a normal End frame arrived. Subsequent
next() calls keep returning None and never upgrade a terminal cause.

The generic implementation applies to SessionLog and TerminalLog. The wire
historically calls both normal log ends SessionEnded. This observation proves
neither provider exit nor terminal descendant/I/O cleanup, and grants no close,
draft-clear, retry or lifecycle success. Nonfatal Missing leaves the stream
receiving with its held record; lag catch-up and resubscription also remain
receiving until one of the actual terminal transitions occurs.

The native follow adapter may consume this only after the reviewed SDK package
lands in an announced node head and reaches the native line. It must join the
typed normal reason with the same original delivery guard and full binding.
Existing callers need no change; a status record or generic None remains
insufficient evidence. Expected-end ledger and Window changes are separate.

## Verification

Four actual framed client tests exercise normal End after a preceding record,
the original Unsubscribed error before fatal None, MAX sequence while the peer
producer retains its sink without sending normal End, and nonfatal Missing
followed by its held record. Repeated None must retain the distinct reason.
The final-sequence fixture releases its reporting task and waits for its completion notice explicitly.
Tests and strict checks run on the build VM, not as local Cargo invocations:

Sequence exhaustion still drops the client's live Subscription, whose existing
Drop may issue best-effort unsubscribe. The fixture does not claim that the
remote registry entry remains open after that client action. It establishes
that MAX caused exhaustion without observing a normal SessionEnded frame.

```sh
cargo clippy -p vessel-client --all-targets -- -D warnings
cargo test -p vessel-client
```

Source preparation alone is not a passing result. V3 review and the integrator's
announced landing remain required before any native consumer adoption.
