# LSP LRU fixture deadline

Status: source correction; targeted execution and strict check pending.

[Decision Log]
- Purpose: verify that filling the document bound evicts the least recently
  used document and permits reopening it, without turning scheduling pressure
  into an unrelated deadline assertion.
- Existing assets and constraint: manager Config::default uses 30 seconds;
  lifecycle config() deliberately overrides it to 3 seconds for quick fixtures.
  The failing case uses the real fake-server stdio and performs 65 sequential
  document queries. Its recorded failure was Timeout on file 55 at that shorter
  fixture bound. The log alone does not establish a scheduling cause or a
  product-default timeout failure.
- Alternatives: a new fake-server barrier protocol or a larger global fixture
  deadline would affect more tests than this capacity regression needs.
- Choice: one named LRU_REQUEST_TIMEOUT of 30 seconds, scoped only to this test.
  It matches the ordinary product-sized budget but changes no product default.
- Why: timeout behavior belongs to the separate hung/malformed-peer and restart
  tests; LRU must test capacity over the actual transport.
- Tradeoff: a genuine stuck LRU query can take longer to fail. The existing
  90-second whole-case bound remains, all per-file success assertions remain,
  and the eviction/reopen/stop assertions are unchanged. No retry, sleep,
  ignored error, reduced file count or unbounded wait is introduced.

The failed whole run was on exact2f1c9f5a8 and remains failed. The LSP crate
has no source difference between the earlier d432ef0f and announced1c762ccf
node heads and the subsequent announced93970acb head. This leaf is based on
93970acb; it changes only lifecycle test
configuration and this explanation. It provides no live service or native UI
proof. All Rust execution uses the build VM.
