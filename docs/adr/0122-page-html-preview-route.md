# ADR 0122: Page HTML preview route

Status: proposed (impl F, 2026-11-20; TRACKER 814). The coordinator chose
option A and recorded 0122 as this ADR's number. It amends ADR 0017's
headers for one route.

## Context

Paseo previews an HTML file in a frame whose scripts run
(`file-panels/html-preview.web.tsx`, `html-preview-csp.ts`, pinned 0.15.4):

- a `srcdoc` frame, `sandbox="allow-scripts"`, `referrerPolicy="no-referrer"`;
- the document is Paseo's prologue, a content policy and then the file:
  - inline script and style run;
  - images, fonts and media come from `data:`/`blob:` only;
  - no fetch, sockets, forms, frames or plugins;
  - `base-uri 'none'`.

The frame's origin is opaque: its scripts cannot reach the app's DOM,
cookies or storage, cannot navigate the top window and get no popups. Paseo
states one hole: a document may navigate itself (meta refresh, `location`),
and no content policy directive stops that.

The page's viewers shipped a static preview instead (the coordinator's
ruling 1 of `docs/plans/wp-file-viewers.md`, 2026-11-19). A `srcdoc`, `blob:`
or `data:` frame takes the policy of the page that writes it. The page's
policy (ADR 0017, `crates/vessel-web/src/guard.rs`) allows script from
`'self'` and the import map's hash only, so the file's inline scripts were
refused. That policy is not to be loosened. A document with a policy of its
own must come over the network, from an origin other than the page's.

## Options

- A: a route on the same `vessel ui` server whose answer is sandboxed by
  its own header. The frame's origin is opaque, separate from the page's
  for DOM, storage, script-visible cookies and `Origin`. Nothing new is
  deployed: the same port and host work behind a proxy.
- B: a second listener on another port. Another origin, but the same site,
  since ports do not change a site. It isolates no more than A, and needs a
  second Host/Origin allowlist and, behind a proxy, a second host name.
- C: another site (a host name of its own). It adds a browser process and
  site boundary, and keeps the page's site cookies off its navigations. It
  needs DNS, TLS and proxy configuration per deployment (`*.localhost`
  covers local use only).

The isolation Paseo relies on, an opaque origin, is the same in all three.

## Decision

Option A.

- `POST /api/preview`:
  - Takes the tab's session key as a bearer, as `/api/status` does, and the
    guard's same-origin checks apply.
  - `X-Vessel-Preview-Slot` names the panel: 1 to 64 of `[A-Za-z0-9_-]`,
    the tab's random prefix and React's id. The body is the document the
    page would frame (Paseo's prologue and the file), UTF-8, at most 64 MiB.
  - The answer is `{"url": "/preview/<id>"}`, where `<id>` is 128 random
    bits in hex.
- The page posts only while its websocket is open and welcomed, so the key
  goes only to a server that has just proved itself (ADR 0017's
  handshake).
- Documents are kept in memory for the launch:
  - a panel's new document replaces its last one;
  - at most 8 documents and 64 MiB, the least recently loaded dropped
    first;
  - each is dropped 10 minutes after it was last loaded or posted.
  Nothing is written to disk.
- `GET /preview/<id>` needs the id alone: a frame's load can carry no
  header. It answers the document with:
  - `Content-Type: text/html; charset=utf-8`;
  - `Content-Security-Policy`: Paseo's policy, then
    `sandbox allow-scripts; frame-ancestors 'self'`;
  - `X-Frame-Options: SAMEORIGIN` and `Cache-Control: no-store`;
  - and, as on every answer, `X-Content-Type-Options: nosniff`,
    `Referrer-Policy: no-referrer` and the cross-origin opener and
    resource policies;
  - no cookie.
  An unknown or expired id is a 404 under the page's own policy.
- The guard adds the page's policy and `X-Frame-Options: DENY` to every
  answer except a preview document (`preview::PreviewDocument`), which
  keeps its own. The other headers are added to it as to all answers.
- The page frames the address with `sandbox="allow-scripts"` and
  `referrerPolicy="no-referrer"`, says "Loading file..." until the address
  comes, and falls back to the static `srcdoc` frame without scripts if
  the post fails.

## Why

- The sandbox is in the answer's own header, so the document's origin is
  opaque wherever it is opened. A top-level visit to `/preview/<id>` gets no
  storage and no cookies. Chromium may also refuse to open it at the top
  level at all: the answer's cross-origin opener policy with a sandbox is a
  network error there.
- From the frame:
  - the page's DOM, its `sessionStorage` (the session key) and its cookies
    are another origin's;
  - `connect-src 'none'` refuses fetch, XHR and sockets, and a websocket
    would carry `Origin: null`, which the guard refuses;
  - `/api/*` needs the bearer;
  - the page itself answers `frame-ancestors 'none'`, so it cannot be
    loaded inside the frame;
  - `form-action 'none'`, no popups, no top navigation.
- What remains is Paseo's own hole: the document can navigate itself with
  a GET, carrying what it already holds (the file).

## Consequences

- The page's policy is unchanged; the preview route is the one answer with
  another.
- `vessel ui` holds up to 64 MiB of preview documents in memory.
- The page now makes one HTTP call besides its websocket (ADR 0017 said it
  made none); it carries the session key only while the socket is
  welcomed.
- The native app is unchanged: HTML stays Source only until ADR 0020.
- Tests:
  - the store's caps, replacement and expiry;
  - the route's headers over real sockets;
  - the bearer and origin checks, and the 404 under the page's policy;
  - the page's post;
  - e2e: the script runs; parent, storage, cookie and network are out of
    reach; the route opened on its own is sandboxed or refused.
