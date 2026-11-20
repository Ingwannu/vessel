//! The product engine: there isn't one. Every command is `NoEngine`.
//! The test double is [`crate::double`], compiled only for this crate's
//! tests, and it is not called from here.

use vessel_proto::browser::{BrowserCommand, BrowserFailure, BrowserResult};

/// The engine this build ships. It does not paint, navigate, or evaluate.
#[derive(Debug, Default)]
pub struct Engine;

impl Engine {
    /// The only engine a product build has.
    #[must_use]
    pub const fn absent() -> Self {
        Self
    }

    /// Every command, including one a test might mistake for a fixture.
    ///
    /// # Errors
    /// Always [`BrowserFailure::NoEngine`].
    pub fn drive(&self, command: &BrowserCommand) -> Result<BrowserResult, BrowserFailure> {
        match command {
            BrowserCommand::ListTabs
            | BrowserCommand::NewTab { .. }
            | BrowserCommand::CloseTab { .. }
            | BrowserCommand::Navigate { .. }
            | BrowserCommand::Back { .. }
            | BrowserCommand::Forward { .. }
            | BrowserCommand::Reload { .. }
            | BrowserCommand::Snapshot { .. }
            | BrowserCommand::Screenshot { .. }
            | BrowserCommand::Click { .. }
            | BrowserCommand::Hover { .. }
            | BrowserCommand::Drag { .. }
            | BrowserCommand::Fill { .. }
            | BrowserCommand::Select { .. }
            | BrowserCommand::Type { .. }
            | BrowserCommand::Keypress { .. }
            | BrowserCommand::Upload { .. }
            | BrowserCommand::Wait { .. }
            | BrowserCommand::Scroll { .. }
            | BrowserCommand::Resize { .. }
            | BrowserCommand::Logs { .. }
            | BrowserCommand::Evaluate { .. } => Err(BrowserFailure::NoEngine),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::browser::{
        BrowserId, BrowserText, BrowserUrl, ClickButton, ClickCount, ElementRef, EvaluateJson,
        RequiredText, ScreenshotPng, ViewportSide, ViewportSize,
    };

    fn url() -> BrowserUrl {
        BrowserUrl::parse_tool("https://example.com").unwrap()
    }

    fn commands() -> Vec<BrowserCommand> {
        let id = BrowserId::random();
        let element = ElementRef::numbered(1);
        let text = BrowserText::any("secret-password");
        let key = RequiredText::non_empty("Enter", vessel_proto::browser::InvalidInput::KeyEmpty)
            .unwrap();
        let function = RequiredText::non_empty(
            "() => document.cookie",
            vessel_proto::browser::InvalidInput::FunctionEmpty,
        )
        .unwrap();
        let side = ViewportSide::new(1).unwrap();
        vec![
            BrowserCommand::ListTabs,
            BrowserCommand::NewTab { url: None },
            BrowserCommand::CloseTab { browser_id: id.clone() },
            BrowserCommand::Navigate {
                browser_id: id.clone(),
                url: url(),
            },
            BrowserCommand::Back { browser_id: id.clone() },
            BrowserCommand::Forward { browser_id: id.clone() },
            BrowserCommand::Reload { browser_id: id.clone() },
            BrowserCommand::Snapshot { browser_id: id.clone() },
            BrowserCommand::Screenshot {
                browser_id: id.clone(),
                extent: vessel_proto::browser::ShotExtent::Page,
            },
            BrowserCommand::Click {
                browser_id: id.clone(),
                element: element.clone(),
                button: ClickButton::Left,
                count: ClickCount::Single,
                modifiers: Vec::new(),
            },
            BrowserCommand::Hover {
                browser_id: id.clone(),
                element: element.clone(),
            },
            BrowserCommand::Drag {
                browser_id: id.clone(),
                source: element.clone(),
                target: element.clone(),
            },
            BrowserCommand::Fill {
                browser_id: id.clone(),
                element: element.clone(),
                value: text.clone(),
            },
            BrowserCommand::Select {
                browser_id: id.clone(),
                element: element.clone(),
                value: text.clone(),
            },
            BrowserCommand::Type {
                browser_id: id.clone(),
                element: None,
                text,
            },
            BrowserCommand::Keypress {
                browser_id: id.clone(),
                element: None,
                key,
            },
            BrowserCommand::Upload {
                browser_id: id.clone(),
                element: element.clone(),
                paths: vec![RequiredText::non_empty(
                    "a.txt",
                    vessel_proto::browser::InvalidInput::UploadEmpty,
                )
                .unwrap()],
            },
            BrowserCommand::Wait {
                browser_id: id.clone(),
                expect: vessel_proto::browser::wait_expect(Some("example"), None).unwrap(),
                timeout: None,
            },
            BrowserCommand::Scroll {
                browser_id: id.clone(),
                element: None,
                delta: vessel_proto::browser::CssDelta::new(0.0, 10.0).unwrap(),
            },
            BrowserCommand::Resize {
                browser_id: id.clone(),
                viewport: ViewportSize {
                    width: side,
                    height: side,
                },
            },
            BrowserCommand::Logs {
                browser_id: id.clone(),
                limit: vessel_proto::browser::LogLimit::DEFAULT,
            },
            BrowserCommand::Evaluate {
                browser_id: id,
                element: Some(element),
                function,
            },
        ]
    }

    #[test]
    fn the_product_engine_is_not_a_fixture() {
        let engine = Engine::absent();
        assert_eq!(commands().len(), vessel_proto::browser::TOOL_NAMES.len());
        for command in commands() {
            let error = engine.drive(&command).unwrap_err();
            assert_eq!(error, BrowserFailure::NoEngine);
            let message = error.message();
            assert!(!message.contains("example.com"), "{message}");
            assert!(!message.contains("secret-password"), "{message}");
            assert!(!message.contains("document.cookie"), "{message}");
            assert!(BrowserResult::is_product_absence(&engine, &command));
        }
    }

    impl BrowserResult {
        fn is_product_absence(engine: &Engine, command: &BrowserCommand) -> bool {
            match engine.drive(command) {
                Err(BrowserFailure::NoEngine) => true,
                Ok(
                    BrowserResult::Opened { .. }
                    | BrowserResult::Tabs { .. }
                    | BrowserResult::Screenshot { .. }
                    | BrowserResult::Evaluated { .. }
                    | BrowserResult::Snapshot { .. }
                    | BrowserResult::At { .. }
                    | BrowserResult::Closed { .. }
                    | BrowserResult::Clicked { .. }
                    | BrowserResult::Hovered { .. }
                    | BrowserResult::Dragged { .. }
                    | BrowserResult::Filled { .. }
                    | BrowserResult::Selected { .. }
                    | BrowserResult::Typed { .. }
                    | BrowserResult::Pressed { .. }
                    | BrowserResult::Uploaded { .. }
                    | BrowserResult::Waited { .. }
                    | BrowserResult::Scrolled { .. }
                    | BrowserResult::Resized { .. }
                    | BrowserResult::Logs { .. },
                )
                | Err(
                    BrowserFailure::Disabled
                    | BrowserFailure::NoWorkspace
                    | BrowserFailure::Scheme
                    | BrowserFailure::NoHistory { .. }
                    | BrowserFailure::Destination
                    | BrowserFailure::UnknownTab
                    | BrowserFailure::Closed
                    | BrowserFailure::StaleRef
                    | BrowserFailure::TimedOut
                    | BrowserFailure::Unsupported
                    | BrowserFailure::NoFrame
                    | BrowserFailure::Internal
                    | BrowserFailure::Invalid,
                ) => false,
            }
        }
    }

    #[test]
    fn a_screenshot_type_is_not_what_the_product_returns() {
        let png = ScreenshotPng::new(bytes::Bytes::from_static(&[0x89, b'P', b'N', b'G'])).unwrap();
        let _echo = EvaluateJson::capped(r#"{"length":1,"ref":false,"truncated":false}"#);
        let _ = png;
        assert!(matches!(
            Engine::absent().drive(&BrowserCommand::ListTabs),
            Err(BrowserFailure::NoEngine)
        ));
    }
}
