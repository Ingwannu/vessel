// What an HTML file's preview frame is given, Paseo's
// `file-panels/html-preview-csp.ts` (pinned 0.15.4): its own doctype, then
// a content policy, then the file as it is. The policy must reach the
// parser before any markup the file declares and land in `<head>`; finding
// the file's own doctype to insert after it would mean reimplementing the
// tokenizer's first insertion mode, so the prologue is supplied instead
// and the file's doctype becomes a stray token the parser ignores.
//
// The document is framed from `vessel ui`'s preview route (ADR 0122,
// TRACKER 814), whose header carries the same policy, sandboxed; there its
// scripts run as Paseo's do. When that route cannot be reached, the static
// `srcdoc` fallback takes the page's own policy and runs no script; the
// prologue keeps either frame from fetching anything.

const POLICY = [
  "default-src 'none'",
  "script-src 'unsafe-inline' 'unsafe-eval' blob:",
  "style-src 'unsafe-inline'",
  "img-src data: blob:",
  "font-src data:",
  "media-src data: blob:",
  "connect-src 'none'",
  "form-action 'none'",
  "base-uri 'none'",
  "frame-src 'none'",
  "object-src 'none'",
].join("; ");

const PROLOGUE = `<!doctype html><meta http-equiv="Content-Security-Policy" content="${POLICY}">`;

/** Left where it is, a BOM would sit mid-document as a zero-width space. */
const BOM = "\uFEFF";

/** `html` with Paseo's prologue in front. */
export function withPreviewCsp(html: string): string {
  return PROLOGUE + (html.startsWith(BOM) ? html.slice(BOM.length) : html);
}
