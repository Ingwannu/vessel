// Which built-in panel shows a file, Paseo's `file-panels/catalog.ts` and
// `resolve.ts` (pinned 0.15.4): Markdown and HTML by extension, with a
// Preview beside their source; images by extension; every other file
// its source (F-F38, F-F39, F-F40; docs/plans/wp-file-viewers.md).
//
// [Decision Log]
// - Purpose: the Files tab opens a file in Paseo's built-in panel for it.
// - Constraint: Paseo also resolves plugin panels; Vessel has none yet
//   (plugin work, area M), so only the built-ins and Source remain.
// - Choice: Paseo's catalog as data, matched as its resolver does: the
//   extension after the base name's last dot, lower case.
// - Cost: a file whose content does not match its extension (a `.png`
//   that is text) opens in the panel its name says, as in Paseo.

/** The built-in panels. */
export type FilePanel = "markdown" | "html" | "image" | "source";

/** A panel with a Preview beside its source. */
export type PreviewPanel = Extract<FilePanel, "markdown" | "html">;

const CATALOG: readonly {
  panel: Exclude<FilePanel, "source">;
  extensions: readonly string[];
}[] = [
  { panel: "markdown", extensions: ["md", "markdown"] },
  { panel: "html", extensions: ["html", "htm"] },
  { panel: "image", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] },
];

/** An image's media type, for its `data:` URL. */
const IMAGE_TYPES: Readonly<Record<string, string>> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  svg: "image/svg+xml",
};

/** The extension after the base name's last dot, lower case (Paseo's
 * `filePanelExtension`); empty when there is none. */
export function fileExtension(path: string): string {
  const name = path.trim().split(/[\\/]/u).pop() ?? "";
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "" : name.slice(dot + 1).toLowerCase();
}

/** The panel that shows `path`. */
export function filePanelOf(path: string): FilePanel {
  const extension = fileExtension(path);
  return (
    CATALOG.find((entry) => entry.extensions.includes(extension))?.panel ??
    "source"
  );
}

/** Whether a panel has a Preview beside its source. */
export function hasPreview(panel: FilePanel): panel is PreviewPanel {
  return panel === "markdown" || panel === "html";
}

/** An image file's media type; `null` for a file the image panel does
 * not claim. */
export function imageType(path: string): string | null {
  return IMAGE_TYPES[fileExtension(path)] ?? null;
}
