//! Isolated synthetic secrets only; no machine credentials or external provider.

use super::*;
use crate::models::config;
use std::error::Error;
use vessel_store::DataDir;

fn provider() -> Provider {
    config::parse(
        r#"
[providers.fixture]
api = "anthropic-messages"
base_url = "https://not-called.invalid/v1"
key = { secret = "FIXTURE_KEY" }
[models.fixture]
provider = "fixture"
model = "fixture-model"
"#,
    )
    .unwrap()
    .get(&"fixture".parse().unwrap())
    .unwrap()
    .1
    .clone()
}

#[tokio::test]
async fn missing_plain_and_keyless_paths_keep_the_policy_without_echoing_values() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    let models = Models::for_tests(&dir);
    let provider = provider();
    assert!(matches!(
        models.key(&provider).await,
        Err(KeyError::Missing(_))
    ));
    crate::fleet::set_secret(&models.files, &"FIXTURE_KEY".parse().unwrap(), "plain").unwrap();
    let error = models.key(&provider).await.unwrap_err();
    assert!(matches!(error, KeyError::Plain(_)));
    assert!(!format!("{error:?}").contains("\"plain\""));
    std::fs::write(models.files.secrets(), "not valid TOML = ").unwrap();
    let mut keyless = provider;
    keyless.key = None;
    assert!(models.key(&keyless).await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_names_are_typed_but_cannot_escape_the_error_chain() {
    let temp = tempfile::tempdir().unwrap();
    let dir = DataDir::new(temp.path());
    let models = Models::for_tests(&dir);
    std::fs::write(
        models.files.secrets(),
        "\"private malformed marker\" = true",
    )
    .unwrap();
    let error = models.key(&provider()).await.unwrap_err();
    assert!(matches!(error, KeyError::Read(_)));
    assert!(!format!("{error} {error:?}").contains("private malformed marker"));
    assert!(error.source().unwrap().source().is_none());
}
