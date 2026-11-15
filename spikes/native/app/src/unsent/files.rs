//! Local refusal stays inside the feature envelope and retains final payload ownership.
use super::Settle;
use vessel_app_link::{
    Event,
    attachments::{Answer, CapabilityFailure, Files, Step, SubmitFailure, WakeOutcome},
    mentions::{Ask, RootAnswer},
};
use vessel_app_view::composer::{
    mentions::Failure,
    tray::{Again, Problem},
};

pub(super) fn settle(files: Files, words: &str) -> Settle {
    let event = match files {
        Files::Suggest(Ask::Root(ask)) => Answer::Root(RootAnswer {
            ask,
            result: Err(Failure::Offline),
        }),
        Files::Suggest(Ask::Suggestions(request)) => {
            Answer::Suggestions(vessel_app_link::mentions::Answer {
                request,
                result: Err(Failure::Offline),
            })
        }
        Files::Create(_) => return Settle::Answer(Box::new(Event::Failed(words.to_owned()))),
        Files::Prompt(prompt) => Answer::Submitted {
            prompt,
            result: Err(SubmitFailure::NotSent),
        },
        Files::Send(attach) => Answer::Upload {
            message: attach.message,
            session: attach.session,
            key: attach.key,
            step: Step::Failed {
                problem: Problem::Node(words.to_owned()),
                again: Again::Restart,
            },
        },
        Files::Capabilities(ask) => Answer::Capabilities {
            ask,
            result: Err(CapabilityFailure::Offline),
        },
        Files::Wake(wake) => Answer::Woke {
            message: wake.message,
            session: wake.session,
            outcome: WakeOutcome::Refused,
        },
        Files::Read(sent) => Answer::Read { sent, bytes: None },
    };
    Settle::Answer(Box::new(Event::Files(event)))
}
