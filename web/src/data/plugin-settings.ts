// A plugin's settings documents on the page (ADR 0115, "Settings" and
// "Surfaces"): the calls, the values a bound control reads and writes, and
// the drafts an input keeps. Paseo's `plugins/settings/use-settings.ts`
// (16-85) and the patterns of public-docs/plugins/reference.md:1418-1427.
//
// [Decision Log]
// - Purpose: a settings screen's switch, select and input edit one top-level
//   property of a document the node keeps, and must not lose a person's
//   text when another client saves first.
// - Choice: Paseo's two patterns, fixed by the control kind (coordinator
//   ruling on P3b, Q2):
//   - a switch or select saves the whole document at once, with its new
//     value, at the revision last read;
//   - an input keeps a draft and the revision it began from; Enter saves at
//     that revision, Escape discards. A conflict keeps the draft.
// - Drafts live in a store keyed by machine, plugin, document and key, so a
//   catalog re-read or a screen re-render does not drop them (ADR 0115,
//   "Settings": clients keep a draft across re-reads).
// - The node checks the values with the plugin's own schema, so the page
//   only reads and replaces one property and sends the rest unchanged.

import type { NodeKey } from "../wire/bridge";
import type {
  PluginSettingsRead,
  PluginSettingsWrite,
  RawJson,
  SettingsRevision,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";
import { Store } from "./store";

/** One settings document of one installation on one machine. */
export interface SettingsAddress {
  node: NodeKey;
  plugin: string;
  settings: string;
}

/** Read a document. */
export function readSettings(
  fleet: Fleet,
  at: SettingsAddress,
): Promise<Outcome<PluginSettingsRead>> {
  return attempt(() =>
    fleet.bridge.call(at.node, "plugin.settings.read", {
      plugin: at.plugin,
      settings: at.settings,
    }),
  );
}

/** Save a whole document over `revision`. */
export function writeSettings(
  fleet: Fleet,
  at: SettingsAddress,
  revision: SettingsRevision,
  values: RawJson,
): Promise<Outcome<PluginSettingsWrite>> {
  return attempt(() =>
    fleet.bridge.call(at.node, "plugin.settings.write", {
      plugin: at.plugin,
      settings: at.settings,
      revision,
      values,
    }),
  );
}

/** Replace a document with its defaults over `revision`. */
export function resetSettings(
  fleet: Fleet,
  at: SettingsAddress,
  revision: SettingsRevision,
): Promise<Outcome<PluginSettingsWrite>> {
  return attempt(() =>
    fleet.bridge.call(at.node, "plugin.settings.reset", {
      plugin: at.plugin,
      settings: at.settings,
      revision,
    }),
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** A document's values as an object of properties; anything else has none. */
function properties(values: RawJson): Readonly<Record<string, unknown>> {
  return isRecord(values) ? values : {};
}

/** What a bound control finds at `key`: its value when it has the control's
 * type, or `wrong` (the row shows it and is disabled). */
export type Bound<T> = { ok: true; value: T } | { ok: false };

/** The boolean at `key`. */
export function boolAt(values: RawJson, key: string): Bound<boolean> {
  const value = properties(values)[key];
  return typeof value === "boolean" ? { ok: true, value } : { ok: false };
}

/** The string at `key`; a missing property reads as empty text. */
export function textAt(values: RawJson, key: string): Bound<string> {
  const value = properties(values)[key];
  if (value === undefined) return { ok: true, value: "" };
  return typeof value === "string" ? { ok: true, value } : { ok: false };
}

/** `values` with `key` set to `value`, every other property kept. */
export function withValue(
  values: RawJson,
  key: string,
  value: boolean | string,
): RawJson {
  return { ...properties(values), [key]: value };
}

/** The words a bound row shows when its value has another type. */
export const WRONG_TYPE = "The saved value has another type.";

/** An input's draft: its text and the revision it began from. */
export interface Draft {
  text: string;
  revision: SettingsRevision;
}

/** Every open draft, by `draftKey`. */
export const settingsDrafts = new Store<ReadonlyMap<string, Draft>>(new Map());

/** Every draft key of the document at `at` starts with this. */
function draftPrefix(at: SettingsAddress): string {
  return `${JSON.stringify([at.node, at.plugin, at.settings])}\u0000`;
}

/** The key of the draft for `key` of the document at `at`. */
export function draftKey(at: SettingsAddress, key: string): string {
  return `${draftPrefix(at)}${key}`;
}

/** Move the drafts of the document at `at` that `moves` picks to
 * `revision`, keeping their text. */
function moveDrafts(
  at: SettingsAddress,
  revision: SettingsRevision,
  moves: (draft: Draft) => boolean,
): void {
  const prefix = draftPrefix(at);
  const picked = (id: string, draft: Draft) =>
    id.startsWith(prefix) && draft.revision !== revision && moves(draft);
  settingsDrafts.update((all) => {
    if (![...all].some(([id, draft]) => picked(id, draft))) return all;
    return new Map(
      [...all].map(([id, draft]) =>
        picked(id, draft) ? [id, { ...draft, revision }] : [id, draft],
      ),
    );
  });
}

/** After a conflict and Reload: every draft of the document moves to the
 * revision just read, so Enter saves over those values (only the draft's
 * own property changes). */
export function rebaseDrafts(
  at: SettingsAddress,
  revision: SettingsRevision,
): void {
  moveDrafts(at, revision, () => true);
}

/** After this page saved the document from `from` to `to`: the drafts that
 * began from `from` follow it, so the page's own save is not taken for
 * another client's (node review R2). A draft from an older revision stays
 * a conflict. */
export function followSave(
  at: SettingsAddress,
  from: SettingsRevision,
  to: SettingsRevision,
): void {
  moveDrafts(at, to, (draft) => draft.revision === from);
}

/** Drop the drafts of `keys` in the document at `at` (a secret input's
 * text when its screen closes). */
export function dropDrafts(at: SettingsAddress, keys: readonly string[]): void {
  const ids = new Set(keys.map((key) => draftKey(at, key)));
  settingsDrafts.update((all) => {
    if (![...all.keys()].some((id) => ids.has(id))) return all;
    return new Map([...all].filter(([id]) => !ids.has(id)));
  });
}

/** Keep `draft` under `id`. */
export function keepDraft(id: string, draft: Draft): void {
  settingsDrafts.update((all) => new Map(all).set(id, draft));
}

/** Drop the draft under `id` (saved or discarded). */
export function dropDraft(id: string): void {
  settingsDrafts.update((all) => {
    if (!all.has(id)) return all;
    const next = new Map(all);
    next.delete(id);
    return next;
  });
}
