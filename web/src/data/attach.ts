// What may be attached to a prompt, decided before a byte is sent
// (milestone 11, Track A-page; ADR 0025): a file's kind by its type, as
// the node decides it; whether the agent takes that kind; whether it fits
// the node's limits; and the name the node will accept.
//
// [Decision Log]
// - Purpose: an attachment the node would refuse should be refused on the
//   page, at once and with its reason, not after it was uploaded.
// - Constraint: the node is the authority (it checks again at `begin` and
//   when the prompt is sent, and counts what the session already holds,
//   which the page does not know). The page's rules are the node's, read
//   from the same places: the kind from the type (`attachments` in the
//   node), the agent's `abilities`, `session.conversation`'s `limits`.
// - Choice: pure functions here, each tested; the tray (`tray.ts`) calls
//   them as files arrive. What only the node can know (the session's own
//   total, the node's 4 GiB) stays the node's: its refusal is shown on
//   the file's chip.

import type {
  AgentAbilities,
  AttachmentKind,
  AttachmentLimits,
} from "../wire/proto";

/** The four image types an agent is given inline (the node's list). */
const INLINE_IMAGES: ReadonlySet<string> = new Set([
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
]);

/** The longest type the node takes. */
const MIME_MAX = 127;

/** A browser's type for a file as the node takes it: lowercase, no
 * parameters, `application/octet-stream` when it is empty or not of the
 * node's shape. */
export function mimeOf(type: string): string {
  const bare = (type.split(";")[0] ?? "").trim().toLowerCase();
  return /^[a-z0-9!#$&^_.+-]+\/[a-z0-9!#$&^_.+-]+$/.test(bare) &&
    bare.length <= MIME_MAX
    ? bare
    : "application/octet-stream";
}

/** The kind the node gives a file of type `mime`. */
export function kindOf(mime: string): AttachmentKind {
  if (INLINE_IMAGES.has(mime)) {
    return "image";
  }
  return mime.startsWith("audio/") ? "audio" : "file";
}

/** Windows opens a device for these stems, whatever the extension. */
const DEVICES = /^(con|prn|aux|nul|conin\$|conout\$|com[0-9¹²³]|lpt[0-9¹²³])$/i;

/** The longest name, in UTF-8 bytes. */
const NAME_MAX = 255;

/** Characters no file name may hold on some platform the node runs on. */
const FORBIDDEN = new Set(["/", "\\", ":", "*", "?", '"', "<", ">", "|"]);

/** `name` as the node accepts a file's name (`AttachmentName`): one path
 * component; no control character nor any of `/ \ : * ? " < > |`; not
 * starting with a space or a dot (`.env` becomes `_.env`, keeping the
 * name and its extension), not ending with one; not a Windows device; at
 * most 255 bytes, cut on a character before the extension. `fallback`
 * when nothing is left. */
export function attachmentName(name: string, fallback: string): string {
  let cleaned = "";
  for (const character of name) {
    const code = character.codePointAt(0) ?? 0;
    const control = code < 0x20 || (code >= 0x7f && code < 0xa0);
    cleaned += control || FORBIDDEN.has(character) ? "_" : character;
  }
  cleaned = cleaned.replace(/[\s.]+$/, "");
  if (/^[\s.]/.test(cleaned)) {
    cleaned = `_${cleaned}`;
  }
  const stem = (cleaned.split(".")[0] ?? "").trimEnd();
  if (DEVICES.test(stem)) {
    cleaned = `_${cleaned}`;
  }
  const result = fitted(cleaned);
  return result === "" ? fallback : result;
}

/** `name` cut to `NAME_MAX` bytes, keeping its extension where it has a
 * short one. */
function fitted(name: string): string {
  const encoder = new TextEncoder();
  if (encoder.encode(name).length <= NAME_MAX) {
    return name;
  }
  const dot = name.lastIndexOf(".");
  const extension = dot > 0 && name.length - dot <= 16 ? name.slice(dot) : "";
  let stem = Array.from(extension === "" ? name : name.slice(0, dot));
  while (
    stem.length > 0 &&
    encoder.encode(stem.join("") + extension).length > NAME_MAX
  ) {
    stem = stem.slice(0, -1);
  }
  return (stem.join("") + extension).replace(/[\s.]+$/, "");
}

/** The first bytes each inline image type starts with (the node checks
 * the same before it keeps one). */
export function looksLike(mime: string, head: Uint8Array): boolean {
  const starts = (bytes: readonly number[], at = 0) =>
    bytes.every((byte, index) => head[at + index] === byte);
  switch (mime) {
    case "image/png":
      return starts([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    case "image/jpeg":
      return starts([0xff, 0xd8, 0xff]);
    case "image/gif":
      return starts([0x47, 0x49, 0x46, 0x38]);
    case "image/webp":
      return (
        starts([0x52, 0x49, 0x46, 0x46]) && starts([0x57, 0x45, 0x42, 0x50], 8)
      );
    default:
      return true;
  }
}

/** A file waiting to be checked. */
export interface Candidate {
  kind: AttachmentKind;
  size: number;
}

/** Why a file is not attached, before anything is sent. */
export type Refusal =
  | { kind: "agent"; what: AttachmentKind }
  | { kind: "too-large"; limit: number }
  | { kind: "empty" }
  | { kind: "too-many"; limit: number }
  | { kind: "inline-total"; limit: number };

/** Whether `candidate` may join `held` (the files already in the tray,
 * not refused) under the agent's `abilities` and the node's `limits`.
 * `null` when it may. */
export function refusal(
  candidate: Candidate,
  held: readonly Candidate[],
  abilities: AgentAbilities | null,
  limits: AttachmentLimits,
): Refusal | null {
  if (!takes(abilities, candidate.kind)) {
    return { kind: "agent", what: candidate.kind };
  }
  if (candidate.size === 0) {
    return { kind: "empty" };
  }
  const limit = limitOf(limits, candidate.kind);
  if (candidate.size > limit) {
    return { kind: "too-large", limit };
  }
  if (held.length + 1 > limits.per_prompt) {
    return { kind: "too-many", limit: limits.per_prompt };
  }
  if (candidate.kind !== "file") {
    const inline = held
      .filter((other) => other.kind !== "file")
      .reduce((sum, other) => sum + other.size, candidate.size);
    if (inline > limits.inline_per_prompt) {
      return { kind: "inline-total", limit: limits.inline_per_prompt };
    }
  }
  return null;
}

/** Whether the agent takes `kind`: files always (a link to a stored
 * file, ACP's baseline), images and audio when its handshake says. */
export function takes(
  abilities: AgentAbilities | null,
  kind: AttachmentKind,
): boolean {
  switch (kind) {
    case "file":
      return true;
    case "image":
      return abilities?.image === true;
    case "audio":
      return abilities?.audio === true;
  }
}

function limitOf(limits: AttachmentLimits, kind: AttachmentKind): number {
  switch (kind) {
    case "image":
      return limits.image;
    case "audio":
      return limits.audio;
    case "file":
      return limits.file;
  }
}

/** A size as a chip says it: "512 B", "48 KB", "3.2 MB". */
export function sizeLabel(bytes: number): string {
  if (bytes < 1024) {
    return `${String(bytes)} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${String(Math.round(bytes / 1024))} KB`;
  }
  const mib = bytes / (1024 * 1024);
  return `${mib < 10 ? mib.toFixed(1) : String(Math.round(mib))} MB`;
}

/** A refusal in the chip's words. */
export function refusalLine(refused: Refusal, agent: string): string {
  switch (refused.kind) {
    case "agent":
      return refused.what === "image"
        ? `${agent} does not take images`
        : `${agent} does not take audio`;
    case "empty":
      return "The file is empty";
    case "too-large":
      return `Larger than ${sizeLabel(refused.limit)}`;
    case "too-many":
      return `At most ${String(refused.limit)} files in a message`;
    case "inline-total":
      return `Images and audio over ${sizeLabel(refused.limit)} in one message`;
  }
}
