// The languages `syntax.ts` colours, by what a lexer needs of each: how
// comments and strings are written, and the words that are keywords or
// constants. Chosen from a file's name.

/** How one language's text is read. */
export interface Language {
  /** The name a reference uses for it (a fence's info string). */
  name: string;
  /** Where a comment runs to the end of the line. */
  line: readonly string[];
  /** A comment's opening and closing marks, when it has block comments. */
  block: readonly [string, string] | null;
  /** The characters that open (and close) a string. */
  quotes: string;
  keywords: ReadonlySet<string>;
  constants: ReadonlySet<string>;
  /** Whether keywords match in any case (SQL). */
  anyCase: boolean;
  /** Markup: `<tag attr="value">`, with `<!-- -->` comments. */
  markup: boolean;
}

function words(text: string): ReadonlySet<string> {
  return new Set(text.split(" ").filter((word) => word !== ""));
}

const C_CONSTANTS = words("true false null nullptr NULL");

function language(
  name: string,
  shape: Partial<Omit<Language, "name">>,
): Language {
  return {
    name,
    line: shape.line ?? [],
    block: shape.block ?? null,
    quotes: shape.quotes ?? "\"'",
    keywords: shape.keywords ?? new Set(),
    constants: shape.constants ?? new Set(),
    anyCase: shape.anyCase ?? false,
    markup: shape.markup ?? false,
  };
}

const SCRIPT_KEYWORDS =
  "abstract as async await break case catch class const continue debugger declare default delete do else enum export extends finally for from function get if implements import in instanceof interface keyof let namespace new of private protected public readonly return satisfies set static super switch this throw try type typeof var void while with yield";

const TYPESCRIPT = language("typescript", {
  line: ["//"],
  block: ["/*", "*/"],
  quotes: "\"'`",
  keywords: words(SCRIPT_KEYWORDS),
  constants: words("true false null undefined NaN Infinity"),
});

const RUST = language("rust", {
  line: ["//"],
  block: ["/*", "*/"],
  quotes: '"',
  keywords: words(
    "as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait type unsafe use where while",
  ),
  constants: words("true false None Some Ok Err"),
});

const GO = language("go", {
  line: ["//"],
  block: ["/*", "*/"],
  quotes: "\"'`",
  keywords: words(
    "break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var",
  ),
  constants: words("true false nil iota"),
});

const C_FAMILY = language("c", {
  line: ["//"],
  block: ["/*", "*/"],
  keywords: words(
    "auto break case catch char class const constexpr continue default delete do double else enum extern final float for fun goto if import include inline int interface let long namespace new override package private protected public return short signed sizeof static struct super switch template this throw try typedef union unsigned using val var virtual void volatile when while",
  ),
  constants: C_CONSTANTS,
});

const PYTHON = language("python", {
  line: ["#"],
  keywords: words(
    "and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield",
  ),
  constants: words("True False None self"),
});

const SHELL = language("shell", {
  line: ["#"],
  keywords: words(
    "if then else elif fi for in do done while until case esac function return local export set unset echo exit",
  ),
});

const RUBY = language("ruby", {
  line: ["#"],
  keywords: words(
    "alias and begin break case class def defined do else elsif end ensure for if in module next not or redo rescue retry return self super then undef unless until when while yield require",
  ),
  constants: words("true false nil"),
});

const DATA = language("data", {
  line: ["#"],
  constants: words("true false null yes no on off"),
});

const JSON_LANGUAGE = language("json", {
  quotes: '"',
  constants: words("true false null"),
});

const CSS = language("css", {
  line: [],
  block: ["/*", "*/"],
  keywords: words("@media @import @supports @keyframes @layer @font-face"),
  constants: words("inherit initial unset none auto important"),
});

const SQL = language("sql", {
  line: ["--"],
  block: ["/*", "*/"],
  quotes: "'\"",
  keywords: words(
    "select from where and or not insert into values update set delete create table index view drop alter add primary key foreign references join left right inner outer on group by order having limit offset as distinct union all case when then else end begin commit rollback with returning",
  ),
  constants: words("null true false"),
  anyCase: true,
});

const MARKUP = language("markup", { markup: true });

const PLAIN = language("text", {});

/** File extensions (lower case) and whole names, by language. */
const BY_EXTENSION: ReadonlyMap<string, Language> = new Map(
  (
    [
      [TYPESCRIPT, "ts tsx mts cts js jsx mjs cjs"],
      [RUST, "rs"],
      [GO, "go"],
      [C_FAMILY, "c h cc cpp cxx hpp hh m mm java kt kts swift cs scala dart"],
      [PYTHON, "py pyi"],
      [SHELL, "sh bash zsh fish ps1"],
      [RUBY, "rb rake gemspec"],
      [DATA, "toml yaml yml ini conf cfg env properties"],
      [JSON_LANGUAGE, "json jsonc json5 lock"],
      [CSS, "css scss less"],
      [SQL, "sql"],
      [MARKUP, "html htm xml svg vue svelte astro"],
    ] satisfies [Language, string][]
  ).flatMap(([lang, extensions]) =>
    extensions.split(" ").map((extension) => [extension, lang] as const),
  ),
);

const BY_NAME: ReadonlyMap<string, Language> = new Map([
  ["dockerfile", SHELL],
  ["makefile", SHELL],
  ["justfile", SHELL],
  [".gitignore", DATA],
  [".env", DATA],
  ["cargo.lock", DATA],
]);

/** The language of a file named `path`, plain text when none fits. */
export function languageOf(path: string): Language {
  const name = (path.split("/").at(-1) ?? "").toLowerCase();
  const whole = BY_NAME.get(name);
  if (whole !== undefined) {
    return whole;
  }
  const dot = name.lastIndexOf(".");
  return dot < 0 ? PLAIN : (BY_EXTENSION.get(name.slice(dot + 1)) ?? PLAIN);
}
