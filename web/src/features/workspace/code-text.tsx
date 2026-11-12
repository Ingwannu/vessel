// One line of code in its syntax colours (data/workspace/syntax.ts), as
// the diff and the file viewer draw it: runs of text in spans named by
// what they are, coloured by the page's `--syn-*` tokens (code.css).

import { useMemo } from "react";

import {
  type LineState,
  type TokenKind,
  tokenize,
} from "../../data/workspace/syntax";
import type { Language } from "../../data/workspace/syntax-languages";

function tokenClass(kind: TokenKind): string {
  switch (kind) {
    case "keyword":
      return "syn-keyword";
    case "string":
      return "syn-string";
    case "comment":
      return "syn-comment";
    case "number":
      return "syn-number";
    case "constant":
      return "syn-const";
    case "function":
      return "syn-func";
    case "tag":
      return "syn-keyword";
  }
}

export function CodeText({
  text,
  language,
  state = "code",
}: {
  text: string;
  language: Language;
  /** What the line before left open (a block comment). */
  state?: LineState;
}) {
  const tokens = useMemo(
    () => tokenize(text, language, state).tokens,
    [text, language, state],
  );
  return (
    <>
      {tokens.map((token, index) =>
        token.kind === null ? (
          token.text
        ) : (
          <span key={index} className={tokenClass(token.kind)}>
            {token.text}
          </span>
        ),
      )}
    </>
  );
}
