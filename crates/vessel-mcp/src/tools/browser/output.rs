//! MCP output encoding edge. JSON values exist only for this foreign protocol.
//!
//! A screenshot is one image block plus image-free metadata. Tab favicons are
//! UI data and are omitted from agent metadata; no remote icon URL is fetched.
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::{Value, json};
use vessel_proto::browser::{BrowserCommand, BrowserId, BrowserResult, BrowserTab, BrowserTool};
mod actions;

pub(super) fn render(
    command: &BrowserCommand,
    result: BrowserResult,
) -> Result<CallToolResult, serde_json::Error> {
    if !accepts(command, &result)
        || command
            .target()
            .is_some_and(|expected| target(&result) != Some(expected))
    {
        return Ok(CallToolResult::error(vec![ContentBlock::text(
            "Browser result did not match the requested operation or target. Its effects are uncertain.",
        )]));
    }
    let tool = command.tool();
    let Some(name) = tool.name().strip_prefix("browser_") else {
        return Ok(CallToolResult::error(vec![ContentBlock::text(
            "Browser result could not be identified.",
        )]));
    };
    let (mut metadata, image) = match result {
        BrowserResult::Screenshot { browser_id, image } => {
            let size = image.size();
            let meta = json!({"browserId":browser_id,"width":size.width().get(),"height":size.height().get(),"mimeType":"image/png"});
            let encoded: String = image.into();
            (meta, Some(ContentBlock::image(encoded, "image/png")))
        }
        BrowserResult::Tab { tab } => {
            let Some(metadata) = completed_tab(tool, &tab) else {
                return Ok(CallToolResult::error(vec![ContentBlock::text(
                    "Browser tab result did not match its operation.",
                )]));
            };
            (metadata, None)
        }
        BrowserResult::Tabs { tabs } => {
            let tabs = tabs.as_slice().iter().map(tab_metadata).collect::<Vec<_>>();
            (json!({"tabs":tabs}), None)
        }
        BrowserResult::Snapshot {
            browser_id,
            revision,
            document,
            content,
            stats,
        } => (
            json!({
                "browserId": browser_id, "revision": revision,
                "url": document.address, "title": document.title,
                "format": "aria-yaml", "snapshot": content.text,
                "truncated": content.completeness == vessel_proto::browser::TextCompleteness::Truncated,
                "stats": {
                    "nodeCount": stats.nodes.get(), "refCount": stats.references.get(),
                    "textLength": stats.text_bytes.get(), "iframeCount": stats.frames.get()
                }
            }),
            None,
        ),
        BrowserResult::Evaluation { browser_id, json } => (
            json!({
                "browserId": browser_id, "resultJson": json.text,
                "truncated": json.completeness == vessel_proto::browser::TextCompleteness::Truncated
            }),
            None,
        ),
        BrowserResult::Applied {
            browser_id,
            outcome,
        } => (actions::metadata(&browser_id, outcome), None),
        BrowserResult::Matched {
            browser_id,
            matched,
        } => (json!({"browserId":browser_id,"matched":matched}), None),
        other @ (BrowserResult::Logs { .. } | BrowserResult::Closed { .. }) => {
            (serde_json::to_value(other)?, None)
        }
    };
    let Some(fields) = metadata.as_object_mut() else {
        return Ok(CallToolResult::error(vec![ContentBlock::text(
            "Browser result metadata could not be encoded.",
        )]));
    };
    let _kind = fields.remove("kind");
    let _old = fields.insert("command".into(), json!(name));
    if let Some(id) = fields.remove("browser_id") {
        let _old = fields.insert("browserId".into(), id);
    }
    let structured = json!({"ok":true,"result":metadata});
    let text = serde_json::to_string(&structured)?;
    let mut content = vec![ContentBlock::text(text)];
    if let Some(image) = image {
        content.push(image);
    }
    let mut reply = CallToolResult::success(content);
    reply.structured_content = Some(structured);
    Ok(reply)
}
// Project completed backend facts. In particular URL and effective dimensions
// can differ from the request; echoing its arguments would hide that outcome.
fn completed_tab(tool: BrowserTool, tab: &BrowserTab) -> Option<Value> {
    use BrowserTool as T;
    match tool {
        T::NewTab => Some(json!({
            "browserId": &tab.browser_id, "workspaceId": tab.workspace, "url": &tab.url
        })),
        T::Navigate => Some(json!({"browserId": &tab.browser_id, "url": &tab.url})),
        T::Back | T::Forward | T::Reload => Some(json!({"browserId": &tab.browser_id})),
        T::Resize => {
            let size = match tab.viewport {
                vessel_proto::browser::BrowserViewport::Responsive(size)
                | vessel_proto::browser::BrowserViewport::Fixed(size) => size,
            };
            Some(
                json!({"browserId": &tab.browser_id, "width": size.width.get(), "height": size.height.get()}),
            )
        }
        T::ListTabs
        | T::Snapshot
        | T::Click
        | T::Fill
        | T::Wait
        | T::Type
        | T::Keypress
        | T::Screenshot
        | T::Upload
        | T::Select
        | T::Hover
        | T::Drag
        | T::Logs
        | T::Evaluate
        | T::Scroll
        | T::CloseTab => None,
    }
}
fn tab_metadata(tab: &BrowserTab) -> Value {
    // Project before serialization: encoding then removing an icon would still
    // allocate/copy its base64 despite never showing it to the agent.
    json!({
        "browserId": &tab.browser_id, "workspaceId": tab.workspace,
        "url": &tab.url, "title": &tab.title, "load": tab.load,
        "history": tab.history, "viewport": tab.viewport, "profile": tab.profile
    })
}

fn target(result: &BrowserResult) -> Option<&BrowserId> {
    match result {
        BrowserResult::Tabs { .. } => None,
        BrowserResult::Tab { tab } => Some(&tab.browser_id),
        BrowserResult::Applied { browser_id, .. }
        | BrowserResult::Snapshot { browser_id, .. }
        | BrowserResult::Screenshot { browser_id, .. }
        | BrowserResult::Evaluation { browser_id, .. }
        | BrowserResult::Logs { browser_id, .. }
        | BrowserResult::Matched { browser_id, .. }
        | BrowserResult::Closed { browser_id } => Some(browser_id),
    }
}
fn accepts(command: &BrowserCommand, result: &BrowserResult) -> bool {
    use BrowserTool as T;
    let tool = command.tool();
    match tool {
        T::ListTabs => matches!(result, BrowserResult::Tabs { .. }),
        T::NewTab | T::Navigate | T::Back | T::Forward | T::Reload | T::Resize => {
            matches!(result, BrowserResult::Tab { .. })
        }
        T::Snapshot => matches!(result, BrowserResult::Snapshot { .. }),
        T::Click | T::Fill | T::Type | T::Keypress | T::Select | T::Hover | T::Drag | T::Scroll => {
            matches!(result, BrowserResult::Applied { outcome, .. } if outcome.tool() == tool && actions::selectors_match(command, outcome))
        }
        // No staged-file/result producer exists yet. A generic Applied success
        // must not stand in for actual selected files or expose private paths.
        T::Upload => false,
        T::Wait => {
            matches!((command, result), (BrowserCommand::Wait(args), BrowserResult::Matched { matched, .. }) if matches!((&args.condition, matched), (vessel_proto::browser::WaitFor::Text(_), vessel_proto::browser::WaitMatch::Text) | (vessel_proto::browser::WaitFor::Url(_), vessel_proto::browser::WaitMatch::Url)))
        }
        T::Screenshot => matches!(result, BrowserResult::Screenshot { .. }),
        T::Logs => matches!(result, BrowserResult::Logs { .. }),
        T::Evaluate => matches!(result, BrowserResult::Evaluation { .. }),
        T::CloseTab => matches!(result, BrowserResult::Closed { .. }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::browser::{NoBrowserArgs, ScreenshotArgs, ScreenshotExtent, ScreenshotPng};
    fn id(last: char) -> BrowserId {
        format!("00000000-0000-4000-8000-00000000000{last}")
            .parse()
            .unwrap()
    }
    fn screenshot() -> ScreenshotPng {
        ScreenshotPng::from_png(bytes::Bytes::from_static(include_bytes!(
            "fixtures/pipe-canary.png"
        )))
        .unwrap()
    }
    #[test]
    fn image_is_present_once_and_never_duplicated_in_structured_or_text_metadata() {
        let image = screenshot();
        let encoded = String::from(image.clone());
        let command = BrowserCommand::Screenshot(ScreenshotArgs {
            browser_id: id('1'),
            extent: ScreenshotExtent::default(),
        });
        let result = render(
            &command,
            BrowserResult::Screenshot {
                browser_id: id('1'),
                image,
            },
        )
        .unwrap();
        let images = result
            .content
            .iter()
            .filter_map(ContentBlock::as_image)
            .collect::<Vec<_>>();
        assert_eq!(images.len(), 1);
        assert_eq!(images.first().unwrap().data, encoded);
        let metadata = serde_json::to_string(&result.structured_content).unwrap();
        assert!(!metadata.contains(&encoded));
        for text in result.content.iter().filter_map(ContentBlock::as_text) {
            assert!(!text.text.contains(&encoded));
        }
        assert_eq!(
            result.structured_content.unwrap()["result"]["width"],
            json!(390)
        );
    }
    #[test]
    fn mismatched_target_or_operation_refuses_before_exposing_a_result() {
        let command = BrowserCommand::Screenshot(ScreenshotArgs {
            browser_id: id('1'),
            extent: ScreenshotExtent::default(),
        });
        let result = render(
            &command,
            BrowserResult::Screenshot {
                browser_id: id('2'),
                image: screenshot(),
            },
        )
        .unwrap();
        assert_eq!(result.is_error, Some(true));
        assert!(result.content.iter().all(|item| item.as_image().is_none()));
        let result = render(
            &BrowserCommand::ListTabs(NoBrowserArgs {}),
            BrowserResult::Applied {
                browser_id: id('1'),
                outcome: vessel_proto::browser::ActionOutcome::Fill {
                    element: "@e1".parse().unwrap(),
                },
            },
        )
        .unwrap();
        assert_eq!(result.is_error, Some(true));
    }
}

#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod tab_tests;

#[cfg(test)]
mod action_tests;
#[cfg(test)]
mod evaluation_tests;
