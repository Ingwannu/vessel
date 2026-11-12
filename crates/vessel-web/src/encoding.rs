//! Which of a file's encodings a request gets: its `Accept-Encoding`
//! against the variants the build made (`build.rs`, `web/scripts/
//! compress.mjs`).
//!
//! [Decision Log] (milestone 11, track P, `docs/PERFORMANCE.md`)
//! - Purpose: the page's script was 1,147 KB on the wire to every client,
//!   whatever it accepted; through a tunnel or a phone's link that is
//!   most of the first load.
//! - Constraint: nothing is compressed per request (the server serves the
//!   same bytes to every tab; compressing them again for each is CPU for
//!   nothing), and nothing new runs before the hello.
//! - Alternatives: `tower-http`'s compression layer (per request, and a
//!   dependency); compressing in `build.rs` with Rust crates (`flate2` and
//!   `brotli` as build dependencies, compiled on every Rust build and run
//!   unoptimised there: seconds of brotli at its best quality each time the
//!   bundle changes).
//! - Choice: the web build writes each text file's brotli and gzip forms
//!   beside it with Node's own zlib (no dependency); `build.rs` embeds them
//!   with the file and refuses one that was not made from the file it sits
//!   beside. This module picks one per request.
//! - The rule: the acceptable encoding with the highest weight (RFC 9110
//!   §12.5.3: a coding is acceptable when named, or covered by `*`, with a
//!   weight above zero); on a tie brotli (smaller) before gzip. Identity
//!   otherwise, even when the client refuses it: an uncompressed file
//!   beats a `406` for a page.

/// A content coding a file can be served in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    /// The file as it is.
    Identity,
    /// `gzip`.
    Gzip,
    /// `br`.
    Brotli,
}

impl Encoding {
    /// Its `Content-Encoding` token; `None` for identity, which sends none.
    pub(crate) const fn token(self) -> Option<&'static str> {
        match self {
            Self::Identity => None,
            Self::Gzip => Some("gzip"),
            Self::Brotli => Some("br"),
        }
    }
}

/// A weight from `0` to `1000` (`q=0.5` is `500`), compared as integers.
type Weight = u16;

/// The encoding to send for `accept` (the request's `Accept-Encoding`, if
/// any), among the variants the file has.
pub(crate) fn choose(accept: Option<&str>, gzip: bool, brotli: bool) -> Encoding {
    let Some(accept) = accept else {
        return Encoding::Identity;
    };
    let brotli = brotli.then(|| weight(accept, &["br"])).flatten();
    let gzip = gzip.then(|| weight(accept, &["gzip", "x-gzip"])).flatten();
    match (brotli, gzip) {
        (Some(brotli), Some(gzip)) if brotli >= gzip => Encoding::Brotli,
        (Some(_brotli), None) => Encoding::Brotli,
        (_, Some(_gzip)) => Encoding::Gzip,
        (None, None) => Encoding::Identity,
    }
}

/// The weight `accept` gives the coding named by any of `names`, when it
/// is above zero: its own entry's if listed, else `*`'s.
fn weight(accept: &str, names: &[&str]) -> Option<Weight> {
    let mut listed = None;
    let mut any = None;
    for entry in accept.split(',') {
        let mut parts = entry.split(';');
        let coding = parts.next().unwrap_or_default().trim();
        let quality = parts.find_map(|parameter| {
            let (key, value) = parameter.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("q")
                .then(|| quality(value.trim()))
        });
        // A malformed weight counts as none rather than as the default.
        let quality = quality.unwrap_or(Some(1000)).unwrap_or(0);
        if names.iter().any(|name| coding.eq_ignore_ascii_case(name)) {
            listed = Some(listed.map_or(quality, |seen: Weight| seen.max(quality)));
        } else if coding == "*" {
            any = Some(quality);
        }
    }
    listed.or(any).filter(|weight| *weight > 0)
}

/// `q`'s value (`0`, `0.8`, `1.000`) in thousandths; `None` if malformed.
fn quality(value: &str) -> Option<Weight> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let thousandths = fraction
        .bytes()
        .chain(std::iter::repeat(b'0'))
        .take(3)
        .try_fold(0_u16, |sum, digit| {
            sum.checked_mul(10)?
                .checked_add(u16::from(digit.checked_sub(b'0')?))
        })?;
    match whole {
        "0" => Some(thousandths),
        "1" if thousandths == 0 => Some(1000),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_browser_gets_brotli_and_an_old_client_gzip() {
        let chrome = Some("gzip, deflate, br, zstd");
        assert_eq!(choose(chrome, true, true), Encoding::Brotli);
        assert_eq!(choose(chrome, true, false), Encoding::Gzip);
        assert_eq!(choose(chrome, false, false), Encoding::Identity);
        assert_eq!(choose(Some("gzip"), true, true), Encoding::Gzip);
        assert_eq!(choose(Some("x-gzip"), true, true), Encoding::Gzip);
        assert_eq!(choose(None, true, true), Encoding::Identity);
        assert_eq!(choose(Some(""), true, true), Encoding::Identity);
    }

    #[test]
    fn weights_decide_and_zero_refuses() {
        assert_eq!(choose(Some("br;q=0, gzip"), true, true), Encoding::Gzip);
        assert_eq!(
            choose(Some("br;q=0.5, gzip;q=0.8"), true, true),
            Encoding::Gzip
        );
        assert_eq!(
            choose(Some("br;q=0.8, gzip;q=0.8"), true, true),
            Encoding::Brotli
        );
        assert_eq!(choose(Some("BR;Q=1.0"), true, true), Encoding::Brotli);
        assert_eq!(choose(Some("*"), true, true), Encoding::Brotli);
        assert_eq!(choose(Some("*;q=0, gzip"), true, true), Encoding::Gzip);
        assert_eq!(choose(Some("*;q=0"), true, true), Encoding::Identity);
        assert_eq!(choose(Some("identity"), true, true), Encoding::Identity);
        // Malformed weights are no weight at all.
        assert_eq!(choose(Some("br;q=2"), true, true), Encoding::Identity);
        assert_eq!(choose(Some("br;q=0.12345"), true, true), Encoding::Identity);
        assert_eq!(choose(Some("br;q=x, gzip"), true, true), Encoding::Gzip);
    }

    #[test]
    fn quality_reads_thousandths() {
        assert_eq!(quality("1"), Some(1000));
        assert_eq!(quality("1.000"), Some(1000));
        assert_eq!(quality("0.5"), Some(500));
        assert_eq!(quality("0.05"), Some(50));
        assert_eq!(quality("0"), Some(0));
        assert_eq!(quality("1.5"), None);
        assert_eq!(quality("-1"), None);
        assert_eq!(quality("0.-1"), None);
    }
}
