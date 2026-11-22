// One line of code in its syntax colours (data/workspace/syntax.ts), as
// the diff and the file viewer draw it: runs of text in spans named by
// what they are, coloured by the page's `--syn-*` tokens.

import { useMemo } from "react";

import {
  type LineState,
  SYNTAX_CLASS,
  SYNTAX_LINE_LIMIT,
  tokenize,
} from "../../data/workspace/syntax";
import type { Language } from "../../data/workspace/syntax-languages";
import "./code-syntax.css";

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
    () => tokenize(text.slice(0, SYNTAX_LINE_LIMIT), language, state).tokens,
    [text, language, state],
  );
  return (
    <>
      {tokens.map((token, index) =>
        token.kind === null ? (
          token.text
        ) : (
          <span key={index} className={SYNTAX_CLASS[token.kind]}>
            {token.text}
          </span>
        ),
      )}
      {text.length > SYNTAX_LINE_LIMIT ? text.slice(SYNTAX_LINE_LIMIT) : null}
    </>
  );
}
