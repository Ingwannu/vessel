//! "Send feedback" with one agent, and the composer's review pill, through
//! the real code panel (P6b-2b): the button makes one prompt command with
//! the review as its one context pill, the review's words and Paseo's
//! delivery; delivered, the sent comments go and a toast names the agent;
//! failed, they stay and the notice says so as an alert. The pill's press,
//! named as Paseo's, opens the workspace's Changes.
//!
//! The prompt is taken on a detached link: the framed fixture serves a
//! node call only between commands, and a prompt command waits on its
//! call. The link's own tests cover the call (`attachments/submit.rs`).
use super::super::super::Tab;
use super::*;
use gpui::{Entity, Modifiers, VisualTestContext, px, size};
use vessel_app_link::attachments::{Answer, Files, SubmitFailure, WithFiles};
use vessel_app_link::{Command, Link, testing::Scenario};
use vessel_app_view::board::CardDetail;
use vessel_app_view::composer::context::ContextAttachment;
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::{Attention, Card, Lane, Phrase};
use vessel_proto::{
    ContentBlock, Delivered, Delivery, HarnessKind, HarnessRef, ProjectId, SessionId,
};

/// The open session as a live card of the diff's project.
fn live_card(shell: &mut Shell, session: SessionId, project: ProjectId) {
    let machine = &mut shell.fleet.machines[0];
    let name = machine.name.clone();
    machine.sessions.push(Card {
        session,
        structured: true,
        live: true,
        title: "Fix the cart".to_owned(),
        machine: name,
        agent: "Claude".to_owned(),
        lane: Some(Lane::Building),
        phrase: Phrase::Starting,
        attention: Attention::AtRest,
        at: chrono::DateTime::UNIX_EPOCH,
        detail: CardDetail {
            harness: Some(HarnessRef::Known(HarnessKind::ClaudeCode)),
            cwd: Some("/work/app".to_owned()),
            project: Some(project),
            ..CardDetail::default()
        },
    });
}

/// The diff read, one comment kept, and the open session the one live
/// agent of its project: the button sends to it alone.
fn one_recipient(
    cx: &mut TestAppContext,
) -> (
    Entity<Shell>,
    &mut VisualTestContext,
    Scenario,
    SessionId,
    ReviewKey,
) {
    let (shell, cx, mut scene) = setup(cx);
    cx.simulate_resize(size(px(1280.0), px(800.0)));
    let (session, key) = shell.update_in(cx, |shell, window, cx| {
        let (session, project) = prepare(shell, &scene.context());
        root(&mut scene, shell, session, project);
        ready(&mut scene, shell);
        live_card(shell, session, project);
        read_rows(shell);
        let (key, _id) = saved(shell, window, cx);
        (session, key)
    });
    redraw(&shell, cx);
    (shell, cx, scene, session, key)
}

/// Whether the open session's composer offers the review.
fn pill_offered(shell: &Shell, session: SessionId) -> bool {
    shell
        .areas
        .composer
        .contexts
        .get(DraftKey::Session { session })
        .is_some_and(|tray| {
            tray.visible()
                .any(|held| matches!(held, ContextAttachment::Review(_)))
        })
}

/// A press on "Send feedback", the shell's commands taken on a detached
/// link meanwhile: the prompts it made.
fn press_send(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Vec<Arc<WithFiles>> {
    let (probe, mut commands) = Link::detached(64).unwrap();
    let fixture = shell.update(cx, |shell, _| shell.swap_test_link(Some(probe)));
    let button = shell
        .update(cx, |shell, _| shell.review_feedback().place.get())
        .unwrap();
    cx.simulate_click(button.center(), Modifiers::none());
    cx.run_until_parked();
    let _probe = shell.update(cx, |shell, _| shell.swap_test_link(fixture));
    let mut prompts = Vec::new();
    while let Ok(command) = commands.try_recv() {
        if let Command::Files(Files::Prompt(prompt)) = command {
            prompts.push(prompt);
        }
    }
    prompts
}

/// The link's answer to `prompt`, as it gives it to the shell.
fn answer(
    shell: &Entity<Shell>,
    cx: &mut VisualTestContext,
    prompt: Arc<WithFiles>,
    result: Result<Delivered, SubmitFailure>,
) {
    shell.update(cx, |shell, cx| {
        shell.composer_answered(Answer::Submitted { prompt, result }, cx);
    });
    redraw(shell, cx);
}

#[gpui::test]
fn feedback_to_the_one_agent_sends_one_review_and_clears_it_once_delivered(
    cx: &mut TestAppContext,
) {
    let (shell, cx, mut scene, session, key) = one_recipient(cx);
    assert!(shell.update(cx, |shell, _| pill_offered(shell, session)));
    let mut prompts = press_send(&shell, cx);
    assert_eq!(prompts.len(), 1);
    let prompt = prompts.remove(0);
    assert_eq!(prompt.session, session);
    // No turn runs: Paseo's send starts one (the node starts a queued
    // prompt at once when idle).
    assert_eq!(prompt.delivery, Delivery::Queue);
    assert_eq!(prompt.text, "Please address this code review.");
    // One pill, the review, sent as its block to a node at protocol 46.
    let pills: Vec<&ContextAttachment> = prompt.context.as_ref().unwrap().attachments().collect();
    assert!(matches!(
        pills.as_slice(),
        [pill @ ContextAttachment::Review(_)]
            if matches!(pill.block(true), ContentBlock::Review(review) if review.comments.len() == 1)
    ));
    answer(&shell, cx, prompt, Ok(Delivered::Started));
    shell.update(cx, |shell, _| {
        assert!(kept(shell, &key).is_empty());
        assert!(!pill_offered(shell, session));
        let toast = shell.areas.toasts.shown().unwrap();
        assert_eq!(toast.tone, ToastTone::Ok);
        // The words name the agent (Fluent isolates the name).
        let sent = shell.words.review().feedback_sent("Fix the cart").unwrap();
        assert_eq!(toast.text, sent);
        assert!(sent.starts_with("Feedback sent to ") && sent.contains("Fix the cart"));
    });
    assert_eq!(cx.debug_bounds("review-room").unwrap().size.height, px(0.0));
    shell.update(cx, |shell, _| scene.close(shell.fixture_link()).unwrap());
}

#[gpui::test]
fn a_failed_feedback_keeps_the_comments_and_says_so_as_an_alert(cx: &mut TestAppContext) {
    let (shell, cx, mut scene, _session, key) = one_recipient(cx);
    let mut prompts = press_send(&shell, cx);
    assert_eq!(prompts.len(), 1);
    answer(&shell, cx, prompts.remove(0), Err(SubmitFailure::NotSent));
    shell.update(cx, |shell, _| {
        assert_eq!(kept(shell, &key).len(), 1);
        assert!(shell.review_feedback().failed);
    });
    let said = arias(&shell, cx);
    assert!(said.iter().any(|aria| {
        aria.contains(r#""Alert""#) && aria.contains("Failed to send feedback. Try again.")
    }));
    shell.update(cx, |shell, _| scene.close(shell.fixture_link()).unwrap());
}

#[gpui::test]
fn the_review_pill_is_named_as_paseos_and_opens_the_changes(cx: &mut TestAppContext) {
    let (shell, cx, mut scene, session, _key) = one_recipient(cx);
    // The panel on another tab: the press brings Changes back.
    shell.update(cx, |shell, _| {
        assert!(pill_offered(shell, session));
        shell.areas.workspace.code.tab = Tab::Files;
    });
    redraw(&shell, cx);
    let said = arias(&shell, cx);
    assert!(said.iter().any(|aria| {
        aria.contains(r#""label": "Open review attachment""#) && aria.contains(r#""Button""#)
    }));
    let pill = cx.debug_bounds("composer-context-0").unwrap();
    cx.simulate_click(pill.center(), Modifiers::none());
    cx.run_until_parked();
    shell.update(cx, |shell, _| {
        let code = &shell.areas.workspace.code;
        assert!(code.open);
        assert!(matches!(code.tab, Tab::Changes));
    });
    shell.update(cx, |shell, _| scene.close(shell.fixture_link()).unwrap());
}
