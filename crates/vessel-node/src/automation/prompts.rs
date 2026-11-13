//! Saved prompts (protocol 11, ADR 0033): the node's rules for them, a
//! name unique within its project (or the machine's), at most
//! [`SavedPrompt::MAX`], a change made from the revision it names. Who may
//! write one and whether its `session` holds together are the service's
//! checks (`service/prompts.rs`), made before these.
//!
//! [Decision Log]
//! - Purpose: a person's reusable text, kept by the node; the page, the
//!   CLI and agents (who may read, not write) see the same list.
//! - Constraint: the store refuses a second name in a scope (a `UNIQUE`
//!   index) but cannot say so as `conflict`, and the count of 256 is not a
//!   constraint the table holds.
//! - Alternatives: an actor like the scheduler's (nothing here runs on its
//!   own; a task for four calls is more than they need); checking without
//!   a lock (two creates of one name at once would meet the index and
//!   answer `internal`, and the count could pass 256 by the number of
//!   writers).
//! - Choice: one async lock around each change's read, check and write.
//!   Changes are a person's, a few a minute at most, so holding the lock
//!   across one store write costs nothing anyone sees; reads take no lock.

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use vessel_proto::automation::{CreatePrompt, SavedPrompt, UpdatePrompt};
use vessel_proto::{Creator, ProjectId, SavedPromptId};
use vessel_store::Store;

use super::error::AutomationError;

/// The node's saved prompts.
#[derive(Debug)]
pub(crate) struct SavedPrompts {
    store: Store,
    /// Held across a change's check and write.
    changing: Mutex<()>,
}

impl SavedPrompts {
    pub(crate) fn new(store: Store) -> Self {
        Self {
            store,
            changing: Mutex::new(()),
        }
    }

    /// Every saved prompt, or `project`'s and the machine's, by name.
    ///
    /// # Errors
    /// The store failed.
    pub(crate) async fn list(
        &self,
        project: Option<ProjectId>,
    ) -> Result<Vec<SavedPrompt>, AutomationError> {
        Ok(self.store.saved_prompts(project).await?)
    }

    /// One saved prompt.
    ///
    /// # Errors
    /// `NotFound`; the store failed.
    pub(crate) async fn get(&self, id: SavedPromptId) -> Result<SavedPrompt, AutomationError> {
        self.store
            .saved_prompt(id)
            .await?
            .ok_or(AutomationError::NotFound("saved prompt"))
    }

    /// Save a new prompt, written by `by` at `now`.
    ///
    /// # Errors
    /// `Conflict` for a name its scope has or the node's bound; the store
    /// failed.
    pub(crate) async fn create(
        &self,
        request: CreatePrompt,
        by: Creator,
        now: DateTime<Utc>,
    ) -> Result<SavedPrompt, AutomationError> {
        let _changing = self.changing.lock().await;
        let (taken, count) = self
            .store
            .prompt_named(request.project, request.name.as_str().to_owned())
            .await?;
        if taken.is_some() {
            return Err(name_taken(&request));
        }
        if usize::try_from(count).unwrap_or(usize::MAX) >= SavedPrompt::MAX {
            return Err(AutomationError::Conflict(format!(
                "the node keeps at most {} saved prompts; delete one first",
                SavedPrompt::MAX
            )));
        }
        let prompt = SavedPrompt {
            id: SavedPromptId::random(),
            name: request.name,
            text: request.text,
            project: request.project,
            session: request.session,
            defined_by: by,
            created_at: now,
            updated_at: now,
            revision: 0,
        };
        self.store.put_prompt(prompt.clone()).await?;
        Ok(prompt)
    }

    /// Change a saved prompt (its project stays), made from
    /// `request.revision`.
    ///
    /// # Errors
    /// `NotFound`; `Conflict` for another revision or a name its scope
    /// has; the store failed.
    pub(crate) async fn update(
        &self,
        request: UpdatePrompt,
        by: Creator,
        now: DateTime<Utc>,
    ) -> Result<SavedPrompt, AutomationError> {
        let _changing = self.changing.lock().await;
        let mut prompt = self.get(request.prompt).await?;
        if request.revision != prompt.revision {
            return Err(AutomationError::Conflict(format!(
                "the saved prompt was changed meanwhile (now revision {}); edit that one",
                prompt.revision
            )));
        }
        if request.name != prompt.name {
            let (taken, _count) = self
                .store
                .prompt_named(prompt.project, request.name.as_str().to_owned())
                .await?;
            if taken.is_some_and(|other| other != prompt.id) {
                return Err(AutomationError::Conflict(format!(
                    "a saved prompt called `{}` is already {}",
                    request.name,
                    scope_words(prompt.project)
                )));
            }
        }
        prompt.name = request.name;
        prompt.text = request.text;
        prompt.session = request.session;
        prompt.defined_by = by;
        prompt.updated_at = now;
        prompt.revision = prompt.revision.wrapping_add(1);
        self.store.put_prompt(prompt.clone()).await?;
        Ok(prompt)
    }

    /// Remove a saved prompt.
    ///
    /// # Errors
    /// `NotFound`; the store failed.
    pub(crate) async fn delete(&self, id: SavedPromptId) -> Result<(), AutomationError> {
        let _changing = self.changing.lock().await;
        let _exists = self.get(id).await?;
        Ok(self.store.forget_prompt(id).await?)
    }
}

fn name_taken(request: &CreatePrompt) -> AutomationError {
    AutomationError::Conflict(format!(
        "a saved prompt called `{}` is already {}",
        request.name,
        scope_words(request.project)
    ))
}

fn scope_words(project: Option<ProjectId>) -> String {
    project.map_or_else(
        || "the machine's".to_owned(),
        |project| format!("in project {project}"),
    )
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use vessel_store::{DataDir, StoreConfig};

    use super::*;

    async fn prompts() -> (SavedPrompts, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let data = DataDir::new(temp.path());
        data.prepare().unwrap();
        let store = Store::open(&data, StoreConfig::default())
            .await
            .unwrap()
            .store;
        (SavedPrompts::new(store), temp)
    }

    fn create(name: &str, project: Option<ProjectId>) -> CreatePrompt {
        CreatePrompt {
            name: name.parse().unwrap(),
            text: "Review the open pull requests.".parse().unwrap(),
            project,
            session: None,
        }
    }

    fn update(prompt: &SavedPrompt, name: &str) -> UpdatePrompt {
        UpdatePrompt {
            prompt: prompt.id,
            revision: prompt.revision,
            name: name.parse().unwrap(),
            text: prompt.text.clone(),
            session: None,
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_str("2026-11-12T09:00:00Z").unwrap()
    }

    fn code(error: &AutomationError) -> vessel_proto::ErrorCode {
        error.to_wire().code
    }

    #[tokio::test]
    async fn a_name_is_unique_within_its_scope_only() {
        let (prompts, _temp) = prompts().await;
        let project = ProjectId::random();
        let machine = prompts
            .create(create("review", None), Creator::Owner, now())
            .await
            .unwrap();
        let again = prompts
            .create(create("review", None), Creator::Owner, now())
            .await
            .unwrap_err();
        assert_eq!(code(&again), vessel_proto::ErrorCode::Conflict);
        // The same name in a project is another prompt.
        let own = prompts
            .create(create("review", Some(project)), Creator::Owner, now())
            .await
            .unwrap();
        let listed = prompts.list(Some(project)).await.unwrap();
        assert_eq!(listed.len(), 2);
        let other = prompts.list(Some(ProjectId::random())).await.unwrap();
        assert_eq!(other, vec![machine.clone()]);
        // Renaming onto a name the scope has is refused; onto its own is not.
        let other = prompts
            .create(create("deploy", None), Creator::Owner, now())
            .await
            .unwrap();
        let clash = prompts
            .update(update(&other, "review"), Creator::Owner, now())
            .await
            .unwrap_err();
        assert_eq!(code(&clash), vessel_proto::ErrorCode::Conflict);
        let renamed = prompts
            .update(update(&own, "review"), Creator::Owner, now())
            .await
            .unwrap();
        assert_eq!(renamed.revision, 1);
    }

    #[tokio::test]
    async fn a_change_names_the_revision_it_was_made_from() {
        let (prompts, _temp) = prompts().await;
        let first = prompts
            .create(create("review", None), Creator::Owner, now())
            .await
            .unwrap();
        let later = now()
            .checked_add_signed(chrono::TimeDelta::minutes(5))
            .unwrap();
        let changed = prompts
            .update(update(&first, "review all"), Creator::Owner, later)
            .await
            .unwrap();
        assert_eq!(changed.created_at, now());
        assert_eq!(changed.updated_at, later);
        let stale = prompts
            .update(update(&first, "review some"), Creator::Owner, later)
            .await
            .unwrap_err();
        assert_eq!(code(&stale), vessel_proto::ErrorCode::Conflict);
        prompts.delete(first.id).await.unwrap();
        let gone = prompts.delete(first.id).await.unwrap_err();
        assert_eq!(code(&gone), vessel_proto::ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn the_node_keeps_at_most_its_bound() {
        let (prompts, _temp) = prompts().await;
        for index in 0..SavedPrompt::MAX {
            let _saved = prompts
                .create(create(&format!("p{index}"), None), Creator::Owner, now())
                .await
                .unwrap();
        }
        let over = prompts
            .create(create("one more", None), Creator::Owner, now())
            .await
            .unwrap_err();
        assert_eq!(code(&over), vessel_proto::ErrorCode::Conflict);
    }

    #[tokio::test]
    async fn two_creates_of_one_name_at_once_make_one() {
        let (prompts, _temp) = prompts().await;
        let prompts = std::sync::Arc::new(prompts);
        let writes = (0..8).map(|_| {
            let prompts = std::sync::Arc::clone(&prompts);
            tokio::spawn(async move {
                prompts
                    .create(create("review", None), Creator::Owner, now())
                    .await
            })
        });
        let mut made = 0_u32;
        for write in writes.collect::<Vec<_>>() {
            match write.await.unwrap() {
                Ok(_) => made = made.saturating_add(1),
                Err(error) => assert_eq!(code(&error), vessel_proto::ErrorCode::Conflict),
            }
        }
        assert_eq!(made, 1);
    }
}
