//! Opt-in synthetic launch-payload audit. No real agent/account is involved.
use std::io::Write as _;

pub(super) fn record(params: &serde_json::Value) -> std::io::Result<()> {
    let Some(path) = std::env::var_os("FAKE_CODEX_LAUNCH_AUDIT") else {
        return Ok(());
    };
    let mut options = std::fs::OpenOptions::new();
    let _ = options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let _ = options.mode(0o600);
    }
    let mut file = options.open(path)?;
    serde_json::to_writer(&mut file, params)?;
    file.write_all(b"\n")
}
