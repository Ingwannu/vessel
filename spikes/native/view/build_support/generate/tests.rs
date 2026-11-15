//! Exercise generated Rust with the real compiler and Clippy, not a line-count
//! parser. The fake resolver deliberately tests cache construction, borrowing
//! and error propagation only; the real formatter has its own runtime tests.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::super::catalog;

const LARGE: usize = 132;
const DEEP: usize = super::plain::FIELDS_PER_RESOLVER * super::plain::FIELDS_PER_RESOLVER + 1;

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("vessel-plain-{}-{name}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

fn compiler(clippy: bool) -> PathBuf {
    let name = if clippy { "clippy-driver" } else { "rustc" };
    std::env::var_os("RUSTC")
        .and_then(|path| {
            PathBuf::from(path)
                .parent()
                .map(|parent| parent.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
        })
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

fn compile(directory: &Path, clippy: bool) -> Output {
    let mut command = Command::new(compiler(clippy));
    command
        .current_dir(directory)
        .env_remove("CLIPPY_ARGS")
        .env("CLIPPY_CONF_DIR", directory)
        .args([
            "--edition=2024",
            "--crate-name",
            "plain_fixture",
            "fixture.rs",
            "-D",
            "warnings",
        ]);
    if clippy {
        command.args([
            "--emit=metadata",
            "-D",
            "clippy::too_many_lines",
            "-o",
            "checked.rmeta",
        ]);
    } else {
        command.arg("-o").arg(executable(directory));
    }
    command.output().unwrap()
}

fn executable(directory: &Path) -> PathBuf {
    directory.join(format!("fixture{}", std::env::consts::EXE_SUFFIX))
}

#[test]
fn large_plain_caches_keep_the_public_borrowed_api_and_bounded_initializers() {
    let scratch = Scratch::new("contract");
    let mut control = "fn over_limit() {\n".to_owned();
    for index in 0..LARGE {
        writeln!(control, "let _unused_{index} = {index};").unwrap();
    }
    control.push_str("}\nfn main() { over_limit(); }\n");
    std::fs::write(scratch.0.join("fixture.rs"), control).unwrap();
    let rejected = compile(&scratch.0, true);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("too_many_lines"));

    for (count, many_areas) in [(LARGE, false), (DEEP, false), (LARGE, true)] {
        let source = fixture(count, many_areas);
        std::fs::write(scratch.0.join("fixture.rs"), source).unwrap();
        for clippy in [true, false] {
            let checked = compile(&scratch.0, clippy);
            assert!(
                checked.status.success(),
                "{}",
                String::from_utf8_lossy(&checked.stderr)
            );
        }
        let ran = Command::new(executable(&scratch.0))
            .arg("resolved-value")
            .output()
            .unwrap();
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
    }
}

fn fixture(count: usize, many_areas: bool) -> String {
    let mut files = BTreeMap::new();
    let mut getters = Vec::new();
    let mut keys = Vec::new();
    for index in 0..count {
        let (area, local) = if many_areas {
            (format!("area-{index:05}"), "title".to_owned())
        } else {
            ("stress".to_owned(), format!("item-{index:05}"))
        };
        let key = format!("{area}-{local}");
        writeln!(
            files.entry(area.clone()).or_default(),
            "# Short fixture message.\n{key} = Value {index}"
        )
        .unwrap();
        getters.push(format!(
            "|translator| translator.r#{}().r#{}(),",
            super::snake(&area),
            super::snake(&local)
        ));
        keys.push(key);
    }
    if !many_areas {
        // A real area may look like an implementation type's name. Private
        // chunk types must live in their own namespace rather than collide.
        let _previous = files.insert(
            "stress-node0".to_owned(),
            "# Short alias fixture.\nstress-node0-title = Other".to_owned(),
        );
        getters.push("|translator| translator.stress_node0().title(),".to_owned());
        keys.push("stress-node0-title".to_owned());
    }
    let language = catalog::parse("en".to_owned(), files).unwrap();
    let mut output = SHIM.to_owned();
    super::api(&mut output, &language).unwrap();
    writeln!(
        output,
        "type Getter = fn(&Translator) -> &str;\nconst GETTERS: [Getter; {}] = [\n{}\n];",
        getters.len(),
        getters.join("\n")
    )
    .unwrap();
    writeln!(output, "const KEYS: [&str; {}] = {:?};", keys.len(), keys).unwrap();
    writeln!(
        output,
        "const FAIL_AT: usize = {};",
        super::plain::FIELDS_PER_RESOLVER
    )
    .unwrap();
    output.push_str(MAIN);
    output
}

const SHIM: &str = r#"
use std::cell::Cell;
#[derive(Debug)]
struct InitError;
struct Bundle { calls: Cell<usize>, fail_at: Option<usize>, prefix: String }
fn resolve(bundle: &Bundle, _english: &Bundle, key: &str, _args: Option<()>) -> Result<String, InitError> {
    let call = bundle.calls.get();
    bundle.calls.set(call + 1);
    if bundle.fail_at == Some(call) { return Err(InitError); }
    Ok(format!("{}:{key}", bundle.prefix))
}
#[derive(Debug)]
struct Translator { plain: Plain }
"#;

const MAIN: &str = r#"
fn main() {
    let prefix = std::env::args().nth(1).unwrap();
    let bundle = Bundle { calls: Cell::new(0), fail_at: None, prefix: prefix.clone() };
    let translator = Translator { plain: Plain::resolve(&bundle, &bundle).unwrap() };
    assert_eq!(bundle.calls.get(), KEYS.len());
    for (getter, key) in GETTERS.iter().zip(KEYS) {
        let first = getter(&translator);
        assert_eq!(first, format!("{prefix}:{key}"));
        assert!(std::ptr::eq(first.as_ptr(), getter(&translator).as_ptr()));
    }
    assert_eq!(bundle.calls.get(), KEYS.len());
    let failed = Bundle { calls: Cell::new(0), fail_at: Some(FAIL_AT), prefix };
    assert!(Plain::resolve(&failed, &failed).is_err());
    assert_eq!(failed.calls.get(), FAIL_AT + 1);
}
"#;
