// Download a workspace file (Paseo's explorer Download, its
// `useFileDownload`): the node pages the file's exact bytes
// (`file.read_bytes`), the page joins them and the browser saves them
// under the file's name.
//
// [Decision Log]
// - Purpose: the Explorer's Download (TRACKER: the page Explorer menu).
// - Constraint: the page reaches the node only through the bridge; it has
//   no download URL to hand the browser (Paseo's web has a token route).
// - Choice: the pages in order, each after the first pinned to the first
//   page's revision (a file changed mid-way is refused, never mixed),
//   into one Blob, saved through an `a download` link.
// - Cost: the whole file is held in the page's memory before the browser
//   writes it, as the native app's sink holds a page at a time.

import type { Fleet } from "../../data/fleet";
import type { Outcome } from "../../data/outcome";
import { readBytes } from "../../data/workspace/files";
import type { NodeKey } from "../../wire/bridge";
import type { FileRevision, RelPath, WorkspaceRoot } from "../../wire/proto";

function decoded(base64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

/** The file's bytes, every page read. */
async function fileBytes(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
): Promise<Outcome<Uint8Array<ArrayBuffer>[]>> {
  const pages: Uint8Array<ArrayBuffer>[] = [];
  let offset = 0;
  let revision: FileRevision | null = null;
  for (;;) {
    const page = await readBytes(fleet, node, root, path, offset, revision);
    if (!page.ok) return page;
    pages.push(decoded(page.value.bytes));
    revision = page.value.revision;
    if (page.value.next === null) return { ok: true, value: pages };
    offset = page.value.next;
  }
}

/** Save `path` as `name` through the browser; a refusal comes back. */
export async function downloadFile(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  name: string,
): Promise<Outcome<null>> {
  const bytes = await fileBytes(fleet, node, root, path);
  if (!bytes.ok) return bytes;
  const url = URL.createObjectURL(new Blob(bytes.value));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  // The browser has taken the file once the click is handled.
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 0);
  return { ok: true, value: null };
}
