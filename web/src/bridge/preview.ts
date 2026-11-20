// An HTML file's Preview as a document of its own (ADR 0122, TRACKER 814).
// The page's own policy refuses inline script in any frame it writes
// (`srcdoc`, `blob:`, `data:` take that policy), so the document is posted
// to `vessel ui` and framed from the address it answers, where it carries
// Paseo's preview policy, sandboxed. The post carries the tab's session
// key, as `/api/status` takes it; the address is the document's only key.

/** The address `vessel ui` answers: `/preview/` and 128 bits in hex. */
const ADDRESS = /^\/preview\/[0-9a-f]{32}$/;

/** `fetch`, replaceable in tests. */
export type Fetch = (input: string, init: RequestInit) => Promise<Response>;

/** Post `document` for panel `slot`; the address to frame it from. */
export async function postPreview(
  key: string,
  slot: string,
  document: string,
  send: Fetch = (input, init) => fetch(input, init),
): Promise<string> {
  const response = await send("/api/preview", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${key}`,
      "Content-Type": "text/html; charset=utf-8",
      "X-Vessel-Preview-Slot": slot,
    },
    body: document,
    cache: "no-store",
    credentials: "same-origin",
    referrerPolicy: "no-referrer",
  });
  if (!response.ok) {
    throw new Error(`vessel ui kept no preview (${String(response.status)}).`);
  }
  const answer: unknown = await response.json();
  if (
    typeof answer === "object" &&
    answer !== null &&
    "url" in answer &&
    typeof answer.url === "string" &&
    ADDRESS.test(answer.url)
  ) {
    return answer.url;
  }
  throw new Error("vessel ui answered no preview address.");
}
