//! A clone followed until it ends (the page's `project-store.ts`
//! `follow`): `project.get { changed_from }` waits on the node (at most
//! 30 s) until the project's revision moves; each answer is sent on as an
//! [`Answer::Project`]. A machine that answers at once with nothing new is
//! asked again only after the quiet time (`view::workspace::pace`).

use std::time::Instant;

use vessel_app_view::workspace::pace::quiet_left;
use vessel_client::NodeClient;
use vessel_proto::workspace::{GetProject, ProjectInfo, ProjectState};

use super::calls::call;
use super::{Answer, Hub, Reply};

/// Follow `project` on its own task, unless it is followed already.
pub(super) fn clone_of(hub: &Hub, client: &NodeClient, reply: &Reply, project: ProjectInfo) {
    let key = (reply.machine.clone(), project.id);
    // The registry stays locked while the task is made and recorded, so a
    // task that ends at once removes itself only after it was recorded.
    let Ok(mut following) = hub.following.lock() else {
        return;
    };
    if following.contains_key(&key) {
        return;
    }
    let (owner, client, reply, done) = (hub.clone(), client.clone(), reply.clone(), key.clone());
    let task = tokio::spawn(async move {
        follow(&client, &reply, project).await;
        if let Ok(mut following) = owner.following.lock() {
            let _was = following.remove(&done);
        }
    });
    let _was = following.insert(key, task.abort_handle());
}

async fn follow(client: &NodeClient, reply: &Reply, project: ProjectInfo) {
    let mut revision = project.revision;
    loop {
        let asked = Instant::now();
        let read = call(
            client,
            GetProject {
                project: project.id,
                changed_from: Some(revision),
            },
        )
        .await;
        let Ok(now) = read else {
            // The machine went away or refused: the list's next read shows
            // the project as it is then.
            return;
        };
        let cloning = matches!(now.state, ProjectState::Cloning { .. });
        let unchanged = now.revision == revision;
        revision = now.revision;
        reply.send(Answer::Project(now)).await;
        if !cloning {
            return;
        }
        if unchanged && let Some(left) = quiet_left(asked, Instant::now()) {
            tokio::time::sleep(left).await;
        }
    }
}
