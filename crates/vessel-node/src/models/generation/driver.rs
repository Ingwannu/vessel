//! One title attempt from a frozen first input; routing, styles and credentials reuse their owners.

use super::super::{
    KeyError, Models,
    metadata::{self, SelectError},
};
use super::{
    attempt::{self, Attempt},
    client::{Client, ClientError},
    preparation::{Context, Lease},
    prepare::{self, PrepareError},
    source::Seed,
    text::{self, TextError},
};
use std::{path::PathBuf, sync::Arc};
use vessel_llm::client::{Endpoint, Redactor};
use vessel_proto::{GeneratedTitle, settings::MetadataSettings};
use vessel_session::{SessionHandle, TitleUpdate};
use vessel_workspace::{Git, project_file::metadata::ProjectStyles};

#[derive(Debug, thiserror::Error)]
pub(super) enum GenerateError {
    #[error("metadata route selection failed")]
    Selection(#[from] SelectError),
    #[error("metadata preparation failed")]
    Prepare(#[from] PrepareError),
    #[error("metadata credential was refused")]
    Key(#[from] KeyError),
    #[error("metadata transport failed")]
    Client(#[from] ClientError),
    #[error("metadata title was unusable")]
    Text(#[from] TextError),
    #[error("metadata content was refused")]
    Content(#[from] vessel_llm::completion::CompletionError),
    #[error("metadata proposal could not reach its original session")]
    Session(#[from] vessel_session::SessionError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    Applied,
    Kept,
    Capacity,
    Blank,
}

pub(super) async fn generate(
    models: &Arc<Models>,
    client: &Client,
    handle: &SessionHandle,
    settings: &MetadataSettings,
    seed: Seed,
    context: Context,
) -> Result<Outcome, GenerateError> {
    if !handle.accepts_generated_title(&seed.expected) {
        return Ok(Outcome::Kept);
    }
    // The existing node and session budgets also bound blocking preparation.
    let Some(lease) = Lease::acquire(models, handle, context.tracked) else {
        return Ok(Outcome::Capacity);
    };
    let catalog = models.snapshot();
    let current = handle.info().model.map(|model| model.route);
    let selected = metadata::select(&catalog, settings, current.as_ref())?;
    let styles = styles(context.cwd, context.git, &lease).await;
    let request = prepare::title(&selected, &seed.material, styles.title.as_ref())?;
    let provider = selected.provider();
    let endpoint = Endpoint {
        api: provider.flavor.api(),
        base_url: provider.base_url.clone(),
        key: lease.key(Arc::clone(models), provider.clone()).await?,
    };
    if !handle.accepts_generated_title(&seed.expected) {
        return Ok(Outcome::Kept);
    }
    let Some(title) = exchange(handle, client, &selected, request, &endpoint).await? else {
        return Ok(Outcome::Blank);
    };
    Ok(
        match handle.apply_generated_title(seed.expected, title).await? {
            TitleUpdate::Applied => Outcome::Applied,
            TitleUpdate::Kept => Outcome::Kept,
        },
    )
}

async fn exchange(
    handle: &SessionHandle,
    client: &Client,
    selected: &metadata::Selection<'_>,
    request: prepare::Prepared,
    endpoint: &Endpoint,
) -> Result<Option<GeneratedTitle>, GenerateError> {
    let redactor = Redactor::new(endpoint.key.as_ref());
    let attempt = Attempt::new(handle, selected);
    let answer = match client.complete(endpoint, request).await {
        Ok(answer) => answer,
        Err(error) => {
            attempt.failed(&error);
            return Err(error.into());
        }
    };
    let candidate = match answer.text {
        Ok(text) => text::decode(&text, &redactor).map_err(GenerateError::Text),
        Err(error) => Err(GenerateError::Content(error)),
    };
    attempt.answer(
        &answer.usage,
        answer.stop,
        candidate
            .as_ref()
            .err()
            .map(|_unusable| attempt::unusable()),
    );
    Ok(candidate?.into_title())
}

async fn styles(cwd: PathBuf, git: Option<Git>, lease: &Lease) -> ProjectStyles {
    let folder = match git {
        Some(git) => match git.toplevel(&cwd).await {
            Ok(Some(root)) => root,
            Ok(None) => cwd,
            Err(_cause) => {
                tracing::warn!(class = "git", "metadata styles use defaults");
                return ProjectStyles::default();
            }
        },
        None => cwd,
    };
    match lease.styles(folder).await {
        Ok(styles) => styles,
        Err(_cause) => {
            tracing::warn!(class = "task", "metadata styles use defaults");
            ProjectStyles::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use vessel_workspace::GitSetup;

    #[tokio::test]
    async fn checked_git_root_and_non_git_folder_choose_their_own_styles() {
        crate::models::tests::within(async {
            let fixture = crate::models::tests::Fixture::new().await;
            let (handle, records) = fixture.structured().await;
            let lease = Lease::acquire(
                &fixture.models,
                &handle,
                tokio_util::task::TaskTracker::new().token(),
            )
            .unwrap();
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("repository");
            let nested = root.join("nested");
            let hooks = temp.path().join("no-hooks");
            std::fs::create_dir_all(&nested).unwrap();
            std::fs::create_dir(&hooks).unwrap();
            let config = temp.path().join("gitconfig");
            std::fs::write(&config, "").unwrap();
            let env = vec![
                ("GIT_CONFIG_GLOBAL".to_owned(), config.into_os_string()),
                ("GIT_CONFIG_NOSYSTEM".to_owned(), OsString::from("1")),
                ("HOME".to_owned(), temp.path().as_os_str().to_owned()),
            ];
            let program = which::which("git").unwrap();
            let git = Git::probe(Some(GitSetup {
                program,
                no_hooks: hooks,
                env,
            }))
            .await
            .git
            .unwrap();
            git.init(&root).await.unwrap();
            std::fs::write(
                root.join("vessel.toml"),
                "[metadata.title]\ninstructions = 'root convention'\n",
            )
            .unwrap();
            std::fs::write(
                nested.join("vessel.toml"),
                "[metadata.title]\ninstructions = 'folder convention'\n",
            )
            .unwrap();
            assert_eq!(
                styles(nested.clone(), Some(git), &lease)
                    .await
                    .title
                    .unwrap()
                    .as_str(),
                "root convention"
            );
            assert_eq!(
                styles(nested, None, &lease).await.title.unwrap().as_str(),
                "folder convention"
            );
            drop(lease);
            handle.kill();
            let _ended = handle.ended().await;
            let _records = records.await.unwrap();
            fixture.stop().await;
        })
        .await;
    }
}
