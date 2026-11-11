//! Which embedded files are named by their content's hash: `build.rs`
//! marks them, and `files` serves them as immutable. Shared by the build
//! script (`#[path]`) and the crate's tests, so the rule is tested where
//! it is used.

/// Whether `url` names a file that changes name when its content does.
///
/// Vite writes every file under `assets/` as `<name>-<hash>.<ext>`, the
/// hash eight characters of URL-safe base64. Both the place and the shape
/// are checked, so a file put there by hand, or a build configured
/// without hashes, is not served as immutable (it would then stay stale
/// until the browser dropped it). A name that merely ends in a dash and
/// eight letters would pass; Vite never writes one without a hash there.
pub(crate) fn hashed(url: &str) -> bool {
    let Some(file) = url.strip_prefix("/assets/") else {
        return false;
    };
    if file.contains('/') {
        return false;
    }
    let stem = file
        .rsplit_once('.')
        .map_or(file, |(stem, _extension)| stem);
    let bytes = stem.as_bytes();
    let Some(cut) = bytes.len().checked_sub(9) else {
        return false;
    };
    let (name, hash) = bytes.split_at(cut);
    !name.is_empty()
        && hash.first() == Some(&b'-')
        && hash
            .iter()
            .skip(1)
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::hashed;

    #[test]
    fn only_assets_named_with_a_hash_are_hashed() {
        for name in [
            "/assets/index-B3kq_x9Z.js",
            "/assets/index-a-b_c-d1.css",
            "/assets/Geist-Variable-Ab12Cd34.woff2",
        ] {
            assert!(hashed(name), "{name}");
        }
        for name in [
            "/index.html",
            "/licenses/pretendard.txt",
            "/assets/Geist.woff2",
            "/assets/index.js",
            "/assets/-abcdefgh.js",
            "/assets/index-abcdefg.js",
            "/assets/index-abc.efgh.js",
            "/assets/sub/index-abcdefgh.js",
            "/elsewhere/index-abcdefgh.js",
        ] {
            assert!(!hashed(name), "{name}");
        }
    }
}
