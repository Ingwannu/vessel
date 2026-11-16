# Web UI identity refusal: WEB-IDENTITY-1

## Recorded incident and acceptance

The unchanged Usage scope case in
`webparity-20261115T143312Z-811c18d5` times out before a Usage interaction,
waiting for the verified tab-session receipt. Its actual DOM says “The UI
was restarted or its identity could not be verified”. Error-context SHA256:
`d5667e687c89c749f2099486637274d897e513c613cc08343de913e0494fb783`.
The one authorized same-source replay,
`webparity-20261115T153059Z-811c18d5`, passes (7.0s case; 26.3s including
fixture). Both results remain recorded. There are no more unchanged replays.
The source investigation, not that pass, determines the correction.

A server that has not received proof before its resource deadline closes
the connection without permanently refusing the tab's identity. The client
backs off and can prove the same unused launch against a fresh challenge.
An invalid proof or misplaced frame remains a final refusal. A consumed
bootstrap never becomes reusable. Authentication and admission waits are
not raised.

## Source finding and chosen correction

The announced base is `9c150c6e` (protocol 24, schema 20), merged under A8.
All 219 incoming changed non-document blobs match that head. The only
architecture conflict keeps its complete text followed by the unchanged
page-feedback section. Incoming generated `web/src/wire/proto.ts` has
trailing whitespace and remains byte-equal to the canonical head under S5;
the broad merge diff-check is therefore recorded as nonzero, not hidden.
The accepted width continuation is merged separately, without executing
or altering its frozen review candidate.

`server::UiConfig` gives authentication a shorter wait than the browser's
own handshake guard. `socket::run` formerly mapped both `timeout` expiry
and invalid proof to `CLOSE_UNAUTHORIZED`. `BridgeSocket` treats that code
as final, and `App` forgets the tab credential and paints the recorded
heading. `Auth::welcome` spends the bootstrap only after valid HMAC proof;
expiry while waiting for a hello does not prove an invalid credential.
This is a concrete source path to the observed state, not a claim that its
unique historical cause can be reconstructed from the retained DOM.

Reuse the existing deadline, close path and browser backoff. A private
closed failure enum separates refusal from elapsed waiting. The latter
uses standard `CloseCode::Again`; no node RPC, version, payload, scope,
credential format, crypto implementation or new dependency is introduced.
The Decision Log is in ADR 0017. The wire change is comments only and is
coordinated with the owner of the separate operation extraction.

## Regression and protected boundaries

- Real server: receive a challenge, let the unchanged product wait expire,
  observe the retryable close, then verify a new connection's fresh-nonce
  welcome with the same original unused launch. No sleep coordinates it;
  the close itself is the barrier.
- Real server: a correctly shaped hello with a wrong HMAC key remains
  unauthorized and does not spend the valid launch code.
- Browser client: a retryable server close redials and proves the same
  session key. The existing final-refusal/no-redial case stays unchanged.
- The sole authorized existing assertion change is the silent-expiry close
  expectation in `a_websocket_must_present_the_token_first_and_in_time`,
  plus its explanation. Wrong-token/misplaced-frame assertions and every
  existing deadline remain byte-for-byte. Original replay, bogus proof,
  stopped-socket and pre-welcome-data regressions remain intact.

Narrow checks and the one whole package gate run on the build VM. No local
Rust compilation, real agent/account, vendor request, live preview or
service mutation is part of this work. V3 reviews this authentication path;
source implementation is not execution or security approval. Checks,
whole-gate evidence and review remain pending at this checkpoint.

## First VM strict result and fixture correction

The first priority strict run, `webparity-20261115T172857Z-a36b0f37`,
fails while compiling the new integration regressions: the sibling module
cannot use a private server-test connection helper, and the library's
random-nonce constructor is private. This is an author test-fixture error,
not a passed Rust execution or an authentication-policy finding. The
previous queued ordinary run ends as an authorized cancellation (143),
not a compiler result.

Keep the connection helper private and move its regression module beneath
its owner. The malformed-proof peer may choose the public server nonce as
its own; its correctly shaped wrong-key proof must still be refused. This
needs no private nonce constructor, new public API or copied connection
helper. Production bytes stay identical to the source-reviewed correction;
only test layout and fixture construction change. Original head assertions
are unchanged except the previously authorized silent-expiry expectation;
one child-test registration is added after them. Subsequent checks use the
ordinary lane, as the one-priority-run ruling requires.

## Validation checkpoint

The corrected source is `7535d60cdace478b183b49287f851e559f96ef18`.
Its ordinary strict run, `webparity-20261115T174436Z-7535d60c`, passes
`clippy --locked -p vessel-web --all-targets -- -D warnings` in 115 seconds.
The raw log records a fresh `vessel-web` compilation. Source review accepts
the production correction and the private-fixture test correction separately;
it is not final runtime approval.

The real-server test run, `webparity-20261115T185321Z-7535d60c`, is still
queued in the ordinary lane. Its plan includes the web bundle and both fake
and daemon prebuilds before `test --locked -p vessel-web --all-targets`.
Neither new Rust regression has an executed result at this checkpoint.
The exact corrected format check, generated TypeScript verification and one
owning root/web gate remain unverified. The earlier compile failure and
queued cancellation remain adverse history, not passing tests.

The continuation is recorded in `backend-m12-web-identity-resume.md`.
This checkpoint changes documentation only; the queued test continues to
select its original immutable source. No new authentication, timeout,
assertion, wire or dependency change accompanies these status notes.

## Subsequent execution and integration

The previously queued real-server run,
`webparity-20261115T185321Z-7535d60c`, completes with all four stages passing:
web bundle, both prerequisite builds, and the actual `vessel-web` tests.
It executes 86 tests (55 library, 31 UI), with zero failures or ignores.
Both new `server::auth_timeout` cases and the protected original websocket
security case pass. The source is exactly `7535d60c`; its log records a fresh
`vessel-web` compilation. This closes the checkpoint's missing Rust execution.

The integration line merges the complete `949dfb15` stack as `ed4cd137`.
Its first whole run is `node-line-20261115T192309Z-ed4cd137`: 17 of 18 stages
pass, Rust 2,701/0/12 (passed/failed/ignored), TypeScript freshness 5/0 with
zero generated difference, web unit 465/0 in 87 files, and forge 4/0. The
same five authentication implementation/test files are byte-identical to
the reviewed source. Both new real-server regressions pass in this whole;
the client suite's ten tests, including the new 1013 regression, pass too.

The full run remains exit 1: web E2E is 79/1. The single failure is
`app.spec.ts:267`, where `startSession` remains on `/new`; the integration
owner investigates it as TRACKER 770. The coordinator counts this head with
that named open intermittent. This is not a wholly green run or an erased
failure. The earlier identity-refusal case passes in this run, which does
not uniquely reconstruct the cause of the historical incident.

The page track fast-forwards to that announced head after the queued job
finishes. Its original whole script is not launched as an additional
unchanged execution: the integration run is the package's first whole.
No retrospective pre/post pin is created. Contemporary run specifications,
checkout records and raw log hashes are retained; selected authentication
blob equality is a separate source comparison. Final bounded runtime review
is submitted with this distinction and the named full-gate failure intact.
There is no live preview, physical device, vendor/account or general security
audit claim, and no new native or service-permission change.
