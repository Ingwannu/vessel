// The activity's words: a card's verb and its age. Mirrors the native
// `view/src/workspace/pull/timeline_words.rs`; Paseo writes ages in
// English only (`data.ts`'s `formatAge`), here each step is a message in
// the person's language.

import type { TimelineItem } from "../../../wire/proto";
import type { Age } from "./timeline";
import type { PullWords } from "./words";

/** "just now", "3m ago", "2h ago", "5d ago", "2mo ago", "1y ago". */
export function ageWords(age: Age, words: PullWords): string {
  if (age.step === "now") {
    return words.ageNow;
  }
  switch (age.step) {
    case "minutes":
      return words.ageMinutes(age.count);
    case "hours":
      return words.ageHours(age.count);
    case "days":
      return words.ageDays(age.count);
    case "months":
      return words.ageMonths(age.count);
    case "years":
      return words.ageYears(age.count);
  }
}

/** What an item's author did (Paseo's `ActivityVerb`, lowercase). */
export function verbWords(item: TimelineItem, words: PullWords): string {
  if (item.kind === "comment") {
    return words.verbCommented;
  }
  switch (item.state) {
    case "approved":
      return words.verbApproved;
    case "changes_requested":
      return words.verbChangesRequested;
    case "commented":
    case "unknown":
      return words.verbReviewed;
  }
}
