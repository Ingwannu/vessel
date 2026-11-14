// Syntax colour for a diff's lines and a file's, by a small lexer:
// comments, strings, numbers, keywords, constants and called names,
// coloured with the page's `--syn-*` tokens (DESIGN.md §4.4).
//
// [Decision Log]
// - Purpose: "a diff or a code viewer needs syntax colour".
// - Constraint: the brief says to use what the page already ships for
//   Markdown's code blocks, and that a new dependency needs a reason; the
//   page ships no highlighter (react-markdown draws code blocks plain,
//   and the tool cards' JSON colours are their own few spans). The panel
//   is a chunk loaded when it opens, and its lines are drawn a window at
//   a time, so colouring must be cheap per line.
// - Alternatives: Shiki (what GOAT and T3 Code use: exact TextMate
//   grammars, but its engine and grammars are megabytes, loaded per
//   language, with a WebAssembly regex engine the page's content policy
//   would have to admit); highlight.js or Prism with a dozen languages
//   (40 to 80 KB compressed, and still a regex grammar per language).
// - Choice: a lexer of a few rules per language family
//   (`syntax-languages.ts`): what a reader most uses colour for (where a
//   string or a comment ends, which words are keywords). A line is read
//   from the state the line before left (inside a block comment or not),
//   so a file is read once in order; a diff's lines, whose neighbours
//   are not the file's, are read each on its own.
// - Cost: no grammar's precision (a regular expression literal, a
//   template's nested expression, a heredoc are not told apart), and a
//   block comment opened outside a diff's hunk is not coloured in it.

import type { Language } from "./syntax-languages";

/** What a run of text is, for its colour (`null`: plain). */
export type TokenKind =
  "keyword" | "string" | "comment" | "number" | "constant" | "function" | "tag";

/** A run of a line's text. */
export interface Token {
  text: string;
  kind: TokenKind | null;
}

/** What a line leaves open for the next: inside a block comment. */
export type LineState = "code" | "comment";

/** Markup's comment marks. */
const MARKUP_COMMENT: readonly [string, string] = ["<!--", "-->"];

const WORD = /[A-Za-z_$@][\w$-]*/y;
const NUMBER =
  /(?:0[xX][\da-fA-F_]+|0[bB][01_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?)[a-zA-Z%]*/y;

/** Whether `word` is one of `set`, in `language`'s case rule. */
function isIn(
  set: ReadonlySet<string>,
  word: string,
  language: Language,
): boolean {
  return set.has(language.anyCase ? word.toLowerCase() : word);
}

/** The tokens of one line of `language`, read from `state`, and the
 * state it leaves. */
export function tokenize(
  text: string,
  language: Language,
  state: LineState = "code",
): { tokens: Token[]; state: LineState } {
  const tokens: Token[] = [];
  const push = (part: string, kind: TokenKind | null) => {
    const last = tokens.at(-1);
    if (last?.kind === kind) {
      last.text += part;
    } else if (part !== "") {
      tokens.push({ text: part, kind });
    }
  };
  const block = language.markup ? MARKUP_COMMENT : language.block;
  let index = 0;
  if (state === "comment" && block !== null) {
    const end = text.indexOf(block[1]);
    if (end < 0) {
      push(text, "comment");
      return { tokens, state: "comment" };
    }
    index = end + block[1].length;
    push(text.slice(0, index), "comment");
  }
  while (index < text.length) {
    const rest = text.slice(index);
    const char = text.charAt(index);
    const lineMark = language.line.find((mark) => rest.startsWith(mark));
    if (lineMark !== undefined) {
      push(rest, "comment");
      break;
    }
    if (block !== null && rest.startsWith(block[0])) {
      const end = text.indexOf(block[1], index + block[0].length);
      if (end < 0) {
        push(rest, "comment");
        return { tokens, state: "comment" };
      }
      const after = end + block[1].length;
      push(text.slice(index, after), "comment");
      index = after;
      continue;
    }
    if (language.quotes.includes(char) && !(language.markup && char === "'")) {
      const end = closing(text, index, char);
      push(text.slice(index, end), "string");
      index = end;
      continue;
    }
    if (language.markup && char === "<") {
      const tag = /<\/?[A-Za-z][\w:.-]*/y;
      tag.lastIndex = index;
      const found = tag.exec(text);
      if (found !== null) {
        push(found[0], "tag");
        index += found[0].length;
        continue;
      }
    }
    NUMBER.lastIndex = index;
    const number = /[\d]/.test(char) ? NUMBER.exec(text) : null;
    if (number !== null) {
      push(number[0], "number");
      index += number[0].length;
      continue;
    }
    WORD.lastIndex = index;
    const word = WORD.exec(text);
    if (word !== null) {
      const value = word[0];
      index += value.length;
      push(value, wordKind(value, text.charAt(index), language));
      continue;
    }
    push(char, null);
    index += 1;
  }
  return { tokens, state: "code" };
}

/** What a word is: a keyword, a constant, a called name, or plain. */
function wordKind(
  word: string,
  next: string,
  language: Language,
): TokenKind | null {
  if (language.markup) {
    return next === "=" ? "function" : null;
  }
  if (isIn(language.keywords, word, language)) {
    return "keyword";
  }
  if (isIn(language.constants, word, language)) {
    return "constant";
  }
  return next === "(" ? "function" : null;
}

/** Where the string opened by `quote` at `start` ends (after its closing
 * quote, or the line's end): a backslash escapes the next character. */
function closing(text: string, start: number, quote: string): number {
  let index = start + 1;
  while (index < text.length) {
    const char = text.charAt(index);
    if (char === "\\") {
      index += 2;
      continue;
    }
    index += 1;
    if (char === quote) {
      return index;
    }
  }
  return text.length;
}

/** The state each line of `lines` starts in, read in order: what a
 * windowed viewer needs to colour any line without the ones before it.
 * One pass, no tokens kept. */
export function lineStates(
  lines: readonly string[],
  language: Language,
): LineState[] {
  const states: LineState[] = [];
  let state: LineState = "code";
  for (const line of lines) {
    states.push(state);
    state =
      language.block === null && !language.markup
        ? "code"
        : tokenize(line, language, state).state;
  }
  return states;
}
