//! Guest navigation is classified at the debugger boundary without exposing URLs.

use serde::Deserialize;
use vessel_proto::browser::{BrowserUrl, PageText};

use super::{Event, EventKind};
use crate::cdp::CdpError;
use crate::cdp::identity::FrameId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuestAddress {
    Web,
    Empty,
    Denied,
}

impl GuestAddress {
    fn classify(value: &PageText) -> Self {
        if value.as_str() == "about:blank" {
            Self::Empty
        } else if value.as_str().parse::<BrowserUrl>().is_ok() {
            Self::Web
        } else {
            Self::Denied
        }
    }
}

#[derive(Debug)]
pub(crate) enum GuestNavigation {
    Requested {
        frame: FrameId,
        address: GuestAddress,
    },
    Committed(GuestAddress),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Requested {
    frame_id: FrameId,
    url: PageText,
}

#[derive(Deserialize)]
struct Committed {
    frame: ObservedFrame,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObservedFrame {
    parent_id: Option<FrameId>,
    url: PageText,
}

impl Event {
    pub(crate) fn guest_navigation(&self) -> Result<Option<GuestNavigation>, CdpError> {
        match self.kind {
            EventKind::NavigationRequested => {
                let request: Requested = self.params.decode().map_err(CdpError::Json)?;
                Ok(Some(GuestNavigation::Requested {
                    frame: request.frame_id,
                    address: GuestAddress::classify(&request.url),
                }))
            }
            EventKind::Navigated => {
                let commit: Committed = self.params.decode().map_err(CdpError::Json)?;
                Ok(commit
                    .frame
                    .parent_id
                    .is_none()
                    .then(|| GuestNavigation::Committed(GuestAddress::classify(&commit.frame.url))))
            }
            EventKind::Attached
            | EventKind::Detached
            | EventKind::Crashed
            | EventKind::DocumentOpened
            | EventKind::WithinDocument
            | EventKind::Loading
            | EventKind::Stopped
            | EventKind::Loaded
            | EventKind::Screencast
            | EventKind::Dialog
            | EventKind::Console
            | EventKind::Exception
            | EventKind::Request
            | EventKind::Response
            | EventKind::RequestFailed
            | EventKind::RequestPaused
            | EventKind::Download
            | EventKind::Unknown => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsupported_guest_request_is_a_refusal_without_a_raw_url() {
        let message = r#"{"method":"Page.frameRequestedNavigation","sessionId":"target-a","params":{"frameId":"frame-a","url":"mailto:fixture@example.test"}}"#;
        let super::super::Incoming::Event(event) = super::super::incoming(message).unwrap() else {
            panic!("expected a navigation request");
        };
        assert!(matches!(
            event.guest_navigation().unwrap(),
            Some(GuestNavigation::Requested {
                address: GuestAddress::Denied,
                ..
            })
        ));
        assert!(!format!("{:?}", event.guest_navigation()).contains("fixture@example.test"));
    }
}
