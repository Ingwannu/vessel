# Browser result parity debt

This is a pre-qualification implementation checklist, not an approved result
projection or a supported engine. The single 22-tool catalogue and input parser
do not prove result parity. Current `BrowserResult` and the test-staged MCP
output renderer are a foundation; the missing observations below remain owed.
No additional tool-name table, protocol number or advertising is introduced.

Reference behavior was read in protocol browser-automation/rpc-schemas.ts
(lines260–498), desktop browser-automation/service.ts (executeLogs at801–824),
server browser-tools/tools.ts (result/content projection at770–922) and desktop
browser-automation/ipc.ts (console retention at205–220). These paths are relative
to the reference packages. This document describes behavior; reference code and
assets must not be copied, adapted or vendored.

## Per-operation result checklist

The command discriminator and actual owned browser id accompany target results.
The node must derive effect facts from the actual completed operation. A requested
URL, ref, point, value or source string is not a substitute for a missing engine
observation. Correlation remains the authenticated Vessel RPC/context, not a
caller-selected cwd or control endpoint.

| Operation | Reference result behavior | Current foundation and owed work |
| --- | --- | --- |
| List tabs | Scoped tab records include address/title, active/loading and history eligibility | Typed inventory exists; no real inventory/focus observation. MCP currently projects richer internal load/history enums rather than the reference flags. |
| New tab | Flat id/workspace/address facts from the created tab | Staged projection now flattens the completed Tab id/workspace/address; actual creation and changed-source checks remain owed. |
| Snapshot | Address/title/workspace, ARIA-YAML marker, text, truncation and named measured stats | Staged traversal now returns same-document address/title under the loader fence, and MCP explicitly projects aria-yaml/text/truncation/named stats. Actual engine execution, owner workspace context and changed-source checks remain owed; no later-document lookup is joined. |
| Click | Selected current ref and optional actual point | Applied has only id; typed action completion/projection remains owed. |
| Fill | Current element ref on successful fill | Applied is insufficient for the final result projection; no executor yet. |
| Wait | Which condition matched | Matched currently has id only. Actual predicate execution and matched-kind result remain owed. |
| Type | Optional ref and optional actual point | Applied has only id; active-element/ref execution and result remain owed. |
| Keypress | Key identity, optional ref/point | Applied has only id. A complete tool chord and observed target completion differ from the finite human raw-key subset. |
| Navigate | Actual resulting address | Staged projection now flattens the completed Tab address; real navigation and changed-source checks remain owed. |
| Back | Completed traversal id | History/controller model exists, no qualified actor/result flow. |
| Forward | Completed traversal id | Same boundary as Back; no invented empty-history success. |
| Reload | Completed reload id | Controller model exists, no qualified actor/result flow. |
| Screenshot | PNG metadata plus one image block | Staged projection omits duplicate image bytes as intended. Actual first paint/capture, full-page handling, codec/paint and source fencing remain owed. |
| Upload | Current file input ref and successful file selection names | Applied has only id. Confined staged regular files and selection results remain owed. The current nonempty path-list DTO has no item/aggregate path budget; settle named decoding and staged-copy count/byte owners before real wiring, rather than treating lexical values or the transport cap as staging admission. Internal private staging paths must never be exposed. |
| Select | Current select ref and selected value | Applied has only id; actual selection, events and result remain owed. |
| Hover | Current ref and optional actual point | Applied has only id; hit testing/actionability and result remain owed. |
| Drag | Both current refs and optional actual source/drop points | Applied has only id; full trusted drag sequence and result remain owed. |
| Logs | Console entries and resource timing/network entries, each requested tail | Current DTO is a deliberately incomplete typed foundation; see log debt below. No real producer/ring exists. |
| Evaluate | Actual serialized result plus truncation | Evaluation has capped JSON but internal field names/shape differ. Staged explicit MCP projection now preserves observed resultJson as an opaque string plus truncated; real engine result/exception behavior and changed-source checks remain owed. Never echo the function as a result. |
| Scroll | Applied deltas, optional ref and actual point | Applied has only id; element/actionability and result projection remain owed. |
| Resize | Actual resulting width/height | Staged projection now extracts completed effective dimensions for fixed/responsive modes, not requested ones. Actual resize and changed-source checks remain owed. |
| Close tab | Completed close id | Controller now retains the target until ACK. Actual actor destruction/watch/slot settlement remains owed. |

## Shared result and log debt

- The reference returns bounded handled-dialog observations alongside both success
  and failure, and appends a human-readable dialog summary. ADR0078 already calls
  for those observations. Current BrowserResult has no typed dialog container or
  executor. Add the existing20-dialog policy's real observations; an ignored
  dialog or empty synthetic list cannot hide an action that never completed.
- Reference console entries include severity/message, optional source/line and a
  timestamp. Current ConsoleEntry has typed severity and redacted text only.
  The omitted fields are parity debt, not a claimed privacy exception.
- Reference resource entries include timing/type/size facts and optional
  method/status. Current NetworkEntry has HTTP URL/method/typed outcome only.
  Preserve actual clock domains/units and absence, not zero timings or guessed
  methods/status. Treat displayed resource addresses as content, never as a
  capability to navigate/fetch a non-HTTP or private destination.
- The reference's MCP text is a human-readable operation summary, while structured
  content carries result/context and optional dialogs. Current staged output emits
  JSON text, no context/dialogs, and several internal result field shapes. A final
  typed projection/independently written summary remains owed. Its owner context
  must come from W18's authenticated tool context; inputs cannot mint authority.
- Screenshot's image-free metadata plus one PNG block and omitted favicons are
  already deliberate documented projections. UTF-8 byte limits, explicit
  truncation, typed missing/failed outcomes and no client-side favicon fetch stay
  as ADR0078 decisions. They do not waive the missing result observations above.

These gaps must be resolved or explicitly reviewed as a justified difference
before the first supported engine/tool offering. There is no silent compatibility
waiver from a source-only foundation review, the bundle-read fixture, a fake CDP
peer or a compiled tool schema. NoEngine/hidden and BROWSER_PROTOCOL=None remain.
