//! Clone observations share call capacity and the original physical admission.
use super::{Answer, Calls, ConnectionContext, Response, SelectedConnection};
use vessel_app_view::workspace::{
    pace::quiet_left,
    projects::{actions::ProjectChanged, is_cloning},
};
use vessel_proto::workspace::{GetProject, ProjectInfo};

pub(super) fn start(
    calls: &Calls,
    selected: &SelectedConnection,
    context: &ConnectionContext,
    answer: &Answer,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
) {
    match answer {
        Answer::Projects {
            result: Ok(list), ..
        } => {
            for project in list.projects.iter().filter(|project| is_cloning(project)) {
                observe(calls, selected, context, project, events);
            }
        }
        Answer::ProjectChanged {
            result: Ok(ProjectChanged::Registered(project)),
            ..
        } if is_cloning(project) => {
            observe(calls, selected, context, project, events);
        }
        _ => {}
    }
}

fn observe(
    calls: &Calls,
    selected: &SelectedConnection,
    context: &ConnectionContext,
    project: &ProjectInfo,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
) {
    let Ok(mut following) = calls.following.lock() else {
        return;
    };
    let key = (context.clone(), project.id);
    if following.contains(&key) {
        return;
    }
    let Ok(permit) = std::sync::Arc::clone(&calls.permits).try_acquire_owned() else {
        return;
    };
    following.push(key.clone());
    let (calls, selected, context, project, events) = (
        calls.clone(),
        selected.clone(),
        context.clone(),
        project.clone(),
        events.clone(),
    );
    drop(tokio::spawn(async move {
        follow(&selected, &context, project, &events).await;
        if let Ok(mut following) = calls.following.lock() {
            following.retain(|entry| *entry != key);
        }
        drop(permit);
    }));
}

async fn follow(
    selected: &SelectedConnection,
    context: &ConnectionContext,
    mut project: ProjectInfo,
    events: &tokio::sync::mpsc::Sender<crate::Event>,
) {
    loop {
        let asked = std::time::Instant::now();
        let read = tokio::time::timeout(
            super::PROJECT_CALL_DEADLINE,
            selected.call(GetProject {
                project: project.id,
                changed_from: Some(project.revision),
            }),
        )
        .await;
        let Ok(Ok(now)) = read else {
            return;
        };
        let unchanged = now.revision == project.revision;
        project = now;
        let event = crate::Event::Projects(Response {
            context: context.clone(),
            answer: Answer::Project {
                node: context.node(),
                project: project.clone(),
            },
        });
        if events.send(event).await.is_err() || !is_cloning(&project) {
            return;
        }
        if unchanged && let Some(left) = quiet_left(asked, std::time::Instant::now()) {
            tokio::time::sleep(left).await;
        }
    }
}
