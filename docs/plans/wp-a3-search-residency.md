# Window-owned search prerequisite

This isolated prerequisite is based on shared app line 074ade16. It contains
none of the A3 Messages UI, preferences, find changes, prompt-index types or RPC.

## Decision log

- Purpose: new conversation residents must preserve the pending global query.
- Existing assets: ConversationArea already contains Searching, deliberately
  retained across clear/session switches; PaletteArea is window-owned. Existing
  Shell search methods and the node adapter suffice; no external API is needed.
- Alternatives: copying Searching into each resident duplicates pending work;
  rebuilding at selection loses query/ticket correlation.
- Choice: move the existing value into PaletteArea.messages_search and redirect
  existing access paths. Keep the existing renderer, dispatch and link.
- Effects: one controller per window. A GPUI shell regression replaces the full
  ConversationArea before debounce and between machine replies, requiring the
  same query, ticket and merged rows to survive.

Narrow test/clippy and review status are reported for the exact commit; this note
does not imply their success. Broader A3 behaviour stays in its separate package.
