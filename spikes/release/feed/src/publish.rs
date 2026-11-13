//! Writing a channel's signed manifest, and reading it back as the app
//! does.
//!
//! The order matters for anyone reading the feed while it is written:
//! the artifacts are already in place (the release script copies them
//! first), then the manifest is written to a temporary name beside its
//! final one and renamed over it. A reader sees the old manifest, whose
//! artifacts are all still there, or the new one, whose artifacts are all
//! already there. Before the rename, the envelope is opened with the
//! app's own code, so a feed the app would refuse is never published.

use std::{
    io::Read as _,
    path::{Path, PathBuf},
};

use sha2::Digest as _;
use vessel_app_updater::{
    Artifact, ArtifactPath, ByteCount, Channel, Envelope, Manifest, Notes, PublicKey, Published,
    SCHEMA, Sha256,
};

use crate::{FeedError, args::Publish, io, read_secret};

/// Write `<feed>/<channel>.json` for `publish`; the line returned says
/// what was published.
pub(crate) fn publish(publish: &Publish) -> Result<String, FeedError> {
    let pair = read_secret(&publish.key)?;
    let notes =
        match &publish.notes {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(io(format!("reading {}", path.display())))?;
                Some(Notes::try_from(text).map_err(|_long| {
                    FeedError::Value("the notes are longer than 8 KiB".to_owned())
                })?)
            }
            None => None,
        };
    let published = Published::try_from(publish.published.clone().unwrap_or_else(now))
        .map_err(|_long| FeedError::Value("--published is longer than 64 bytes".to_owned()))?;
    let artifacts = publish
        .artifacts
        .iter()
        .map(|artifact| {
            let (size, sha256) = measure(&publish.feed, &artifact.path)?;
            Ok(Artifact {
                target: artifact.target,
                packaging: artifact.packaging,
                path: artifact.path.clone(),
                size,
                sha256,
            })
        })
        .collect::<Result<Vec<_>, FeedError>>()?;
    let manifest = Manifest {
        schema: SCHEMA,
        channel: publish.channel.clone(),
        label: publish.label.clone(),
        published,
        notes,
        artifacts,
    };
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|error| FeedError::Value(format!("the manifest does not serialise: {error}")))?;
    let trusted = format!(
        "vessel-app channel={} label={}",
        publish.channel, publish.label
    );
    let signature = minisign::sign(
        Some(&pair.pk),
        &pair.sk,
        text.as_bytes(),
        Some(&trusted),
        None,
    )
    .map_err(|source| FeedError::Key {
        what: "signing the manifest",
        source,
    })?
    .into_string();
    let envelope = serde_json::to_vec_pretty(&Envelope {
        manifest: text,
        signature,
    })
    .map_err(|error| FeedError::Value(format!("the envelope does not serialise: {error}")))?;

    let public = PublicKey::from_base64(&pair.pk.to_base64())
        .map_err(|error| FeedError::Check(error.to_string()))?;
    let opened = Manifest::open(&envelope, &public, &publish.channel)
        .map_err(|error| FeedError::Check(error.to_string()))?;
    if opened != manifest {
        return Err(FeedError::Check(
            "the manifest read back differs".to_owned(),
        ));
    }

    let path = publish.feed.join(format!("{}.json", publish.channel));
    let temporary = publish.feed.join(format!(".{}.json.tmp", publish.channel));
    std::fs::write(&temporary, &envelope)
        .map_err(io(format!("writing {}", temporary.display())))?;
    std::fs::rename(&temporary, &path).map_err(io(format!("renaming onto {}", path.display())))?;
    Ok(format!(
        "published {} on {} with {} artifact(s): {}",
        publish.label,
        publish.channel,
        manifest.artifacts.len(),
        path.display()
    ))
}

/// Read `<feed>/<channel>.json` with `public` as the app does, and check
/// every artifact it names is under the feed with its size and digest.
pub(crate) fn check(feed: &Path, public: &str, channel: &Channel) -> Result<String, FeedError> {
    let public = public_key(public)?;
    let path = feed.join(format!("{channel}.json"));
    let envelope = std::fs::read(&path).map_err(io(format!("reading {}", path.display())))?;
    let manifest = Manifest::open(&envelope, &public, channel)
        .map_err(|error| FeedError::Check(error.to_string()))?;
    let mut lines = vec![format!(
        "{} on {} ({}), signed by the given key",
        manifest.label,
        manifest.channel,
        manifest.published.as_str()
    )];
    for artifact in &manifest.artifacts {
        let (size, sha256) = measure(feed, &artifact.path)?;
        if size != artifact.size || sha256 != artifact.sha256 {
            return Err(FeedError::Check(format!(
                "{} is not the file the manifest names",
                artifact.path
            )));
        }
        lines.push(format!(
            "  {} {}: {} ({} bytes) ok",
            artifact.target,
            artifact.packaging,
            artifact.path,
            size.get()
        ));
    }
    Ok(lines.join("\n"))
}

/// A public key given as its base64 line or as a `.pub` file's path.
fn public_key(given: &str) -> Result<PublicKey, FeedError> {
    if let Ok(key) = PublicKey::from_base64(given) {
        return Ok(key);
    }
    let text = std::fs::read_to_string(given).map_err(io(format!("reading the key {given}")))?;
    let line = text
        .lines()
        .find(|line| !line.starts_with("untrusted comment:") && !line.trim().is_empty())
        .unwrap_or_default();
    PublicKey::from_base64(line).map_err(|error| FeedError::Value(error.to_string()))
}

/// Size and SHA-256 of the file at `path` under `feed`, read in pieces.
pub(crate) fn measure(feed: &Path, path: &ArtifactPath) -> Result<(ByteCount, Sha256), FeedError> {
    let file: PathBuf = path
        .as_str()
        .split('/')
        .fold(feed.to_path_buf(), |dir, name| dir.join(name));
    let (size, sha256) = hash_file(&file)?;
    let size = ByteCount::try_from(size)
        .map_err(|error| FeedError::Value(format!("{}: {error}", file.display())))?;
    Ok((size, sha256))
}

/// Size and SHA-256 of a file, read in 64 KiB pieces.
pub(crate) fn hash_file(file: &Path) -> Result<(u64, Sha256), FeedError> {
    let mut reader =
        std::fs::File::open(file).map_err(io(format!("opening {}", file.display())))?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut size: u64 = 0;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(io(format!("reading {}", file.display())))?;
        let Some(piece) = buffer.get(..read).filter(|piece| !piece.is_empty()) else {
            break;
        };
        hasher.update(piece);
        size = size.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
    }
    Ok((size, Sha256::from_bytes(hasher.finalize().into())))
}

/// Now, in RFC 3339 at UTC, to the second.
fn now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    rfc3339(seconds)
}

/// Seconds since 1970 as `YYYY-MM-DDTHH:MM:SSZ` (Howard Hinnant's
/// days-to-civil; no date crate for one timestamp).
fn rfc3339(seconds: u64) -> String {
    let days = i64::try_from(seconds.checked_div(86_400).unwrap_or(0)).unwrap_or(0);
    let of_day = seconds.checked_rem(86_400).unwrap_or(0);
    let (hour, minute, second) = (
        of_day.checked_div(3600).unwrap_or(0),
        of_day
            .checked_rem(3600)
            .and_then(|rest| rest.checked_div(60))
            .unwrap_or(0),
        of_day.checked_rem(60).unwrap_or(0),
    );
    let (year, month, day) = civil(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil(days: i64) -> (i64, i64, i64) {
    let z = days.saturating_add(719_468);
    let era = z.checked_div_euclid(146_097).unwrap_or(0);
    let doe = z.saturating_sub(era.saturating_mul(146_097));
    let yoe = doe
        .saturating_sub(doe.checked_div(1460).unwrap_or(0))
        .saturating_add(doe.checked_div(36_524).unwrap_or(0))
        .saturating_sub(doe.checked_div(146_096).unwrap_or(0))
        .checked_div(365)
        .unwrap_or(0);
    let doy = doe.saturating_sub(
        yoe.saturating_mul(365)
            .saturating_add(yoe.checked_div(4).unwrap_or(0))
            .saturating_sub(yoe.checked_div(100).unwrap_or(0)),
    );
    let mp = doy
        .saturating_mul(5)
        .saturating_add(2)
        .checked_div(153)
        .unwrap_or(0);
    let day = doy
        .saturating_sub(
            mp.saturating_mul(153)
                .saturating_add(2)
                .checked_div(5)
                .unwrap_or(0),
        )
        .saturating_add(1);
    let month = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let year = yoe
        .saturating_add(era.saturating_mul(400))
        .saturating_add(i64::from(month <= 2));
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::ArtifactArg;
    use vessel_app_updater::{Packaging, Target};

    #[test]
    fn dates_are_written_as_rfc3339() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_794_537_000), "2026-11-13T02:30:00Z");
    }

    #[test]
    fn a_published_feed_checks_and_a_changed_artifact_does_not() {
        let keys = tempfile::tempdir().unwrap();
        crate::keygen(&keys.path().join("k")).unwrap();
        let secret = keys.path().join("k").join(crate::SECRET_FILE);
        let feed = tempfile::tempdir().unwrap();
        std::fs::create_dir(feed.path().join("0.0.0.0.2")).unwrap();
        std::fs::write(feed.path().join("0.0.0.0.2/vessel.exe"), b"build").unwrap();
        let request = Publish {
            key: secret.clone(),
            feed: feed.path().to_path_buf(),
            channel: Channel::preview(),
            label: "0.0.0.0.2".parse().unwrap(),
            notes: None,
            published: Some("2026-11-13T00:00:00Z".to_owned()),
            artifacts: vec![ArtifactArg {
                target: Target::WindowsX86_64,
                packaging: Packaging::Executable,
                path: "0.0.0.0.2/vessel.exe".parse().unwrap(),
            }],
        };

        let _line = publish(&request).unwrap();

        let public = read_secret(&secret).unwrap().pk.to_base64();
        let checked = check(feed.path(), &public, &Channel::preview()).unwrap();
        assert!(checked.contains("0.0.0.0.2 on preview"), "{checked}");
        assert!(check(feed.path(), &public, &"stable".parse().unwrap()).is_err());

        std::fs::write(feed.path().join("0.0.0.0.2/vessel.exe"), b"other").unwrap();
        assert!(matches!(
            check(feed.path(), &public, &Channel::preview()),
            Err(FeedError::Check(_))
        ));
    }

    #[test]
    fn keygen_refuses_to_replace_a_key() {
        let keys = tempfile::tempdir().unwrap();
        crate::keygen(keys.path()).unwrap();
        assert!(matches!(
            crate::keygen(keys.path()),
            Err(FeedError::Exists(_))
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let secret = std::fs::metadata(keys.path().join(crate::SECRET_FILE)).unwrap();
            assert_eq!(secret.permissions().mode() & 0o777, 0o600);
            let dir = std::fs::metadata(keys.path()).unwrap();
            assert_eq!(dir.permissions().mode() & 0o777, 0o700);
        }
    }
}
