//! Candidate compositor operations are sealed and use the PNG transport limit.

use super::{Acknowledged, Method, Request, sealed};
use serde::{Deserialize, Serialize};
use vessel_proto::browser::{ScreenshotPng, ViewportSide, ViewportSize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartScreencast {
    format: Format,
    max_width: ViewportSide,
    max_height: ViewportSide,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Format {
    Png,
}

impl StartScreencast {
    pub(crate) const fn with_viewport(viewport: ViewportSize) -> Self {
        Self {
            format: Format::Png,
            max_width: viewport.width,
            max_height: viewport.height,
        }
    }
}
request!(StartScreencast, Acknowledged, StartScreencast);

/// Echo the compositor's token independently of the flattened CDP session.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(transparent)]
struct FrameToken(std::num::NonZeroU32);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreencastFrame {
    session_id: FrameToken,
    data: ScreenshotPng,
}

impl ScreencastFrame {
    pub(crate) const fn acknowledgement(&self) -> ScreencastAck {
        ScreencastAck {
            session_id: self.session_id,
        }
    }

    pub(crate) fn into_image(self) -> ScreenshotPng {
        self.data
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreencastAck {
    session_id: FrameToken,
}
request!(ScreencastAck, Acknowledged, ScreencastAck);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_request_bounds_compositor_geometry() {
        let viewport = ViewportSize {
            width: ViewportSide::try_from(390).unwrap(),
            height: ViewportSide::try_from(844).unwrap(),
        };
        let request = StartScreencast::with_viewport(viewport);
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"format":"png","maxWidth":390,"maxHeight":844})
        );
    }
}
