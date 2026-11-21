// An activity card's menu: Paseo's `ActivityKebab`
// (`pull-request-panel/pane.tsx`, pinned 0.15.4) on a comment or a review,
// and the thread header's own (Open alone): what each offers, in Paseo's
// order, and the forge's name its "Open on" row says. Mirrors the native
// `view/src/workspace/pull/card_menu.rs`.
//
// [Decision Log]
// - Purpose: Copy and Open on GitHub, which a card's own press does not
//   give (a press folds a card with words), as Paseo's menu does.
// - Choice: the rule is here, pure and tested; the page draws the kit's
//   menu from it. Add to chat while the session has a composer and the
//   item something to add (`canAdd`, Paseo's
//   `canAddPullRequestActivityToChat`), Copy while it has words, Open on
//   the forge.
// - Copy puts the item's markdown source on the clipboard as it came,
//   untrimmed (the plain-text flavour of Paseo's
//   `writeMarkdownToRichClipboard`, pane.tsx 692-694).
// - Differences: no HTML flavour beside the text; Paseo always lists
//   Open, but an item the node sends without a page has no Open row here
//   (the row would do nothing), and a menu with no row draws no kebab.

import type { RepoRef, TimelineItem } from "../../../wire/proto";
import { canAdd } from "./attach";
import { trimmed } from "./timeline";

/** One row of a card's menu: add the item to the session's composer,
 * copy its words, open its page on the forge. */
export type CardAct = "addToChat" | "copy" | "open";

/** What Copy puts on the clipboard: the item's markdown source verbatim,
 * when it has words (Paseo offers Copy while `body.trim()` is not empty,
 * and copies `body` itself); null without words. */
export function copied(item: TimelineItem): string | null {
  return trimmed(item.body) === "" ? null : item.body;
}

/** Whether an item has a page on the forge. */
function hasPage(item: TimelineItem): boolean {
  return item.url !== undefined && item.url !== null;
}

/** What a comment's or a review's menu offers; `attach`: the session has
 * a composer to add to (Paseo's `attachEnabled`). */
export function itemActs(item: TimelineItem, attach: boolean): CardAct[] {
  const acts: CardAct[] = [];
  if (attach && canAdd(item)) {
    acts.push("addToChat");
  }
  if (copied(item) !== null) {
    acts.push("copy");
  }
  if (hasPage(item)) {
    acts.push("open");
  }
  return acts;
}

/** What a thread header's menu offers: Open, on its first comment's page
 * (`first`, absent for a thread with none). */
export function threadActs(first: TimelineItem | null | undefined): CardAct[] {
  return first !== undefined && first !== null && hasPage(first)
    ? ["open"]
    : [];
}

/** The forge's name for "Open on …" (Paseo's `brandLabel`): GitHub, or
 * the host for a kind the page does not know. */
export function brand(repo: RepoRef): string {
  switch (repo.kind) {
    case "github":
      return "GitHub";
    case "unknown":
      return repo.host;
  }
}
