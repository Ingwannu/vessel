// The images of sent prompts, as the timeline shows them (milestone 11,
// Track A-page): fetched with `attachment.read` only when their row is
// near the view, kept within a bound, shown from a small thumbnail, and,
// for a file sent from this tab, taken from the local file instead.
//
// [Decision Log]
// - Purpose: a prompt that carried an image shows the image (all three
//   references do), and another device shows what was sent from this one.
// - Constraint: the timeline is windowed (only rows near the view are
//   drawn) and a session can carry 100 MB of attachments; fetching every
//   image of a long session when it opens would read all of them. The
//   page's content policy takes `data:` images and no `blob:` ones
//   (ADR 0017), so an image is drawn from a `data:` address.
// - Choice: a fetch per image when its row asks (the row asks when it is
//   within a screen of the view), two at a time, one fetch per image
//   however many rows ask; the bytes kept as a `Blob` and drawn as a
//   thumbnail of the tile's size (data/thumbnail.ts), the full image read
//   into a `data:` address only when it is opened; at most 48 images or
//   64 MiB of bytes kept, the least recently asked dropped. An image
//   attached in this tab is seeded from its local file (`Tray.sent`), so
//   it is never fetched back.
// - Cost: an image scrolled far away and back may be fetched again; a
//   thumbnail is drawn on the main thread (one canvas, at 128px tall).

import type { NodeKey } from "../wire/bridge";
import type { AttachmentId, SessionId } from "../wire/proto";
import { readAttachment } from "./conversation-calls";
import type { Fleet } from "./fleet";
import { thumbnail } from "./thumbnail";

/** The most images kept, and their bytes. */
const MOST = 48;
const MOST_BYTES = 64 * 1024 * 1024;

/** A tile is 64px tall; its thumbnail is drawn at twice that. */
export const THUMB_HEIGHT = 128;

/** An image as the timeline holds it. */
export interface KeptImage {
  /** The bytes, for the full-size view. */
  blob: Blob;
  /** A small `data:` picture for the tile; `null` when it could not be
   * drawn (the tile then says so). */
  thumb: string | null;
}

const kept = new Map<string, KeptImage>();
const loading = new Map<string, Promise<KeptImage | null>>();
let bytes = 0;

/** Images read at once: the others wait their turn, so a fast scroll
 * through a session of images does not ask for all of them together. */
const AT_ONCE = 2;
let reading = 0;
const turns: (() => void)[] = [];

async function inTurn<T>(work: () => Promise<T>): Promise<T> {
  if (reading >= AT_ONCE) {
    await new Promise<void>((resolve) => turns.push(resolve));
  }
  reading += 1;
  try {
    return await work();
  } finally {
    reading -= 1;
    turns.shift()?.();
  }
}

function keyOf(node: NodeKey, session: SessionId, id: AttachmentId): string {
  return `${node}\u0000${session}\u0000${id}`;
}

function keep(key: string, image: KeptImage): void {
  const old = kept.get(key);
  if (old !== undefined) {
    kept.delete(key);
    bytes -= old.blob.size;
  }
  kept.set(key, image);
  bytes += image.blob.size;
  while (kept.size > MOST || (bytes > MOST_BYTES && kept.size > 1)) {
    const [oldest] = kept.keys();
    if (oldest === undefined) {
      break;
    }
    const dropped = kept.get(oldest);
    kept.delete(oldest);
    if (dropped !== undefined) {
      bytes -= dropped.blob.size;
    }
  }
}

/** Show an image sent from this tab from its local file, under the id
 * the node gave it (its thumbnail is drawn now, so the row that asks
 * for it waits on the drawing, not on the machine). */
export function seedImage(
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
  blob: Blob,
): void {
  const key = keyOf(node, session, id);
  const work = thumbnail(blob, THUMB_HEIGHT).then((thumb) => {
    loading.delete(key);
    const image = { blob, thumb };
    keep(key, image);
    return image;
  });
  loading.set(key, work);
}

/** The image when it is kept, marking it recently used. */
export function keptImage(
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
): KeptImage | null {
  const key = keyOf(node, session, id);
  const found = kept.get(key);
  if (found === undefined) {
    return null;
  }
  kept.delete(key);
  kept.set(key, found);
  return found;
}

/** The pieces of an image larger than this are not read (the node's
 * image bound is 10 MiB; a larger answer is not an image to show). */
const IMAGE_MAX = 16 * 1024 * 1024;

/** The image, fetched from the machine if it is not kept; `null` when it
 * could not be read. */
export function loadImage(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  id: AttachmentId,
  mime: string,
): Promise<KeptImage | null> {
  const found = keptImage(node, session, id);
  if (found !== null) {
    return Promise.resolve(found);
  }
  const key = keyOf(node, session, id);
  const running = loading.get(key);
  if (running !== undefined) {
    return running;
  }
  const work = inTurn(() => fetchImage(fleet, node, session, id, mime)).then(
    async (blob) => {
      loading.delete(key);
      if (blob === null) {
        return null;
      }
      const image = { blob, thumb: await thumbnail(blob, THUMB_HEIGHT) };
      keep(key, image);
      return image;
    },
  );
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
    if (!read.ok || read.value.total > IMAGE_MAX) {
      return null;
    }
    const piece = bytesOf(read.value.data);
    if (piece.length > 0) {
      parts.push(piece);
      offset += piece.length;
    }
    if (offset >= read.value.total) {
      return new Blob(parts, { type: mime });
    }
    if (piece.length === 0) {
      return null;
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
