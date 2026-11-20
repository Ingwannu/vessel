//! Where the app's logs go: logcat on Android (tag `vessel`), standard
//! error elsewhere. `RUST_LOG` is not read on a phone (there is no
//! environment to set); the level is `info` for the app and the bridge,
//! `warn` for the rest.
//!
//! Nothing secret is logged: the launch address and pairing links are
//! never passed to `tracing` (see `window` and `vessel-phone-core`).

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

/// The filter: the app and Vessel's crates at `info`, the rest at `warn`.
const FILTER: &str = "warn,vessel_phone=info,vessel_phone_core=info,vessel_web=info,vessel_client=info";

/// Install the subscriber once; a second call (the activity made again in
/// the same process) changes nothing.
pub(crate) fn init() {
    let filter = EnvFilter::new(FILTER);
    #[cfg(target_os = "android")]
    let installed = tracing_logcat::LogcatMakeWriter::new(
        tracing_logcat::LogcatTag::Fixed("vessel".to_owned()),
    )
    .map(|writer| {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .with_ansi(false)
                    .without_time(),
            )
            .try_init()
            .is_ok()
    })
    .unwrap_or(false);
    #[cfg(not(target_os = "android"))]
    let installed = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .try_init()
        .is_ok();
    if !installed {
        tracing::debug!("logging was already set up");
    }
}
