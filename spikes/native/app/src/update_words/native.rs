//! Project updater diagnostics into safe, translated recovery choices. The
//! original `UpdateError` remains in `UpdateState`; no diagnostic becomes UI text.

use vessel_app_updater::{FetchError, InstallError, OpenError, StageError, UpdateError};
use vessel_app_view::updates::Problem;

pub(super) fn problem(error: &UpdateError) -> Problem {
    match error {
        UpdateError::Fetch(error) => fetch(error),
        UpdateError::Open(error) => open(error),
        UpdateError::NoBuild { .. } => Problem::NoBuild,
        UpdateError::Stage(error) => stage(error),
        UpdateError::Install(error) => install(error),
        UpdateError::Interrupted => Problem::Interrupted,
    }
}

fn fetch(error: &FetchError) -> Problem {
    match error {
        FetchError::Tls
        | FetchError::Unreachable(_)
        | FetchError::Status { .. }
        | FetchError::Stalled => Problem::Check,
        FetchError::TooLarge { .. } => Problem::FeedTooLarge,
    }
}

fn open(error: &OpenError) -> Problem {
    match error {
        OpenError::Bounds => Problem::FeedTooLarge,
        OpenError::Envelope(_)
        | OpenError::Manifest(_)
        | OpenError::Channel { .. }
        | OpenError::Duplicate { .. }
        | OpenError::CompanionTwice { .. } => Problem::FeedInvalid,
        OpenError::Signature(_) => Problem::FeedUnverified,
        OpenError::Schema { .. } => Problem::FeedNewFormat,
    }
}

fn stage(error: &StageError) -> Problem {
    match error {
        StageError::Fetch(_) => Problem::Download,
        StageError::Write(_) => Problem::DownloadWrite,
        // Another copy of the app holds the download; what arrived is kept
        // and the next attempt goes on from it, so this is an interruption,
        // not a connection or a write problem.
        StageError::Busy => Problem::Interrupted,
        // A signed gzip copy that does not unpack is bytes not to install.
        StageError::TooLarge
        | StageError::Short { .. }
        | StageError::Digest
        | StageError::Unpack(_) => Problem::DownloadIntegrity,
    }
}

fn install(error: &InstallError) -> Problem {
    match error {
        InstallError::MoveAside(_) | InstallError::Permissions(_) | InstallError::Replace(_) => {
            Problem::InstallWrite
        }
        InstallError::Unsupported(_) => Problem::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_updater::{
        Channel, MAX_ENVELOPE_BYTES, Packaging, PublicKey, Role, SCHEMA, Target,
    };

    fn io() -> std::io::Error {
        std::io::Error::other("isolated fixture")
    }

    #[test]
    fn each_recovery_category_is_chosen_from_typed_errors_without_io() {
        let label = "0.0.0.0.7".parse().unwrap();
        let cases = [
            (UpdateError::Fetch(FetchError::Tls), Problem::Check),
            (
                UpdateError::Fetch(FetchError::TooLarge {
                    limit: MAX_ENVELOPE_BYTES,
                }),
                Problem::FeedTooLarge,
            ),
            (
                UpdateError::Open(OpenError::Channel {
                    found: Channel::preview(),
                }),
                Problem::FeedInvalid,
            ),
            // Classification uses the typed outer signature variant, not a
            // particular crypto parser's diagnostic or error text.
            (
                UpdateError::Open(OpenError::Signature(
                    PublicKey::from_base64("invalid fixture").unwrap_err(),
                )),
                Problem::FeedUnverified,
            ),
            (
                UpdateError::Open(OpenError::Schema {
                    found: SCHEMA.saturating_add(1),
                }),
                Problem::FeedNewFormat,
            ),
            (
                UpdateError::NoBuild {
                    latest: label,
                    target: Target::LinuxX86_64,
                    packaging: Packaging::Executable,
                },
                Problem::NoBuild,
            ),
            (
                UpdateError::Stage(StageError::Fetch(FetchError::Tls)),
                Problem::Download,
            ),
            (
                UpdateError::Stage(StageError::Write(io())),
                Problem::DownloadWrite,
            ),
            (
                UpdateError::Stage(StageError::Digest),
                Problem::DownloadIntegrity,
            ),
            (
                UpdateError::Install(InstallError::Replace(io())),
                Problem::InstallWrite,
            ),
            (
                UpdateError::Install(InstallError::Unsupported(Packaging::AppBundle)),
                Problem::Unsupported,
            ),
            (UpdateError::Interrupted, Problem::Interrupted),
        ];
        for (error, expected) in cases {
            assert_eq!(problem(&error), expected);
        }
    }

    #[test]
    fn shared_recovery_keeps_constructible_alternate_variants() {
        for error in [
            FetchError::Tls,
            FetchError::Stalled,
            FetchError::Status {
                status: 503_u16.try_into().unwrap(),
            },
        ] {
            assert_eq!(fetch(&error), Problem::Check);
        }
        assert_eq!(open(&OpenError::Bounds), Problem::FeedTooLarge);
        assert_eq!(
            open(&OpenError::Duplicate {
                target: Target::LinuxX86_64,
                packaging: Packaging::Executable
            }),
            Problem::FeedInvalid
        );
        assert_eq!(
            open(&OpenError::CompanionTwice {
                target: Target::LinuxX86_64,
                role: Role::Node
            }),
            Problem::FeedInvalid
        );
        for error in [
            StageError::TooLarge,
            StageError::Short {
                received: 1,
                expected: 2,
            },
            StageError::Digest,
            StageError::Unpack(io()),
        ] {
            assert_eq!(stage(&error), Problem::DownloadIntegrity);
        }
        assert_eq!(stage(&StageError::Busy), Problem::Interrupted);
        for error in [
            InstallError::MoveAside(io()),
            InstallError::Permissions(io()),
            InstallError::Replace(io()),
        ] {
            assert_eq!(install(&error), Problem::InstallWrite);
        }
    }
}
