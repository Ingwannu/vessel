//! The languages the lexer colours, by what it needs of each: how comments
//! and strings are written, and the words that are keywords or constants.
//! Chosen from a file's name. The page's
//! `data/workspace/syntax-languages.ts`, word for word.

use std::collections::HashSet;
use std::sync::LazyLock;

/// How one language's text is read.
#[derive(Debug)]
pub struct Language {
    /// The name a reference uses for it (a fence's info string).
    pub name: &'static str,
    /// Where a comment runs to the end of the line.
    pub(super) line: &'static [&'static str],
    /// A comment's opening and closing marks, when it has block comments.
    pub(super) block: Option<(&'static str, &'static str)>,
    /// The characters that open (and close) a string.
    pub(super) quotes: &'static str,
    pub(super) keywords: HashSet<&'static str>,
    pub(super) constants: HashSet<&'static str>,
    /// Whether keywords match in any case (SQL).
    pub(super) any_case: bool,
    /// Markup: `<tag attr="value">`, with `<!-- -->` comments.
    pub(super) markup: bool,
}

/// The shape of a language before its word sets are made.
struct Shape {
    line: &'static [&'static str],
    block: Option<(&'static str, &'static str)>,
    quotes: &'static str,
    keywords: &'static str,
    constants: &'static str,
    any_case: bool,
    markup: bool,
}

const PLAIN_SHAPE: Shape = Shape {
    line: &[],
    block: None,
    quotes: "\"'",
    keywords: "",
    constants: "",
    any_case: false,
    markup: false,
};

fn language(name: &'static str, shape: &Shape) -> Language {
    let words = |text: &'static str| text.split(' ').filter(|word| !word.is_empty()).collect();
    Language {
        name,
        line: shape.line,
        block: shape.block,
        quotes: shape.quotes,
        keywords: words(shape.keywords),
        constants: words(shape.constants),
        any_case: shape.any_case,
        markup: shape.markup,
    }
}

const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));

const SCRIPT_KEYWORDS: &str = "abstract as async await break case catch class const continue debugger declare default delete do else enum export extends finally for from function get if implements import in instanceof interface keyof let namespace new of private protected public readonly return satisfies set static super switch this throw try type typeof var void while with yield";

/// Every language, in the order of [`by_extension`]'s table.
static LANGUAGES: LazyLock<Languages> = LazyLock::new(|| Languages {
    typescript: language(
        "typescript",
        &Shape {
            line: &["//"],
            block: C_BLOCK,
            quotes: "\"'`",
            keywords: SCRIPT_KEYWORDS,
            constants: "true false null undefined NaN Infinity",
            ..PLAIN_SHAPE
        },
    ),
    rust: language(
        "rust",
        &Shape {
            line: &["//"],
            block: C_BLOCK,
            quotes: "\"",
            keywords: "as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait type unsafe use where while",
            constants: "true false None Some Ok Err",
            ..PLAIN_SHAPE
        },
    ),
    go: language(
        "go",
        &Shape {
            line: &["//"],
            block: C_BLOCK,
            quotes: "\"'`",
            keywords: "break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var",
            constants: "true false nil iota",
            ..PLAIN_SHAPE
        },
    ),
    c: language(
        "c",
        &Shape {
            line: &["//"],
            block: C_BLOCK,
            keywords: "auto break case catch char class const constexpr continue default delete do double else enum extern final float for fun goto if import include inline int interface let long namespace new override package private protected public return short signed sizeof static struct super switch template this throw try typedef union unsigned using val var virtual void volatile when while",
            constants: "true false null nullptr NULL",
            ..PLAIN_SHAPE
        },
    ),
    python: language(
        "python",
        &Shape {
            line: &["#"],
            keywords: "and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield",
            constants: "True False None self",
            ..PLAIN_SHAPE
        },
    ),
    shell: language(
        "shell",
        &Shape {
            line: &["#"],
            keywords: "if then else elif fi for in do done while until case esac function return local export set unset echo exit",
            ..PLAIN_SHAPE
        },
    ),
    ruby: language(
        "ruby",
        &Shape {
            line: &["#"],
            keywords: "alias and begin break case class def defined do else elsif end ensure for if in module next not or redo rescue retry return self super then undef unless until when while yield require",
            constants: "true false nil",
            ..PLAIN_SHAPE
        },
    ),
    data: language(
        "data",
        &Shape {
            line: &["#"],
            constants: "true false null yes no on off",
            ..PLAIN_SHAPE
        },
    ),
    json: language(
        "json",
        &Shape {
            quotes: "\"",
            constants: "true false null",
            ..PLAIN_SHAPE
        },
    ),
    css: language(
        "css",
        &Shape {
            block: C_BLOCK,
            keywords: "@media @import @supports @keyframes @layer @font-face",
            constants: "inherit initial unset none auto important",
            ..PLAIN_SHAPE
        },
    ),
    sql: language(
        "sql",
        &Shape {
            line: &["--"],
            block: C_BLOCK,
            quotes: "'\"",
            keywords: "select from where and or not insert into values update set delete create table index view drop alter add primary key foreign references join left right inner outer on group by order having limit offset as distinct union all case when then else end begin commit rollback with returning",
            constants: "null true false",
            any_case: true,
            ..PLAIN_SHAPE
        },
    ),
    markup: language(
        "markup",
        &Shape {
            markup: true,
            ..PLAIN_SHAPE
        },
    ),
    plain: language("text", &PLAIN_SHAPE),
});

struct Languages {
    typescript: Language,
    rust: Language,
    go: Language,
    c: Language,
    python: Language,
    shell: Language,
    ruby: Language,
    data: Language,
    json: Language,
    css: Language,
    sql: Language,
    markup: Language,
    plain: Language,
}

/// The language of a file extension (lower case).
fn by_extension(extension: &str) -> Option<&'static Language> {
    let all = &*LANGUAGES;
    Some(match extension {
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" => &all.typescript,
        "rs" => &all.rust,
        "go" => &all.go,
        "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "m" | "mm" | "java" | "kt" | "kts"
        | "swift" | "cs" | "scala" | "dart" => &all.c,
        "py" | "pyi" => &all.python,
        "sh" | "bash" | "zsh" | "fish" | "ps1" => &all.shell,
        "rb" | "rake" | "gemspec" => &all.ruby,
        "toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "env" | "properties" => &all.data,
        "json" | "jsonc" | "json5" | "lock" => &all.json,
        "css" | "scss" | "less" => &all.css,
        "sql" => &all.sql,
        "html" | "htm" | "xml" | "svg" | "vue" | "svelte" | "astro" => &all.markup,
        _other => return None,
    })
}

/// The language of a whole file name (lower case).
fn by_name(name: &str) -> Option<&'static Language> {
    let all = &*LANGUAGES;
    Some(match name {
        "dockerfile" | "makefile" | "justfile" => &all.shell,
        ".gitignore" | ".env" | "cargo.lock" => &all.data,
        _other => return None,
    })
}

/// The language of a file named `path`, plain text when none fits.
#[must_use]
pub fn language_of(path: &str) -> &'static Language {
    let name = path.rsplit('/').next().unwrap_or_default().to_lowercase();
    if let Some(language) = by_name(&name) {
        return language;
    }
    name.rfind('.')
        .and_then(|dot| name.get(dot.saturating_add(1)..))
        .and_then(by_extension)
        .unwrap_or(&LANGUAGES.plain)
}
