//! Embed the shipped directory catalogue so `all` grows with the asset tree.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    let root = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| io::Error::other("missing manifest directory"))?,
    )
    .join("skills");
    let output = PathBuf::from(
        std::env::var_os("OUT_DIR")
            .ok_or_else(|| io::Error::other("missing build output directory"))?,
    )
    .join("skills.rs");
    let mut files = Vec::new();
    collect(&root, &root, &mut files)?;
    files.sort();
    let mut source = String::from("pub(super) const FILES: &[(&str, &[u8])] = &[\n");
    for relative in files {
        let absolute = root.join(&relative);
        let relative = relative
            .to_str()
            .ok_or_else(|| io::Error::other("skill path is not UTF-8"))?
            .replace('\\', "/");
        let absolute = absolute
            .to_str()
            .ok_or_else(|| io::Error::other("skill source path is not UTF-8"))?;
        // Debug escaping is required because these are Rust string literals.
        writeln!(&mut source, "({relative:?}, include_bytes!({absolute:?})),")
            .map_err(io::Error::other)?;
    }
    source.push_str("];\n");
    fs::write(output, source)?;
    writeln!(io::stdout().lock(), "cargo:rerun-if-changed=skills")
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for item in fs::read_dir(dir)? {
        let item = item?;
        let kind = item.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::other("bundled skills cannot contain links"));
        }
        if kind.is_dir() {
            collect(root, &item.path(), files)?;
        } else if kind.is_file() {
            files.push(
                item.path()
                    .strip_prefix(root)
                    .map_err(io::Error::other)?
                    .to_path_buf(),
            );
        } else {
            return Err(io::Error::other("bundled skills must be regular files"));
        }
    }
    Ok(())
}
