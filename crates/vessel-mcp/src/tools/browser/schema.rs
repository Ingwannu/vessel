//! MCP JSON Schema edge. Construction does not register or offer a browser tool.
use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::browser::{
    BrowserTool, MAX_BROWSER_INPUT_BYTES, MAX_LOG_ENTRIES, MAX_UPLOAD_FILES, MAX_VIEWPORT_SIDE,
    MAX_WAIT_MS,
};

pub(super) fn schema(tool: BrowserTool) -> Tool {
    let mut properties = JsonObject::new();
    let mut required = Vec::new();
    if !matches!(tool, BrowserTool::ListTabs | BrowserTool::NewTab) {
        let _replaced = properties.insert("browserId".into(),json!({"type":"string","description":"Use the exact browserId returned for this workspace. This is not a CDP target id."}));
        required.push("browserId");
    }
    fields(tool, &mut properties, &mut required);
    let mut schema = json!({"type":"object","properties":properties,"required":required});
    if tool == BrowserTool::Wait {
        schema["oneOf"] = json!([
            {"required":["text"],"not":{"required":["url"]}},
            {"required":["url"],"not":{"required":["text"]}}
        ]);
    }
    Tool::new(
        tool.name(),
        description(tool),
        schema.as_object().cloned().unwrap_or_default(),
    )
}
fn fields(tool: BrowserTool, p: &mut JsonObject, required: &mut Vec<&'static str>) {
    use BrowserTool as T;
    match tool {
        T::ListTabs | T::Snapshot | T::Back | T::Forward | T::Reload | T::CloseTab => {}
        T::NewTab => {
            let _replaced = p.insert("url".into(), url());
        }
        T::Navigate => {
            let _replaced = p.insert("url".into(), url());
            required.push("url");
        }
        T::Click => {
            reference(p, required);
            let _replaced = p.insert(
                "button".into(),
                json!({"type":"string","enum":["left","right","middle"],"default":"left"}),
            );
            let _replaced = p.insert(
                "doubleClick".into(),
                json!({"type":"boolean","default":false}),
            );
            let _replaced = p.insert("modifiers".into(),json!({"type":"array","items":{"type":"string","enum":["Alt","Control","Meta","Shift"]},"default":[]}));
        }
        T::Fill | T::Select => {
            reference(p, required);
            let _replaced = p.insert("value".into(), text());
            required.push("value");
        }
        T::Hover => reference(p, required),
        T::Wait => {
            let _replaced = p.insert("text".into(), nonempty());
            let _replaced = p.insert("url".into(), nonempty());
            let _replaced = p.insert(
                "timeoutMs".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_WAIT_MS}),
            );
        }
        T::Type => {
            optional_ref(p);
            let _replaced = p.insert("text".into(), text());
            required.push("text");
        }
        T::Keypress => {
            optional_ref(p);
            let _replaced = p.insert("key".into(), nonempty());
            required.push("key");
        }
        T::Screenshot => {
            let _replaced = p.insert("fullPage".into(), json!({"type":"boolean","default":false}));
        }
        T::Upload => {
            reference(p, required);
            let _replaced = p.insert("filePaths".into(),json!({"type":"array","minItems":1,"maxItems":MAX_UPLOAD_FILES,"items":{"type":"string","minLength":1,"maxLength":vessel_proto::workspace::RelPath::MAX_BYTES},"description":format!("Untrusted workspace paths, each at most {} UTF-8 bytes. A successful upload requires separately admitted private staged regular files.",vessel_proto::workspace::RelPath::MAX_BYTES)}));
            required.push("filePaths");
        }
        T::Drag => {
            let _replaced = p.insert("sourceRef".into(), reference_schema());
            let _replaced = p.insert("targetRef".into(), reference_schema());
            required.extend(["sourceRef", "targetRef"]);
        }
        T::Logs => {
            let _replaced = p.insert(
                "maxEntries".into(),
                json!({"type":"integer","minimum":1,"maximum":MAX_LOG_ENTRIES}),
            );
        }
        T::Evaluate => {
            optional_ref(p);
            let _replaced = p.insert("function".into(), nonempty());
            required.push("function");
        }
        T::Scroll => {
            optional_ref(p);
            let _replaced = p.insert("deltaX".into(), json!({"type":"number"}));
            let _replaced = p.insert("deltaY".into(), json!({"type":"number"}));
            required.extend(["deltaX", "deltaY"]);
        }
        T::Resize => {
            for key in ["width", "height"] {
                let _replaced = p.insert(
                    key.into(),
                    json!({"type":"integer","minimum":1,"maximum":MAX_VIEWPORT_SIDE}),
                );
            }
            required.extend(["width", "height"]);
        }
    }
}
fn text() -> Value {
    json!({"type":"string","maxLength":MAX_BROWSER_INPUT_BYTES,"description":"The node also enforces the text limit in UTF-8 bytes."})
}
fn nonempty() -> Value {
    let mut schema = text();
    schema["minLength"] = json!(1);
    schema
}
fn url() -> Value {
    json!({"type":"string","description":"HTTP(S) URL or scheme-less host (HTTP default). Destination policy is enforced by the node; a URL cannot grant private Preview access."})
}
fn reference_schema() -> Value {
    json!({"type":"string","pattern":"^@e[0-9]+$","description":"A current reference from a snapshot of the same tab; navigation invalidates it."})
}
fn optional_ref(p: &mut JsonObject) {
    let _replaced = p.insert("ref".into(), reference_schema());
}
fn reference(p: &mut JsonObject, required: &mut Vec<&'static str>) {
    optional_ref(p);
    required.push("ref");
}
fn description(tool: BrowserTool) -> &'static str {
    use BrowserTool as T;
    match tool {
        T::ListTabs => "List this agent workspace's permitted browser tabs on its node.",
        T::NewTab => {
            "Create a background browser tab in this workspace. Omitted URL uses the node default."
        }
        T::Snapshot => {
            "Read a bounded page snapshot and assign references for this tab's current document."
        }
        T::Click => {
            "Click a current element reference with the requested button, click count and modifiers."
        }
        T::Fill => "Replace an input-like element's value; an empty string clears it.",
        T::Wait => {
            "Wait for exactly one text or URL substring condition; the host default is used when omitted."
        }
        T::Type => "Type text into a current referenced element or this tab's current focus.",
        T::Keypress => "Dispatch a supported key chord to a current element or this tab's focus.",
        T::Navigate => {
            "Navigate this tab to an HTTP(S) destination admitted by its existing profile policy."
        }
        T::Back => "Navigate backward in this tab's real history; an empty stack is an error.",
        T::Forward => "Navigate forward in this tab's real history; an empty stack is an error.",
        T::Reload => "Reload the tab's current page and invalidate previous element references.",
        T::Screenshot => {
            "Capture a bounded PNG from the real browser; no unpainted fixture is returned."
        }
        T::Upload => "Attach staged regular workspace files to a current file-input reference.",
        T::Select => "Select an available option in a current element reference.",
        T::Hover => "Move the browser pointer over a current actionable element reference.",
        T::Drag => "Drag between two current references after validating both endpoints.",
        T::Logs => "Read bounded console and request metadata tails for this permitted tab.",
        T::Evaluate => {
            "Run a page function, optionally with a current referenced element, and return bounded data."
        }
        T::Scroll => {
            "Send finite CSS wheel deltas to the viewport or a current referenced element."
        }
        T::Resize => "Resize this tab's CSS viewport within side and total raster limits.",
        T::CloseTab => "Close the owned tab and settle its watches and pending operations.",
    }
}
