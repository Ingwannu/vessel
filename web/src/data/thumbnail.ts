// Pictures of images the page holds as bytes (a file attached here, an
// attachment read back from the machine), within the page's content
// policy: `img-src 'self' data:` (ADR 0017) takes no `blob:` address, so
// an image is drawn from a `data:` address. A thumbnail is made small (a
// canvas at twice the height it is shown at), so a session of large
// images costs a few kilobytes of text each; the image at full size is
// read only when a person opens it.

/** A `data:` address for a thumbnail of `blob`, `height` pixels tall
 * (at most `height * 4` wide), or `null` when it cannot be drawn (not an
 * image, or no canvas, as in the unit tests). */
export async function thumbnail(
  blob: Blob,
  height: number,
): Promise<string | null> {
  if (typeof createImageBitmap !== "function") {
    return null;
  }
  try {
    const bitmap = await createImageBitmap(blob);
    const scale = Math.min(1, height / bitmap.height);
    const tall = Math.max(1, Math.round(bitmap.height * scale));
    const wide = Math.max(
      1,
      Math.min(height * 4, Math.round(bitmap.width * scale)),
    );
    const canvas = document.createElement("canvas");
    canvas.width = wide;
    canvas.height = tall;
    const context = canvas.getContext("2d");
    if (context === null) {
      bitmap.close();
      return null;
    }
    context.drawImage(bitmap, 0, 0, wide, tall);
    bitmap.close();
    return canvas.toDataURL("image/png");
  } catch {
    return null;
  }
}

/** `blob` as a `data:` address, whole (the full-size view). */
export function dataAddress(blob: Blob): Promise<string | null> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => {
      resolve(typeof reader.result === "string" ? reader.result : null);
    };
    reader.onerror = () => {
      resolve(null);
    };
    reader.readAsDataURL(blob);
  });
}
