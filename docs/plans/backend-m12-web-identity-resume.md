# Authentication timeout validation continuation

The implementation and test-fixture correction are committed in `a36b0f37`
and `7535d60c`, after baseline `3015b640`. The review accepts their source;
final execution and integration approval remain pending. This checkpoint is
documentation only and does not broaden that review.

The ordinary Clippy run `webparity-20261115T174436Z-7535d60c` passes in
115 seconds with a fresh `vessel-web` compile. The earlier priority compile
failure and canceled queued run are retained. The real-server test job
`webparity-20261115T185321Z-7535d60c` remains ordinary and must not be
canceled, duplicated or promoted. It selects source `7535d60c` explicitly.

After that job finishes, read its actual test names, counts and failures.
Require both the expiry/fresh-proof and well-formed wrong-proof regressions
to execute. An exit code without selected tests is insufficient. If it is
green, run the one owning root gate through the managed build runner, with
both prerequisite builds, strict checks, generated TypeScript verification
and the complete web gate. No owning whole has run for this package yet.
Do not compile Rust locally or test against a real saved sign-in, provider,
agent or preview service. No fixture fleet or display remains active from
this package.

The source pin must be contemporary with the owning whole, and must say
which inputs it selects. A local committed-source comparison does not attest
the remote dependency or runtime environment. Preserve the old failed logs;
record any changed source and its actual compilation before final review.
Then commit the results and submit the exact package to the authentication
reviewer. Latest-line integration remains a separate combined gate.

The correction proves a reachable classification fault, not the unique
historical cause of the recorded refusal screen. A code already consumed by
a valid proof stays consumed if its welcome is lost. The change restores no
code and relaxes no authentication check or deadline.

At the next clean package boundary, merge the announced node head using
the canonical merge rules; do not merge into this outstanding execution.
The next assigned feature is the web Usage parity consumer. Its report and
source-consent methods still require a reviewed, assigned and merged node
contract before activation. Outside preparation is not an executable page,
wire handoff or feature acceptance. No native screen or live service change
is part of this continuation.
