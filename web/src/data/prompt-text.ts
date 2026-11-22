const UTF8 = new TextEncoder();

/** Shared wire limits. Each feature decides whether blank text is useful. */
export function promptTextError(prompt: string): string | null {
  if (UTF8.encode(prompt).length > 16_384)
    return "Prompt must be 16 KiB or fewer.";
  return /\p{Cc}/u.test(prompt.replace(/[\t\n\r]/g, ""))
    ? "Prompt contains an unsupported control character."
    : null;
}
