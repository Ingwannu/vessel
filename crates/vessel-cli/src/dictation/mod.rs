//! Explicit file client: validate before connecting, drain updates, print only an accepted Final.
//!
//! Capture, resampling and recognition belong to their existing owners. This
//! command keeps one connection and never restarts or resends failed audio.
mod client;
mod driver;
mod error;
mod file;
mod send;

use client::Client;
use error::Failure;
use std::path::Path;
use vessel_proto::dictation::DictationId;

/// Main's source-chain renderer receives only a fixed speech diagnostic.
pub(crate) use error::DictateError;

pub(crate) async fn run(
    target: &crate::args::Target,
    path: &Path,
) -> Result<(), crate::error::CliError> {
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    let input = tokio::select! {
        biased;
        result = &mut interrupt => return Err(DictateError::new(driver::interrupted(result)).into()),
        result = file::FileInput::open(path) => result.map_err(DictateError::from)?,
    };
    let connected = tokio::select! {
        biased;
        result = &mut interrupt => return Err(DictateError::new(driver::interrupted(result)).into()),
        result = crate::connect::connect(target) => result?,
    };
    let client = Client::Node(connected.client);
    let id = DictationId::random();
    let text = driver::transcribe(&client, &id, input, interrupt.as_mut()).await?;
    let final_line = format!("{}\n", text.as_str());
    if let Err(source) = crate::io::out(final_line.as_bytes()).await {
        return Err(
            driver::cancel_after_failure(&client, &id, Failure::Output(source))
                .await
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn invalid_input_is_refused_before_resolving_a_node() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("invalid.wav");
        tokio::fs::write(&path, b"not a wave").await.unwrap();
        let target = crate::args::Target {
            node: Some("this is not a node address".to_owned()),
            token: None,
            data_dir: Some(root.path().join("data")),
            config_dir: Some(root.path().join("config")),
        };
        let error = super::run(&target, &path).await.unwrap_err();
        assert!(matches!(error, crate::error::CliError::Dictation(_)));
        assert!(!root.path().join("data").exists());
        assert!(!root.path().join("config").exists());
    }
}
