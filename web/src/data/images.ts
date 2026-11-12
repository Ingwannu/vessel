// The images of sent prompts, as the timeline shows them (milestone 11,
// Track A-page): fetched with `attachment.read` only when their row is
// near the view, kept as object URLs within a bound, and, for a file sent
// from this tab, shown from the local file instead.
//
// [Decision Log]
// - Purpose: a prompt that carried an image shows the image (all three
//   references do), and another device shows what was sent from this one.
// - Constraint: the timeline is windowed (only rows near the view are
//   drawn) and a session can carry 100 MB of attachments; fetching every
//   image of a long session when it opens would read all of them.
// - Choice: a fetch per image when its row asks (the row asks when it is
//   within a screen of the view), joined pieces into one `Blob`, one fetch
//   at a time per image however many rows ask; at most 48 images or
//   64 MiB kept, the least recently asked dropped and its URL revoked.
//   An image attached in this tab is seeded from its local file
//   (`Tray.sent`), so it is never fetched back.
// - Cost: an image scrolled far away and back may be fetched again.

import type { NodeKey } from "../wire/bridge";
import type { AttachmentId, SessionId } from "../wire/proto";
import { readAttachment } from "./conversation-calls";
import type { Fleet } from "./fleet";

/** The most images kept, and their bytes. */
const MOST = 48;
const MOST_BYTES = 64 * 1024 * 1024;

interface Kept {
  url: string;
  bytes: number;
  /** Whether the URL is the page's to revoke (a seeded one is the
   * tray's, which handed it over: it is). */
  revoke: boolean;
}

const kept = new Map<string, Kept>();
const loading = new Map<string, Promise<string | null>>();
let bytes = 0;

function keyOf(node: NodeKey, session: SessionId, id: AttachmentId): string {
  return `${node}\u0000${session}\u0000${id}`;
}

function keep(key: string, entry: Kept): void {
  const old = kept.get(key);
  if (old !== undefined) {
    kept.delete(key);
    bytes -= old.bytes;
  }
  kept.set(key, entry);
  bytes += entry.bytes;
  while (kept.size > MOST || (bytes > MOST_BYTES && kept.size > 1)) {
    const [oldest] = kept.keys();
    if (oldest === undefined) {
      break;
    }
    const dropped = kept.get(oldest);
    kept.delete(oldest);
    if (dropped !== undefined) {
      bytes -= dropped.bytes;
      if (dropped.revoke) {
        URL.revokeObjectURL(dropped.url);
      }
    }
  }
}

/** Show an image sent from this tab from its local file, under the id
 * the node gave it. */
export function seedImage(
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
  url: string,
  size: number,
): void {
  keep(keyOf(node, session, id), { url, bytes: size, revoke: true });
}

/** The image's URL when it is kept, marking it recently used. */
export function keptImage(
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
): string | null {
  const key = keyOf(node, session, id);
  const found = kept.get(key);
  if (found === undefined) {
    return null;
  }
  kept.delete(key);
  kept.set(key, found);
  return found.url;
}

/** The pieces of an image larger than this are not read (the node's
 * image bound is 10 MiB; a larger answer is not an image to show). */
const IMAGE_MAX = 16 * 1024 * 1024;

/** The image's URL, fetched from the machine if it is not kept. `null`
 * when it could not be read. */
export function loadImage(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
  mime: string,
): Promise<string | null> {
  const found = keptImage(node, session, id);
  if (found !== null) {
    return Promise.resolve(found);
  }
  const key = keyOf(node, session, id);
  const running = loading.get(key);
  if (running !== undefined) {
    return running;
  }
  const work = fetchImage(fleet, node, session, id, mime).then((blob) => {
    loading.delete(key);
    if (blob === null) {
      return null;
    }
    const url = URL.createObjectURL(blob);
    keep(key, { url, bytes: blob.size, revoke: true });
    return url;
  });
  loading.set(key, work);
  return work;
}

async function fetchImage(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
  mime: string,
): Promise<Blob | null> {
  const parts: Uint8Array<ArrayBuffer>[] = [];
  let offset = 0;
  for (;;) {
    const read = await readAttachment(fleet, node, session, id, offset);
    if (!read.ok) {
      return null;
    }
    const piece = bytesOf(read.value.data);
    if (piece.length === 0 || read.value.total > IMAGE_MAX) {
      return offset >= read.value.total
        ? new Blob(parts, { type: mime })
        : null;
    }
    parts.push(piece);
    offset += piece.length;
    if (offset >= read.value.total) {
      return new Blob(parts, { type: mime });
    }
  }
}

/** Base64 `text` as bytes. */
function bytesOf(text: string): Uint8Array<ArrayBuffer> {
  const binary = atob(text);
  const out = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    out[index] = binary.charCodeAt(index);
  }
  return out;
}
