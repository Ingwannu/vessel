//! `SHA256SUMS` and its signature for one label's directory: what a
//! person checks a download against by hand, as `vesseld` releases have
//! (ADR 0008). The app does not read it; it checks the signed manifest.
//!
//! The format is `sha256sum`'s (`<hex>  <name>`, sorted by name), so
//! `sha256sum -c SHA256SUMS` works in the directory, and the signature is
//! a plain minisign file, so `minisign -Vm SHA256SUMS -P <key>` works.

use std::{fmt::Write as _, path::Path};

use vessel_app_updater::Label;

use crate::{FeedError, io, publish::hash_file, read_secret};

const SUMS: &str = "SHA256SUMS";
const SIGNATURE: &str = "SHA256SUMS.minisig";

/// Write `SHA256SUMS` over every regular file in `dir` (but the two this
/// writes), and sign it with the key at `key`, the trusted comment naming
/// `label`.
///
/// The label is given, not read from `dir`'s name: `build.sh` signs the
/// set in its hidden staging directory before renaming it into place, and
/// labels up to 0.0.0.0.7 carry that directory's name in their comment
/// ("vessel-app .0.0.0.0.7.partial.<pid> SHA256SUMS"), which anyone who
/// verifies sees. Published labels are never rebuilt, so they keep it.
pub(crate) fn write(key: &Path, label: &Label, dir: &Path) -> Result<String, FeedError> {
    let pair = read_secret(key)?;
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io(format!("reading {}", dir.display())))? {
        let entry = entry.map_err(io(format!("reading {}", dir.display())))?;
        let is_file = entry
            .file_type()
            .map_err(io(format!("reading {}", dir.display())))?
            .is_file();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|raw| FeedError::Value(format!("{} is not UTF-8", raw.to_string_lossy())))?;
        let ours = name == SUMS || name == SIGNATURE || name.starts_with('.');
        if is_file && !ours {
            names.push(name);
        }
    }
    names.sort();
    let mut sums = String::new();
    for name in &names {
        let (_size, digest) = hash_file(&dir.join(name))?;
        let _infallible = writeln!(sums, "{digest}  {name}");
    }
    let trusted = format!("vessel-app {label} SHA256SUMS");
    let signature = minisign::sign(
        Some(&pair.pk),
        &pair.sk,
        sums.as_bytes(),
        Some(&trusted),
        None,
    )
    .map_err(|source| FeedError::Key {
        what: "signing SHA256SUMS",
        source,
    })?
    .into_string();
    let sums_path = dir.join(SUMS);
    std::fs::write(&sums_path, &sums).map_err(io(format!("writing {}", sums_path.display())))?;
    let signature_path = dir.join(SIGNATURE);
    std::fs::write(&signature_path, signature)
        .map_err(io(format!("writing {}", signature_path.display())))?;
    Ok(format!(
        "{} file(s) in {} and {SIGNATURE}",
        names.len(),
        sums_path.display()
    ))
}

/// Check `SHA256SUMS.minisig` in `dir` against `SHA256SUMS` with the
/// release key's public half, and every listed file against its sum.
pub(crate) fn check(public: &str, dir: &Path) -> Result<String, FeedError> {
    let key = crate::publish::public_key(public)?;
    let sums_path = dir.join(SUMS);
    let sums = std::fs::read_to_string(&sums_path)
        .map_err(io(format!("reading {}", sums_path.display())))?;
    let signature_path = dir.join(SIGNATURE);
    let signature = std::fs::read_to_string(&signature_path)
        .map_err(io(format!("reading {}", signature_path.display())))?;
    key.verify(sums.as_bytes(), &signature)
        .map_err(|error| FeedError::Check(format!("{SIGNATURE}: {error}")))?;
    let mut count: usize = 0;
    for line in sums.lines() {
        let (digest, name) = line
            .split_once("  ")
            .ok_or_else(|| FeedError::Check(format!("{SUMS}: not a sum line: {line}")))?;
        let (_size, actual) = hash_file(&dir.join(name))?;
        if actual.to_string() != digest {
            return Err(FeedError::Check(format!("{name} does not match {SUMS}")));
        }
        count = count.saturating_add(1);
    }
    Ok(format!(
        "{SIGNATURE} verifies with the given key; {count} file(s) match {SUMS}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_cover_the_files_and_verify_with_the_key() {
        let keys = tempfile::tempdir().unwrap();
        crate::keygen(keys.path()).unwrap();
        let secret = keys.path().join(crate::SECRET_FILE);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.zip"), b"zip").unwrap();
        std::fs::write(dir.path().join("a.exe"), b"exe").unwrap();
        std::fs::write(dir.path().join(".partial"), b"x").unwrap();

        let label: Label = "0.0.0.0.8".parse().unwrap();
        let _first = write(&secret, &label, dir.path()).unwrap();
        // Twice: the files written the first time are not summed.
        let _second = write(&secret, &label, dir.path()).unwrap();

        let sums = std::fs::read_to_string(dir.path().join(SUMS)).unwrap();
        let names: Vec<_> = sums
            .lines()
            .map(|line| line.split("  ").nth(1).unwrap())
            .collect();
        assert_eq!(names, ["a.exe", "b.zip"]);
        let signature = std::fs::read_to_string(dir.path().join(SIGNATURE)).unwrap();
        // The comment names the label, never the directory the set is in
        // (a temporary one while build.sh stages it).
        assert!(
            signature.contains("trusted comment: vessel-app 0.0.0.0.8 SHA256SUMS\n"),
            "{signature}"
        );
        let directory = dir.path().file_name().unwrap().to_string_lossy();
        assert!(!signature.contains(directory.as_ref()), "{signature}");
        let public = read_secret(&secret).unwrap().pk.to_base64();
        vessel_app_updater::PublicKey::from_base64(&public)
            .unwrap()
            .verify(sums.as_bytes(), &signature)
            .unwrap();
        let _line = check(&public, dir.path()).unwrap();
        std::fs::write(dir.path().join("a.exe"), b"changed").unwrap();
        assert!(matches!(
            check(&public, dir.path()),
            Err(FeedError::Check(_))
        ));
    }
}
