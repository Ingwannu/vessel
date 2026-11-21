// The context pills' words, in English and Korean (TRACKER 833): Paseo's
// (attachment-pill-content.tsx 30-45; composer/attachments/workspace.tsx
// 72-86; i18n/resources/{en,ko}.ts `composer.attachments.removeFile`).
// Paseo writes the pull request items' words in English only; the Korean
// beside them is ours, in the pull request pane's terms
// (data/workspace/pull/words.ts).

import type { ContextPill } from "./context-tray";

const en = {
  strip: "Attachments",
  comment: "Comment",
  review: "Review",
  check: "Check logs",
  removeContext: "Remove context attachment",
  removeFile: "Remove file attachment",
};

const ko: typeof en = {
  strip: "첨부",
  comment: "댓글",
  review: "리뷰",
  check: "체크 로그",
  removeContext: "컨텍스트 첨부 제거",
  removeFile: "파일 첨부 제거",
};

export type ContextWords = typeof en;

/** The pills' words for `language` (a BCP 47 tag): Korean for `ko` and
 * its regions, English otherwise. */
export function contextWords(language: string): ContextWords {
  return /^ko(?:-|$)/i.test(language) ? ko : en;
}

/** A pill's second line: a file's path, or its kind's word. */
export function pillLine(pill: ContextPill, words: ContextWords): string {
  switch (pill.kind) {
    case "comment":
      return words.comment;
    case "review":
      return words.review;
    case "check":
      return words.check;
    case "file":
      return pill.detail ?? "";
  }
}

/** A pill's remove button's name. */
export function removeLabel(pill: ContextPill, words: ContextWords): string {
  return pill.kind === "file" ? words.removeFile : words.removeContext;
}
