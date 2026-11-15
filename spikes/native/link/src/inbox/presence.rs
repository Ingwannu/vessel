//! One coalescing presence worker per authenticated transport. Real input is
//! never replaced by a focus-only report while the previous RPC is pending.
use super::transport::Client;
use super::{Answer, node::problem};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::{Notify, watch};
use vessel_app_view::inbox::{
    Problem, Source,
    presence::{Activity, Focus, Report},
};
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};

#[derive(Debug, Default)]
struct Pending {
    report: Option<Report>,
    started: bool,
}
#[derive(Debug, Default)]
struct Inner {
    pending: Mutex<Pending>,
    wake: Notify,
}
/// Dropping the last `Connected` clone closes its worker, even while RPC or UI delivery waits.
#[derive(Debug, Clone)]
pub(crate) struct PresenceHub {
    inner: Arc<Inner>,
    closed: watch::Sender<()>,
}
impl Default for PresenceHub {
    fn default() -> Self {
        Self {
            inner: Arc::default(),
            closed: watch::Sender::new(()),
        }
    }
}
impl PresenceHub {
    pub(super) fn report(
        &self,
        client: Client,
        source: Source,
        report: Report,
        events: tokio::sync::mpsc::Sender<crate::Event>,
    ) {
        let start = {
            let mut state = self
                .inner
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let activity = if state
                .report
                .is_some_and(|pending| pending.activity == Activity::Active)
            {
                Activity::Active
            } else {
                report.activity
            };
            state.report = Some(Report {
                activity,
                idle: if activity == Activity::Active {
                    None
                } else {
                    report.idle
                },
                ..report
            });
            !std::mem::replace(&mut state.started, true)
        };
        if start {
            let inner = Arc::clone(&self.inner);
            let closed = self.closed.subscribe();
            drop(tokio::spawn(run(inner, closed, client, source, events)));
        }
        self.inner.wake.notify_one();
    }
}
async fn run(
    inner: Arc<Inner>,
    mut closed: watch::Receiver<()>,
    client: Client,
    source: Source,
    events: tokio::sync::mpsc::Sender<crate::Event>,
) {
    loop {
        let report = inner
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .report
            .take();
        let Some(report) = report else {
            tokio::select! { biased; _closed = closed.changed() => return, () = inner.wake.notified() => {} }
            continue;
        };
        let call = async {
            let report = wire(&source, report, client.protocol());
            client.call(report).await.map(|_ack| ()).map_err(problem)
        };
        let result = tokio::select! {
            biased;
            _closed = closed.changed() => return,
            result = tokio::time::timeout(super::DEADLINE,call) => result.unwrap_or(Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable))),
        };
        let retired = result == Err(Problem::Offline);
        tokio::select! {
            biased;
            _closed = closed.changed() => return,
            _sent = events.send(crate::Event::Inbox(Answer::Presence(source.clone(),result))) => {}
        }
        if retired {
            return;
        }
    }
}
fn wire(
    source: &Source,
    report: Report,
    protocol: vessel_proto::ProtocolVersion,
) -> ReportPushPresence {
    let supports_attention = vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL
        .is_some_and(|since| since <= protocol);
    let focus = match report.focus {
        Focus::Session { node, session } if node == source.node() => PushFocus::Session { session },
        Focus::Terminal { node, terminal } if node == source.node() && supports_attention => {
            PushFocus::Terminal { terminal }
        }
        Focus::Session { .. } | Focus::Terminal { .. } | Focus::Elsewhere => PushFocus::Elsewhere,
        Focus::Background => PushFocus::Background,
    };
    ReportPushPresence {
        focus,
        activity: match report.activity {
            Activity::Active => PushActivity::Active,
            Activity::Unchanged => PushActivity::Unchanged,
        },
        idle_age: report
            .idle
            .filter(|_sample| supports_attention)
            .map(|sample| {
                vessel_proto::push::PushIdleAge::new(sample.age_at(std::time::Instant::now()).get())
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focus_is_node_local_and_never_fabricates_activity_or_a_terminal_session() {
        let mut epochs = vessel_app_view::connection::ConnectionEpochs::default();
        let source = Source::new(
            epochs
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("test"),
                    vessel_proto::NodeId::random(),
                )
                .unwrap(),
        )
        .unwrap();
        let protocol = vessel_proto::Method::ReportPushPresence.since().unwrap();
        let foreign = Report {
            focus: Focus::Session {
                node: vessel_proto::NodeId::random(),
                session: vessel_proto::SessionId::random(),
            },
            activity: Activity::Unchanged,
            idle: None,
        };
        assert_eq!(wire(&source, foreign, protocol).focus, PushFocus::Elsewhere);
        assert_eq!(
            wire(&source, foreign, protocol).activity,
            PushActivity::Unchanged
        );
        let terminal = vessel_proto::TerminalId::random();
        assert_eq!(
            wire(
                &source,
                Report {
                    focus: Focus::Terminal {
                        node: source.node(),
                        terminal
                    },
                    activity: Activity::Active,
                    idle: None
                },
                protocol
            )
            .focus,
            PushFocus::Elsewhere
        );
        let idle = wire(
            &source,
            Report {
                idle: Some(vessel_app_view::inbox::presence::IdleSample::new(
                    vessel_app_view::inbox::presence::IdleAge::new(42),
                    std::time::Instant::now(),
                )),
                ..foreign
            },
            protocol,
        );
        assert_eq!(idle.idle_age, None);
        assert_eq!(idle.focus, PushFocus::Elsewhere);
    }
}
