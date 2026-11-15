//! Fenced reads issue an actual driver command after mutation completion.
use super::*;
use crate::AgentInput;
use std::{future::Future, time::Duration};
use vessel_proto::{ChoiceValue, OptionCategory, OptionId, OptionKind};

const PHASE: Duration = Duration::from_secs(5);
async fn phase<T>(name: &str, job: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, job)
        .await
        .unwrap_or_else(|_| panic!("fenced read phase timed out: {name}"))
}
fn choice(id: &str) -> OptionChoice {
    OptionChoice {
        id: OptionId::new(id),
        value: ChoiceValue::Toggle { on: true },
    }
}
fn native() -> Vec<SessionOption> {
    vec![SessionOption {
        id: OptionId::new("actual-read"),
        name: "Native current value".into(),
        description: None,
        category: OptionCategory::Other,
        kind: OptionKind::Toggle { on: true },
    }]
}
async fn setter(
    receive: &mut mpsc::Receiver<Command>,
    expected: &str,
) -> crate::input::OptionsReply {
    let Command::SetOption { choice, reply } =
        phase("actual setter", receive.recv()).await.unwrap()
    else {
        panic!("not a setter")
    };
    assert_eq!(choice.id.as_str(), expected);
    reply
}
async fn read(receive: &mut mpsc::Receiver<Command>) -> oneshot::Sender<Vec<SessionOption>> {
    let Command::Options { reply } = phase("actual options command", receive.recv())
        .await
        .unwrap()
    else {
        panic!("not a read")
    };
    reply
}

#[tokio::test]
async fn fenced_read_waits_for_the_whole_bundle_and_returns_native_read_not_last_reply() {
    vessel_testkit::within(
        Duration::from_secs(15),
        Box::pin(async {
            let (commands, mut receive) = mpsc::channel(crate::process::COMMANDS);
            let input = AgentInput::new(commands);
            let bundle_input = input.clone();
            let options = LaunchChoices::try_from(vec![
                LaunchChoice::try_from(choice("a")).unwrap(),
                LaunchChoice::try_from(choice("b")).unwrap(),
            ])
            .unwrap();
            let bundle =
                tokio::spawn(async move { bundle_input.apply_options(options).await.unwrap() });
            let a = setter(&mut receive, "a").await;
            let mut fenced = Box::pin(input.options_fenced());
            assert!(futures_util::poll!(&mut fenced).is_pending());
            assert!(matches!(
                receive.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            a.send(Ok(Configured::without_notice(Vec::new()))).unwrap();
            let b = setter(&mut receive, "b").await;
            assert!(matches!(
                receive.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            b.send(Ok(Configured::without_notice(Vec::new()))).unwrap();
            read(&mut receive).await.send(native()).unwrap();
            assert_eq!(
                phase("actual fenced read reply", fenced).await.unwrap(),
                native()
            );
            assert_eq!(
                phase("bundle completion", bundle)
                    .await
                    .unwrap()
                    .applied()
                    .len(),
                2
            );
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_read_keeps_the_fence_until_actual_reply_and_driver_close_releases_it() {
    vessel_testkit::within(
        Duration::from_secs(15),
        Box::pin(async {
            let (commands, mut receive) = mpsc::channel(crate::process::COMMANDS);
            let input = AgentInput::new(commands);
            let read_input = input.clone();
            let pending = tokio::spawn(async move { read_input.options_fenced().await });
            let held = read(&mut receive).await;
            pending.abort();
            assert!(
                phase("cancelled reader joined", pending)
                    .await
                    .unwrap_err()
                    .is_cancelled()
            );
            let mut following = Box::pin(input.set_option(choice("after")));
            assert!(futures_util::poll!(&mut following).is_pending());
            assert!(matches!(
                receive.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            held.send(native()).unwrap();
            setter(&mut receive, "after")
                .await
                .send(Ok(Configured::without_notice(Vec::new())))
                .unwrap();
            assert!(phase("following setter result", following).await.is_ok());
            let close_input = input.clone();
            let closing = tokio::spawn(async move { close_input.options_fenced().await });
            let unanswered = read(&mut receive).await;
            drop(receive);
            assert!(matches!(
                phase("closed driver read reply", closing).await.unwrap(),
                Err(InputError::Closed)
            ));
            assert!(matches!(
                phase("retained input remains closed", input.options_fenced()).await,
                Err(InputError::Closed)
            ));
            drop(unanswered);
        }),
    )
    .await
    .unwrap();
}
