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
