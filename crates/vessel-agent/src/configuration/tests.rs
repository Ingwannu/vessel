//! Completion barriers prove the shared owner rather than provider timing.

use super::*;
use crate::AgentInput;
use std::{future::Future, time::Duration};
use vessel_proto::{ChoiceValue, OptionId};

const LIMIT: Duration = Duration::from_secs(5);
const SAFETY: Duration = Duration::from_secs(15);

async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(LIMIT, future)
        .await
        .unwrap_or_else(|_| panic!("phase timed out: {name}"))
}

fn choice(id: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Toggle { on: true },
    }
}
fn choices(ids: &[&str]) -> LaunchChoices {
    ids.iter()
        .map(|id| LaunchChoice::try_from(choice(id)).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}
async fn setter(receive: &mut mpsc::Receiver<Command>, id: &str) -> crate::input::OptionsReply {
    let Command::SetOption { choice, reply } = phase("next actual driver setter", receive.recv())
        .await
        .unwrap()
    else {
        panic!("not a setter")
    };
    assert_eq!(choice.id.as_str(), id);
    reply
}
fn completed(notice: Option<&str>) -> Configured {
    Configured {
        options: Vec::new(),
        notice: notice.map(|value| value.parse().unwrap()),
    }
}

#[tokio::test]
async fn bundle_waits_for_each_completion_and_keeps_first_notice_and_prefix() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let (send, mut receive) = mpsc::channel(crate::process::COMMANDS);
            let input = AgentInput::new(send);
            let bundle_input = input.clone();
            let bundle = tokio::spawn(async move {
                bundle_input
                    .apply_options(choices(&["a", "b", "c", "never"]))
                    .await
                    .unwrap()
            });
            let first = setter(&mut receive, "a").await;
            let single_input = input.clone();
            let single =
                tokio::spawn(async move { single_input.set_option(choice("outside")).await });
            // The driver loop is free, yet neither the next bundle step nor
            // the old setter may run before the actual provider answer.
            assert!(matches!(
                receive.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            first.send(Ok(completed(Some("first notice")))).unwrap();
            setter(&mut receive, "b")
                .await
                .send(Ok(completed(Some("later notice"))))
                .unwrap();
            setter(&mut receive, "c")
                .await
                .send(Err(InputError::Closed))
                .unwrap();
            let result = phase("bundle result", bundle).await.unwrap();
            assert_eq!(
                result
                    .applied()
                    .iter()
                    .map(|value| value.as_choice().id.as_str())
                    .collect::<Vec<_>>(),
                ["a", "b"]
            );
            assert_eq!(result.notice().unwrap().as_str(), "first notice");
            assert_eq!(
                result.failure().unwrap().choice().as_choice().id.as_str(),
                "c"
            );
            assert!(matches!(
                result.failure().unwrap().error(),
                InputError::Closed
            ));
            setter(&mut receive, "outside")
                .await
                .send(Ok(completed(None)))
                .unwrap();
            assert!(
                phase("old single setter result", single)
                    .await
                    .unwrap()
                    .is_ok()
            );
            assert!(matches!(
                receive.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            drop(input);
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_caller_does_not_release_the_real_mutation_fence() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let (send, mut receive) = mpsc::channel(crate::process::COMMANDS);
            let input = AgentInput::new(send);
            let pending_input = input.clone();
            let pending =
                tokio::spawn(
                    async move { pending_input.apply_options(choices(&["a", "b"])).await },
                );
            let first = setter(&mut receive, "a").await;
            pending.abort();
            assert!(
                phase("cancelled caller joined", pending)
                    .await
                    .unwrap_err()
                    .is_cancelled()
            );
            let single_input = input.clone();
            let single =
                tokio::spawn(async move { single_input.set_option(choice("after")).await });
            assert!(matches!(
                receive.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            first.send(Ok(completed(None))).unwrap();
            setter(&mut receive, "b")
                .await
                .send(Ok(completed(None)))
                .unwrap();
            setter(&mut receive, "after")
                .await
                .send(Ok(completed(None)))
                .unwrap();
            assert!(phase("following setter", single).await.unwrap().is_ok());
            drop(input);
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn driver_closure_ends_owner_even_with_external_handles_and_unanswered_reply() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let (send, mut receive) = mpsc::channel(crate::process::COMMANDS);
            let input = Configuration::spawn(send);
            let pending_input = input.clone();
            let pending = tokio::spawn(async move {
                pending_input
                    .apply_options(choices(&["a", "b"]))
                    .await
                    .unwrap()
            });
            let unanswered = setter(&mut receive, "a").await;
            drop(receive);
            let result = phase("driver closure result", pending).await.unwrap();
            assert!(result.applied().is_empty());
            assert!(matches!(
                result.failure().unwrap().error(),
                InputError::Closed
            ));
            phase("configuration owner exits", input.0.closed()).await;
            assert!(matches!(
                input.single(choice("later")).await,
                Err(InputError::Closed)
            ));
            // Retain both sender and handle until after the owner demonstrably exits.
            drop(unanswered);
            drop(input);
        }),
    )
    .await
    .unwrap();
}
