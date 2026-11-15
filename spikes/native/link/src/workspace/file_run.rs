//! One admitted file effect implementation for every C5 owner.
//! Keeping save_text here preserves its exactly-one-write rule during cutover.
use super::{FileAnswer, FileAsk, PathChange, Save, file_call::Client};
use vessel_app_view::workspace::{problem::Problem, save::save_text};
use vessel_proto::workspace::{ReadFile, WriteExpect, WriteFile};

pub(super) async fn run(client: &Client<'_>, ask: FileAsk) -> FileAnswer {
    match ask {
        FileAsk::Intelligence(ask) => FileAnswer::Intelligence {
            result: super::intelligence::run(client, &ask).await,
            ask,
        },
        FileAsk::List(ask) => FileAnswer::Listed {
            result: client.call(ask.clone()).await,
            ask,
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
    let saved = save_text(
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
                limit: None,
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
