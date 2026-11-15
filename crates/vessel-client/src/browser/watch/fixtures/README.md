# Watch metadata test image

owned-page.png is the 390x844 PNG from W14's own synthetic "Private pipe fixture"
(2026-11-15 private-pipe diagnostic4, sandboxed Chrome149, no real page/login).
It is our test data, not a copied reference asset. Only the binary image is
retained here, without a profile, endpoint, cookie or arbitrary page content.

The cursor tests validate scope/revision/CSS metadata against a real PNG payload.
They do not produce an application frame, execute an engine, validate codec/paint,
or qualify the runtime, proxy, Preview or Windows. The staged model cannot open
a stream or turn NoEngine into supported content.
