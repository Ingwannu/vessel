// A workspace file's exact bytes, every page of `file.read_bytes` read in
// order, each after the first pinned to the first page's revision (a file
// changed mid-way is refused, never mixed). The Explorer's Download and the
// image panel (F-F40) read through it.
//
// The image panel shows the bytes as a `data:` URL: the page's own policy
// allows `data:` images and not `blob:` ones (plan difference 3), and the
// node's `file.read_bytes` caps a file at 64 MiB.

import type { NodeKey } from "../../wire/bridge";
import type { FileRevision, RelPath, WorkspaceRoot } from "../../wire/proto";
import type { Fleet } from "../fleet";
import type { Outcome } from "../outcome";
import { readBytes } from "./files";

/** A page's base64 as bytes. */
export function decodedPage(base64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

/** The file's bytes, page by page, and the revision they are of. */
export interface FileBytes {
  pages: Uint8Array<ArrayBuffer>[];
  revision: FileRevision | null;
}

/** Every page of `path`. */
export async function readFileBytes(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
): Promise<Outcome<FileBytes>> {
  const pages: Uint8Array<ArrayBuffer>[] = [];
  let offset = 0;
  let revision: FileRevision | null = null;
  for (;;) {
    const page = await readBytes(fleet, node, root, path, offset, revision);
    if (!page.ok) return page;
    pages.push(decodedPage(page.value.bytes));
    revision = page.value.revision;
    if (page.value.next === null) {
      return { ok: true, value: { pages, revision } };
    }
    offset = page.value.next;
  }
}

/** Bytes per `String.fromCharCode` call: well under engines' argument
 * limits. */
const CHUNK = 0x8000;

/** `pages` as a `data:` URL of `type`. */
export function dataUrl(
  pages: readonly Uint8Array<ArrayBuffer>[],
  type: string,
): string {
  let binary = "";
  for (const page of pages) {
    for (let start = 0; start < page.length; start += CHUNK) {
      binary += String.fromCharCode(...page.subarray(start, start + CHUNK));
    }
  }
  return `data:${type};base64,${btoa(binary)}`;
}
