//! One file effect implementation, shared by legacy callers and admitted C5 calls.
//! Keeping save_text here preserves its exactly-one-write rule during cutover.
use super::{Answer, FileAsk, PathChange, Save, file_call::Client};
use vessel_app_view::workspace::{problem::Problem, save::save_text};
use vessel_proto::workspace::{ReadFile, WriteExpect, WriteFile};

pub(super) async fn run(client: &Client<'_>, ask: FileAsk) -> Answer {
    match ask {
        FileAsk::Intelligence(ask) => Answer::Intelligence {
            result: super::intelligence::run(client, &ask).await,
            ask,
        },
        FileAsk::List(ask) => Answer::Listed {
            result: client.call(ask.clone()).await,
            ask,
        },
        FileAsk::Read { ask, ticket } => Answer::Read {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::EditorRead { ask, ticket } => Answer::EditorRead {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::Save(save) => saved(client, save).await,
        FileAsk::Find(ask) => Answer::Found {
            result: client.call(ask.clone()).await,
            ask,
        },
        FileAsk::TrackedFind { ask, ticket } => Answer::TrackedFound {
            result: client.call(ask.clone()).await,
            ask,
            ticket,
        },
        FileAsk::Grep(ask) => Answer::Grepped {
            result: client.call(ask.clone()).await,
            ask,
        },
        FileAsk::Path { change, ticket } => Answer::PathChanged {
            result: path_change(client, change.clone()).await,
            change,
            ticket,
        },
    }
}

/// A guarded save, at most one write.
async fn saved(client: &Client<'_>, save: Save) -> Answer {
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
    Answer::Saved {
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
