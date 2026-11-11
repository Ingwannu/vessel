// Which keyboard conventions the page follows: a Mac's (Command is the
// primary modifier, keys shown as glyphs) or everyone else's.

/** The two conventions shortcuts are shown and matched by. */
export type Platform = "mac" | "other";

/** `platform` and `userAgent` as a convention: Apple's desktops and
 * tablets with a keyboard are "mac". */
export function platformOf(platform: string, userAgent: string): Platform {
  return /Mac|iPhone|iPad|iPod/i.test(platform) ||
    /Macintosh|Mac OS X/i.test(userAgent)
    ? "mac"
    : "other";
}

/** This browser's convention. */
export function currentPlatform(): Platform {
  return typeof navigator === "undefined"
    ? "other"
    : platformOf(navigator.platform, navigator.userAgent);
}
