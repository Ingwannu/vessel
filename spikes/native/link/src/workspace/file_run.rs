//! One admitted file effect implementation for every C5 owner.
//! Keeping the shared source-save helper here preserves its exactly-one-write rule during cutover.
use super::{FileAnswer, FileAsk, PathChange, Save, file_call::Client};
use vessel_app_view::workspace::{problem::Problem, save::save_existing_text};
use vessel_proto::workspace::{ReadFile, WriteExpect, WriteFile};

pub(super) async fn run(client: &Client<'_>, ask: FileAsk) -> FileAnswer {
    match ask {
        FileAsk::Observe { ask, ticket } => FileAnswer::Observed {
            result: client.observe(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::Intelligence(ask) => FileAnswer::Intelligence {
            result: super::intelligence::run(client, &ask).await,
            ask,
        },
        FileAsk::Inspect { ask, ticket } => FileAnswer::Inspected {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::List { ask, ticket } => FileAnswer::Listed {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::Read { ask, ticket } => FileAnswer::Read {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::EditorRead { ask, ticket } => FileAnswer::EditorRead {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::Save(save) => saved(client, save).await,
        FileAsk::Find(ask) => FileAnswer::Found {
            result: client.call(ask.clone()).await,
            ask,
        },
        FileAsk::TrackedFind { ask, ticket } => FileAnswer::TrackedFound {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::Grep(ask) => FileAnswer::Grepped {
            result: client.call(ask.clone()).await,
            ask,
        },
        FileAsk::Path { change, ticket } => FileAnswer::PathChanged {
            result: path_change(client, change.clone()).await,
            change,
            ticket,
        },
    }
}

/// A guarded save, at most one write.
async fn saved(client: &Client<'_>, save: Save) -> FileAnswer {
    let saved = save_existing_text(
        |expect, text| {
            client.call(WriteFile {
                root: save.root,
                path: save.path.clone(),
                text,
                expect,
            })
        },
        || {
            client.call(ReadFile {
                root: save.root,
                path: save.path.clone(),
                offset: 0,
                limit: u32::try_from(vessel_app_view::workspace::editor::MAX_EDITABLE_BYTES).ok(),
            })
        },
        save.text,
        save.base.as_ref(),
    )
    .await;
    FileAnswer::Saved {
        ticket: save.ticket,
        root: save.root,
        path: save.path,
        saved,
    }
}

async fn path_change(client: &Client<'_>, change: PathChange) -> Result<(), Problem> {
    match change {
        PathChange::CreateDir(ask) => client.call(ask).await.map(drop),
        PathChange::Rename(ask) => client.call(ask).await.map(drop),
        PathChange::Delete(ask) => client.call(ask).await.map(drop),
        PathChange::CreateFile { root, path } => client
            .call(WriteFile {
                root,
                path,
                text: String::new(),
                expect: WriteExpect::Absent,
            })
            .await
            .map(drop),
    }
}
