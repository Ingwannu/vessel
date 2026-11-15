//! One catalogue maps the reference tool names to typed browser operations.
//!
//! This does not advertise the tools: the MCP policy must first establish a
//! ready engine and the session-start setting/grant. Unknown names are rejected
//! at the MCP decoding edge rather than passed through as arbitrary CDP methods.

use super::arguments::{
    ClickArgs, DragArgs, ElementArgs, EvaluateArgs, KeypressArgs, LogsArgs, NavigateArgs,
    NewTabArgs, NoBrowserArgs, ResizeArgs, ScreenshotArgs, ScrollArgs, TabArgs, TypeArgs,
    ValueArgs, WaitArgs,
};
use super::bounds::COMMAND_TIMEOUT;
use super::identity::BrowserId;
use super::upload::UploadArgs;
use serde::{Deserialize, Serialize};
use std::time::Duration;

trait BrowserTarget {
    fn browser_target(&self) -> Option<&BrowserId>;
    fn command_timeout(&self) -> Duration {
        COMMAND_TIMEOUT
    }
}

impl BrowserTarget for NoBrowserArgs {
    fn browser_target(&self) -> Option<&BrowserId> {
        None
    }
}
impl BrowserTarget for NewTabArgs {
    fn browser_target(&self) -> Option<&BrowserId> {
        None
    }
}
macro_rules! direct_targets {
    ($($kind:ty),+) => {$(impl BrowserTarget for $kind {
        fn browser_target(&self) -> Option<&BrowserId> { Some(&self.browser_id) }
    })+};
}
direct_targets!(
    TabArgs,
    ElementArgs,
    TypeArgs,
    KeypressArgs,
    NavigateArgs,
    ScreenshotArgs,
    DragArgs,
    EvaluateArgs,
    ScrollArgs,
    ResizeArgs,
    LogsArgs
);
impl BrowserTarget for WaitArgs {
    fn browser_target(&self) -> Option<&BrowserId> {
        Some(&self.browser_id)
    }
    fn command_timeout(&self) -> Duration {
        self.budget.command_timeout()
    }
}
macro_rules! element_targets {
    ($($kind:ty),+) => {$(impl BrowserTarget for $kind {
        fn browser_target(&self) -> Option<&BrowserId> { Some(&self.target.browser_id) }
    })+};
}
element_targets!(ClickArgs, ValueArgs, UploadArgs);

macro_rules! browser_tools {
    ($($variant:ident($args:ty) => $name:literal, $doc:literal;)+) => {
        /// A closed command vocabulary prevents exposing an arbitrary CDP tunnel.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BrowserTool {
            $(#[doc = $doc] $variant,)+
        }

        /// Domain commands share one catalogue with the MCP name and scope vocabulary.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(tag="command",content="args",rename_all="snake_case")]
        #[cfg_attr(feature="ts",derive(ts_rs::TS))]
        pub enum BrowserCommand {
            $(#[doc=$doc] $variant($args),)+
        }

        impl BrowserCommand {
            /// A command cannot claim the name of another command's schema.
            #[must_use]
            pub const fn tool(&self)->BrowserTool {
                match self { $(Self::$variant(_) => BrowserTool::$variant,)+ }
            }

            /// The selected command owns its finite deadline, including explicit wait allowance.
            #[must_use]
            pub fn command_timeout(&self)->Duration {
                match self { $(Self::$variant(args) => args.command_timeout(),)+ }
            }

            /// Admission uses the same target projection for every command.
            #[must_use]
            pub fn target(&self)->Option<&BrowserId>{
                match self { $(Self::$variant(args) => args.browser_target(),)+ }
            }
        }

        /// Schema builders and policy filters use the same ordered catalogue.
        pub const BROWSER_TOOLS: &[BrowserTool] = &[$(BrowserTool::$variant,)+];

        impl BrowserTool {
            /// Preserve Paseo's exact MCP name at the serialization edge.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name,)+ }
            }

            /// Decode a tool name once; no caller-provided CDP method is accepted.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                match name { $($name => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

browser_tools! {
    ListTabs(NoBrowserArgs) => "browser_list_tabs", "Inventory is filtered before any target can be disclosed.";
    NewTab(NewTabArgs) => "browser_new_tab", "Background creation must not steal pane focus.";
    Snapshot(TabArgs) => "browser_snapshot", "A snapshot creates the element-reference generation.";
    Click(ClickArgs) => "browser_click", "Mouse input is resolved through a current element reference.";
    Fill(ValueArgs) => "browser_fill", "Form replacement is distinct from appending typed text.";
    Wait(WaitArgs) => "browser_wait", "A bounded predicate wait must not hold the actor indefinitely.";
    Type(TypeArgs) => "browser_type", "Text input can address the current focus or a specific element.";
    Keypress(KeypressArgs) => "browser_keypress", "Keys/chords require trusted input rather than page evaluation.";
    Navigate(NavigateArgs) => "browser_navigate", "Navigation must pass the destination policy again.";
    Back(TabArgs) => "browser_back", "Empty backward history is a typed refusal.";
    Forward(TabArgs) => "browser_forward", "Empty forward history is a typed refusal.";
    Reload(TabArgs) => "browser_reload", "Reload invalidates references even when the URL stays the same.";
    Screenshot(ScreenshotArgs) => "browser_screenshot", "A screenshot requires a real painted frame.";
    Upload(UploadArgs) => "browser_upload", "Only staged workspace-contained files reach the browser.";
    Select(ValueArgs) => "browser_select", "Selection validates an available option, not arbitrary HTML.";
    Hover(ElementArgs) => "browser_hover", "Hover preserves the reference's actionability checks.";
    Drag(DragArgs) => "browser_drag", "Both endpoints must be validated before the first pointer press.";
    Logs(LogsArgs) => "browser_logs", "A bounded log history must never become an unbounded tracing stream.";
    Evaluate(EvaluateArgs) => "browser_evaluate", "Evaluation is page code and never host-process code.";
    Scroll(ScrollArgs) => "browser_scroll", "Finite wheel deltas are distinct from untrusted key strings.";
    Resize(ResizeArgs) => "browser_resize", "The target viewport and the pane's remembered size change together.";
    CloseTab(TabArgs) => "browser_close_tab", "Closing a target must settle pending calls and frame watches.";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_names_are_unique_and_round_trip() {
        // Each source registration (including the three navigation entries)
        // must survive without adding an arbitrary passthrough fallback.
        let mut names = std::collections::HashSet::new();
        for tool in BROWSER_TOOLS {
            assert!(names.insert(tool.name()));
            assert_eq!(BrowserTool::from_name(tool.name()), Some(*tool));
        }
        assert_eq!(names.len(), 22);
        assert_eq!(BrowserTool::from_name("Browser.getVersion"), None);
        assert_eq!(BrowserTool::from_name("browser_new_tab "), None);
    }
}

#[cfg(test)]
mod command_tests {
    use super::super::arguments::{ClickCount, MouseButton};
    use super::*;

    #[test]
    fn invalid_quantities_cannot_enter_an_otherwise_valid_target_command() {
        let id = BrowserId::random();
        let request = serde_json::json!({"command":"resize","args":{"browser_id":id,"size":{"width":0,"height":1}}});
        assert!(serde_json::from_value::<BrowserCommand>(request).is_err());
        let empty = BrowserCommand::ListTabs(NoBrowserArgs {});
        assert_eq!(empty.tool(), BrowserTool::ListTabs);
        assert_eq!(empty.target(), None);
    }

    #[test]
    fn nested_element_arguments_still_project_the_authorized_target() {
        let id = BrowserId::random();
        let command = BrowserCommand::Click(ClickArgs {
            target: ElementArgs {
                browser_id: id.clone(),
                element: "@e1".parse().unwrap(),
            },
            button: MouseButton::Left,
            count: ClickCount::Single,
            modifiers: Vec::new(),
        });
        assert_eq!(command.target(), Some(&id));
        assert_eq!(command.tool(), BrowserTool::Click);
        assert_eq!(
            serde_json::from_str::<BrowserCommand>(&serde_json::to_string(&command).unwrap())
                .unwrap(),
            command
        );
    }
}
