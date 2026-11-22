import { StreamLanguage, syntaxTree } from "@codemirror/language";
import type { Range } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  type EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";

import {
  type LineState,
  SYNTAX_CLASS,
  SYNTAX_LINE_LIMIT,
  type Token,
  type TokenKind,
  tokenize,
} from "../../data/workspace/syntax";
import type { Language } from "../../data/workspace/syntax-languages";
import "./code-syntax.css";

/** CodeMirror token names carry the same classifications through incremental parsing. */
const STREAM_NAME: Readonly<Record<TokenKind, string>> = {
  keyword: "keyword",
  string: "string",
  comment: "comment",
  number: "number",
  constant: "atom",
  function: "variableName.function",
  tag: "tagName",
};

function streamMark(kind: TokenKind): [string, Decoration] {
  return [STREAM_NAME[kind], Decoration.mark({ class: SYNTAX_CLASS[kind] })];
}

const MARK = new Map([
  streamMark("keyword"),
  streamMark("string"),
  streamMark("comment"),
  streamMark("number"),
  streamMark("constant"),
  streamMark("function"),
  streamMark("tag"),
]);

type StreamState = {
  nextLine: LineState;
  parts: readonly Token[];
  next: number;
};

/** Keep block-comment state in CodeMirror's parser and share the reader's classes. */
export function fileEditorSyntax(language: Language) {
  const lexer = StreamLanguage.define<StreamState>({
    name: language.name,
    startState: () => ({ nextLine: "code", parts: [], next: 0 }),
    token(stream, state) {
      if (stream.sol()) {
        // Read the whole line for its next-line state, even past the colour limit.
        const line = tokenize(stream.string, language, state.nextLine);
        state.nextLine = line.state;
        state.parts = line.tokens;
        state.next = 0;
      }
      const remaining = SYNTAX_LINE_LIMIT - stream.pos;
      const part = state.parts[state.next];
      if (remaining <= 0 || part === undefined) {
        stream.skipToEnd();
        state.parts = [];
        return null;
      }
      state.next += 1;
      stream.pos += Math.min(part.text.length, remaining);
      if (stream.eol()) state.parts = [];
      return part.kind === null ? null : STREAM_NAME[part.kind];
    },
  });
  return [lexer, tokenColours];
}

function visibleColours(view: EditorView): DecorationSet {
  const tree = syntaxTree(view.state);
  const marks: Range<Decoration>[] = [];
  for (const { from, to } of view.visibleRanges) {
    tree.iterate({
      from,
      to,
      enter(node) {
        const mark = MARK.get(node.name);
        const start = Math.max(from, node.from);
        const end = Math.min(to, node.to);
        if (mark !== undefined && start < end) {
          marks.push(mark.range(start, end));
        }
      },
    });
  }
  return Decoration.set(marks);
}

const tokenColours = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = visibleColours(view);
    }

    update(update: ViewUpdate) {
      if (
        update.docChanged ||
        update.viewportChanged ||
        syntaxTree(update.startState) !== syntaxTree(update.state)
      ) {
        this.decorations = visibleColours(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);
