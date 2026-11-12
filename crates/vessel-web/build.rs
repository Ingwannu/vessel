//! Embeds the web UI's files into the crate (ADR 0017).
//!
//! [Decision Log]
//! - Purpose: `vessel ui` serves the single-page app from the binary, so a
//!   release is one file and nothing is read from disk at run time.
//! - Constraint: the Rust gate (`cargo build`, `cargo test`) must not need
//!   Node, and nothing built is committed.
//! - Choice: the bundle Vite writes (`web/dist`, or `VESSEL_WEB_DIST`) is
//!   embedded file by file with `include_bytes!` when it has an
//!   `index.html`; otherwise the placeholder page in `assets/placeholder`,
//!   which says how to build the UI. `VESSEL_WEB_REQUIRE_BUNDLE=1` (CI's web
//!   job, the release) makes a missing bundle a build error instead.
//! - Each file is recorded with its content's SHA-256 (its entity tag) and
//!   whether its name carries a content hash (`src/hashed.rs`), which
//!   decides its caching (`files`).
//! - Compressed forms (milestone 11, `docs/PERFORMANCE.md`): the web build
//!   writes `<file>.br` and `<file>.gz` beside a text file and lists, in
//!   `.encodings`, the SHA-256 of each file it compressed. They are
//!   embedded with their file, not as files of their own; one whose file
//!   no longer has that hash (an edited or rebuilt file next to an old
//!   variant) fails the build, so a stale variant is never served.
//!   Hidden files (the manifest) are not embedded.
//! - The index's import map (milestone 11, ADR 0017's amendment): the one
//!   inline script the page has, pinning the chunks loaded later. Its
//!   SHA-256 goes into `csp.rs` as the macro `import_map_source!`, which
//!   `guard` expands into the policy's `script-src`, so the policy stays
//!   one static string and admits exactly that map.
//! - Watching: the bundle's directory when it exists; otherwise all of
//!   `web/`, so a bundle built later is noticed (cargo scans a watched
//!   directory recursively).
//! - Alternatives: `rust-embed` (a dependency for forty lines);
//!   committing `web/dist` (minified diffs in every UI change).

use base64::Engine as _;
use sha2::{Digest as _, Sha256};

// The rule for content-hashed names, shared with the crate's tests.
#[path = "src/hashed.rs"]
mod hashed;
use hashed::hashed;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs, io};

/// One file to embed.
struct File {
    /// Its URL path, from `/`.
    url: String,
    /// Where it is on disk.
    disk: PathBuf,
}

/// The compressed forms the build makes: their suffix, and their field in
/// `File`.
const VARIANTS: [(&str, &str); 2] = [(".gz", "gzip"), (".br", "brotli")];

/// Compressed forms found, each with the URL of the file it belongs to.
type Variants = Vec<(String, File)>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            let _said = writeln!(io::stdout().lock(), "cargo::error={message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("no CARGO_MANIFEST_DIR")?);
    let web = manifest.join("../../web");
    let dist = env::var_os("VESSEL_WEB_DIST").map_or_else(|| web.join("dist"), PathBuf::from);
    let required = env::var_os("VESSEL_WEB_REQUIRE_BUNDLE").is_some_and(|value| value == "1");
    let mut cargo = String::new();
    for variable in ["VESSEL_WEB_DIST", "VESSEL_WEB_REQUIRE_BUNDLE"] {
        let _written = writeln!(cargo, "cargo::rerun-if-env-changed={variable}");
    }
    let _written = writeln!(
        cargo,
        "cargo::rerun-if-changed={}",
        manifest.join("src/hashed.rs").display()
    );
    let placeholder = manifest.join("assets/placeholder");
    let _written = writeln!(cargo, "cargo::rerun-if-changed={}", placeholder.display());
    let bundled = dist.join("index.html").is_file();
    let (root, watched) = if bundled {
        (dist.clone(), dist)
    } else if required {
        return Err(format!(
            "VESSEL_WEB_REQUIRE_BUNDLE=1, but there is no web bundle at {} \
             (run `npm ci && npm run build` in web/)",
            dist.display()
        ));
    } else {
        (placeholder, web)
    };
    let _written = writeln!(cargo, "cargo::rerun-if-changed={}", watched.display());
    let mut found = Vec::new();
    collect(&root, &root, &mut found)
        .map_err(|error| format!("reading {}: {error}", root.display()))?;
    let manifest = fs::read_to_string(root.join(".encodings")).unwrap_or_default();
    let (mut files, variants) = split(found, &manifest)?;
    files.sort_by(|a, b| a.url.cmp(&b.url));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("no OUT_DIR")?);
    let out = out_dir.join("files.rs");
    write_if_changed(
        &out,
        &table(&files, &variants, bundled).map_err(|error| format!("hashing assets: {error}"))?,
    )
    .map_err(|error| format!("writing {}: {error}", out.display()))?;
    let csp = out_dir.join("csp.rs");
    let index = fs::read_to_string(root.join("index.html"))
        .map_err(|error| format!("reading the index: {error}"))?;
    write_if_changed(&csp, &import_map_source(&index)?)
        .map_err(|error| format!("writing {}: {error}", csp.display()))?;
    io::stdout()
        .lock()
        .write_all(cargo.as_bytes())
        .map_err(|error| format!("talking to cargo: {error}"))
}

/// Every file under `dir`, by its path from `root`.
fn collect(root: &Path, dir: &Path, files: &mut Vec<File>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if entry.file_type()?.is_dir() {
            collect(root, &path, files)?;
        } else {
            let relative = path.strip_prefix(root).map_err(io::Error::other)?;
            let url = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .fold(String::new(), |mut url, part| {
                    url.push('/');
                    url.push_str(&part);
                    url
                });
            files.push(File { url, disk: path });
        }
    }
    Ok(())
}

/// The files to serve, and the compressed forms among `found` by the URL
/// of the file each belongs to. A compressed form must be listed in
/// `manifest` (`<url> <sha256>` lines) with its file's current hash.
fn split(found: Vec<File>, manifest: &str) -> Result<(Vec<File>, Variants), String> {
    let urls: std::collections::HashSet<String> =
        found.iter().map(|file| file.url.clone()).collect();
    let mut files = Vec::new();
    let mut variants = Vec::new();
    for file in found {
        let base = VARIANTS.iter().find_map(|(suffix, _name)| {
            file.url
                .strip_suffix(suffix)
                .filter(|base| urls.contains(*base))
                .map(str::to_owned)
        });
        match base {
            Some(base) => variants.push((base, file)),
            None => files.push(file),
        }
    }
    for (base, variant) in &variants {
        let Some(file) = files.iter().find(|file| &file.url == base) else {
            continue;
        };
        let listed = manifest
            .lines()
            .find_map(|line| line.strip_prefix(base.as_str())?.strip_prefix(' '));
        let actual = hex(&fs::read(&file.disk).map_err(|error| error.to_string())?);
        if listed != Some(actual.as_str()) {
            return Err(format!(
                "{} was not made from {} as it is now (run `npm run build` in web/)",
                variant.disk.display(),
                file.disk.display()
            ));
        }
    }
    Ok((files, variants))
}

/// `bytes`' SHA-256 in lowercase hex.
fn hex(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in Sha256::digest(bytes) {
        let _written = write!(text, "{byte:02x}");
    }
    text
}

/// The generated table: `FILES` and whether it is the real bundle.
fn table(files: &[File], variants: &Variants, bundled: bool) -> io::Result<String> {
    let mut out = String::from("/// The web UI's files, generated by `build.rs`.\n");
    out.push_str("pub(crate) static FILES: &[File] = &[\n");
    for file in files {
        let digest = hex(&fs::read(&file.disk)?);
        let mut forms = String::new();
        for (suffix, name) in VARIANTS {
            let variant = variants
                .iter()
                .find(|(base, variant)| *base == file.url && variant.url.ends_with(suffix));
            let _written = match variant {
                Some((_base, variant)) => write!(
                    forms,
                    "{name}: Some(Variant {{ bytes: include_bytes!({:?}), etag: {:?} }}), ",
                    variant.disk.display().to_string(),
                    format!("\"{digest}-{}\"", suffix.trim_start_matches('.')),
                ),
                None => write!(forms, "{name}: None, "),
            };
        }
        let _written = writeln!(
            out,
            "    File {{ path: {:?}, kind: {:?}, bytes: include_bytes!({:?}), etag: {:?}, \
             hashed: {}, {forms}}},",
            file.url,
            kind(&file.url),
            file.disk.display().to_string(),
            format!("\"{digest}\""),
            hashed(&file.url),
        );
    }
    out.push_str("];\n");
    let _written = writeln!(
        out,
        "/// Whether `FILES` is the real bundle (else the placeholder page).\n\
         pub(crate) const BUNDLED: bool = {bundled};"
    );
    Ok(out)
}

/// `csp.rs`: the macro giving `script-src`'s source for the index's
/// import map (` 'sha256-<base64>'`), or nothing when it has none.
fn import_map_source(index: &str) -> Result<String, String> {
    let open = "<script type=\"importmap\">";
    let source = match index.split_once(open) {
        None => String::new(),
        Some((_before, rest)) => {
            let (map, _after) = rest
                .split_once("</script>")
                .ok_or("the index's import map is not closed")?;
            if rest.contains(open) {
                return Err("the index has more than one import map".to_owned());
            }
            let digest = Sha256::digest(map.as_bytes());
            format!(
                " 'sha256-{}'",
                base64::engine::general_purpose::STANDARD.encode(digest)
            )
        }
    };
    Ok(format!(
        "/// `script-src`'s source for the index's import map, generated by\n\
         /// `build.rs`; empty when the index has none.\n\
         macro_rules! import_map_source {{\n    () => {{\n        {source:?}\n    }};\n}}\n"
    ))
}

/// A file's `Content-Type`, by its extension.
fn kind(url: &str) -> &'static str {
    let extension = url.rsplit_once('.').map_or("", |(_, extension)| extension);
    match extension {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Write `content` only when it differs, so an unchanged table does not
/// make the crate compile again.
fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|old| old == content) {
        return Ok(());
    }
    fs::write(path, content)
}
