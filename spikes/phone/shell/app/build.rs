//! Tauri's build step (the app's configuration and access control, read
//! from `tauri.conf.json`), and the build's display label.
//!
//! [Decision Log]
//! - Purpose: the owner installs a build, uses it and sends feedback; each
//!   build carries a label the coordinator gives out (`0.0.0.0.1`, then
//!   `0.0.0.0.2`, …) and the app shows it, so feedback names the build.
//! - Constraint: Tauri's and Android's version fields are semantic
//!   versions and integers; a five-part label is neither.
//! - Choice: the label comes from `VESSEL_APP_LABEL` at build time
//!   (checked here: numbers and dots only, so it can be shown and put into
//!   Android's `versionName` as it is), and is `dev` when unset. Android's
//!   `versionCode`, which must grow with every build a phone is to accept
//!   as an update, is the Gradle project's to derive (see
//!   `gen/android/app/build.gradle.kts`).

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut cargo = String::from("cargo::rerun-if-env-changed=VESSEL_APP_LABEL\n");
    let label = std::env::var("VESSEL_APP_LABEL").unwrap_or_else(|_unset| "dev".to_owned());
    let valid = label == "dev"
        || (!label.is_empty()
            && label.split('.').all(|part| {
                !part.is_empty() && part.len() <= 9 && part.bytes().all(|byte| byte.is_ascii_digit())
            }));
    if !valid {
        let _said = writeln!(
            std::io::stdout().lock(),
            "cargo::error=VESSEL_APP_LABEL is numbers and dots (0.0.0.0.1), not {label:?}"
        );
        return ExitCode::FAILURE;
    }
    let _written = writeln!(cargo, "cargo::rustc-env=VESSEL_APP_LABEL={label}");
    let _said = std::io::stdout().lock().write_all(cargo.as_bytes());
    tauri_build::build();
    ExitCode::SUCCESS
}
