import { describe, expect, it } from "vitest";

import { lineStates, tokenize } from "./syntax";
import { languageOf } from "./syntax-languages";

/** The coloured runs of `line`, as `kind:text`. */
function runs(line: string, path: string): string[] {
  return tokenize(line, languageOf(path))
    .tokens.filter((token) => token.kind !== null)
    .map((token) => `${token.kind ?? ""}:${token.text}`);
}

describe("syntax colour", () => {
  it("chooses the language by the file's name", () => {
    expect(languageOf("src/app.tsx").name).toBe("typescript");
    expect(languageOf("crates/x/src/lib.rs").name).toBe("rust");
    expect(languageOf("Dockerfile").name).toBe("shell");
    expect(languageOf("notes.unknownext").name).toBe("text");
  });

  it("colours keywords, strings, numbers, calls and comments", () => {
    expect(runs('const x = load("a.json", 42); // why', "a.ts")).toEqual([
      "keyword:const",
      "function:load",
      'string:"a.json"',
      "number:42",
      "comment:// why",
    ]);
  });

  it("ends a string at its quote, not at an escaped one", () => {
    expect(runs(String.raw`let s = "a \" b"; true`, "a.rs")).toEqual([
      "keyword:let",
      String.raw`string:"a \" b"`,
      "constant:true",
    ]);
  });

  it("carries a block comment to the next line", () => {
    const language = languageOf("a.go");
    const first = tokenize("x := 1 /* starts", language);
    expect(first.state).toBe("comment");
    const second = tokenize("still */ y", language, first.state);
    expect(second.tokens[0]).toEqual({ text: "still */", kind: "comment" });
    expect(lineStates(["/* a", "b */", "c"], language)).toEqual([
      "code",
      "comment",
      "code",
    ]);
  });

  it("reads SQL keywords in any case and markup's tags", () => {
    expect(runs("SELECT id FROM t", "q.sql")).toEqual([
      "keyword:SELECT",
      "keyword:FROM",
    ]);
    expect(runs('<a href="x">', "p.html")).toEqual([
      "tag:<a",
      "function:href",
      'string:"x"',
    ]);
  });

  it("keeps a line's text whole", () => {
    const line = '  fn main() { println!("hi"); } // done';
    const { tokens } = tokenize(line, languageOf("main.rs"));
    expect(tokens.map((token) => token.text).join("")).toBe(line);
  });
});
