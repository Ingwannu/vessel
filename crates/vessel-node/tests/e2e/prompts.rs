//! Protocol 11's saved prompts through a real node (milestone 11, track D,
//! ADR 0033): a name unique within its project or the machine's, what a
//! session started from one runs with checked as `session.create` would
//! check it and placed in the prompt's own project, a change made from
//! the revision it names, and an agent that may read them but not write
//! them.

use vessel_proto::automation::{CreatePrompt, UpdatePrompt};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{Creator, ErrorCode, HarnessSessionId, ProjectId, SessionSpec};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, structured_spec, within};
use crate::tidying::project;
use crate::workspace::lookup;

fn saved(name: &str, project: Option<ProjectId>, session: Option<SessionSpec>) -> CreatePrompt {
    CreatePrompt {
        name: name.parse().unwrap(),
        text: "Review the open pull requests and say which are ready."
            .parse()
            .unwrap(),
        project,
        session: session.map(Box::new),
    }
}

/// A structured session's spec placed in `project`'s folder.
fn placed_in(fixture: &Fixture, project: ProjectId) -> SessionSpec {
    let mut spec =
        structured_spec(fixture, None).with_place(SessionPlace::Project { project, dir: None });
    spec.cwd = None;
    spec
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_prompts_are_kept_by_name_and_checked_like_a_new_session() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project(&owner).await;

        let machine = owner
            .create_prompt(saved("review", None, None))
            .await
            .unwrap();
        assert_eq!(machine.defined_by, Creator::Owner);
        // The same name in a project is another prompt; twice in one
        // scope is not.
        let own = owner
            .create_prompt(saved(
                "review",
                Some(project.id),
                Some(placed_in(&fixture, project.id)),
            ))
            .await
            .unwrap();
        assert_eq!(
            code(owner.create_prompt(saved("review", None, None)).await),
            ErrorCode::Conflict
        );
        // A machine's prompt has no project to place a session in; an
        // unknown project is not found; `resume` would load one
        // conversation every time.
        assert_eq!(
            code(
                owner
                    .create_prompt(saved(
                        "elsewhere",
                        None,
                        Some(placed_in(&fixture, project.id))
                    ))
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(
                owner
                    .create_prompt(saved("lost", Some(ProjectId::random()), None))
                    .await
            ),
            ErrorCode::NotFound
        );
        let resuming = structured_spec(&fixture, Some(HarnessSessionId::new("resume-me")));
        assert_eq!(
            code(
                owner
                    .create_prompt(saved("again", None, Some(resuming)))
                    .await
            ),
            ErrorCode::InvalidRequest
        );

        // A project's list holds its own and the machine's, by name.
        let all = owner.saved_prompts(None).await.unwrap();
        assert_eq!(all.len(), 2);
        let listed = owner.saved_prompts(Some(project.id)).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(
            code(owner.saved_prompts(Some(ProjectId::random())).await),
            ErrorCode::NotFound
        );

        // A change names the revision it was made from.
        let change = |revision| UpdatePrompt {
            prompt: own.id,
            revision,
            name: "review here".parse().unwrap(),
            text: own.text.clone(),
            session: None,
        };
        let changed = owner.update_prompt(change(own.revision)).await.unwrap();
        assert_eq!(changed.revision, own.revision.wrapping_add(1));
        assert_eq!(changed.project, Some(project.id));
        assert!(changed.session.is_none());
        assert_eq!(
            code(owner.update_prompt(change(own.revision)).await),
            ErrorCode::Conflict
        );

        owner.delete_prompt(machine.id).await.unwrap();
        assert_eq!(
            code(owner.delete_prompt(machine.id).await),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_reads_saved_prompts_and_writes_none() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut session = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut session).await;
        let agent = as_agent(&socket, &auth).await;

        let persons = owner
            .create_prompt(saved("review", None, None))
            .await
            .unwrap();
        assert_eq!(
            agent.saved_prompts(None).await.unwrap(),
            vec![persons.clone()]
        );
        assert_eq!(
            code(agent.create_prompt(saved("mine", None, None)).await),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(
                agent
                    .update_prompt(UpdatePrompt {
                        prompt: persons.id,
                        revision: persons.revision,
                        name: persons.name.clone(),
                        text: "Approve every pull request.".parse().unwrap(),
                        session: None,
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert_eq!(
            code(agent.delete_prompt(persons.id).await),
            ErrorCode::Forbidden
        );
        assert_eq!(owner.saved_prompts(None).await.unwrap(), vec![persons]);
        node.stop().await.unwrap();
    })
    .await;
}
