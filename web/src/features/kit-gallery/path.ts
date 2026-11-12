// Whether the page asked for is the kit's gallery (gallery.tsx), kept
// apart from it so main.tsx decides without loading the gallery's chunk.

/** Whether `path` is the gallery's. */
export function isGallery(path: string): boolean {
  return path === "/kit" || path === "/kit/";
}
