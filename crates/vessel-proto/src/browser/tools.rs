//! One catalogue maps the reference tool names to typed browser operations.
//!
//! This does not advertise the tools: the MCP policy must first establish a
//! ready engine and the session-start setting/grant. Unknown names are rejected
//! at the MCP decoding edge rather than passed through as arbitrary CDP methods.

macro_rules! browser_tools {
    ($($variant:ident => $name:literal, $doc:literal;)+) => {
        /// A closed command vocabulary prevents exposing an arbitrary CDP tunnel.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BrowserTool {
            $(#[doc = $doc] $variant,)+
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
    ListTabs => "browser_list_tabs", "Inventory is filtered before any target can be disclosed.";
    NewTab => "browser_new_tab", "Background creation must not steal pane focus.";
    Snapshot => "browser_snapshot", "A snapshot creates the element-reference generation.";
    Click => "browser_click", "Mouse input is resolved through a current element reference.";
    Fill => "browser_fill", "Form replacement is distinct from appending typed text.";
    Wait => "browser_wait", "A bounded predicate wait must not hold the actor indefinitely.";
    Type => "browser_type", "Text input can address the current focus or a specific element.";
    Keypress => "browser_keypress", "Keys/chords require trusted input rather than page evaluation.";
    Navigate => "browser_navigate", "Navigation must pass the destination policy again.";
    Back => "browser_back", "Empty backward history is a typed refusal.";
    Forward => "browser_forward", "Empty forward history is a typed refusal.";
    Reload => "browser_reload", "Reload invalidates references even when the URL stays the same.";
    Screenshot => "browser_screenshot", "A screenshot requires a real painted frame.";
    Upload => "browser_upload", "Only staged workspace-contained files reach the browser.";
    Select => "browser_select", "Selection validates an available option, not arbitrary HTML.";
    Hover => "browser_hover", "Hover preserves the reference's actionability checks.";
    Drag => "browser_drag", "Both endpoints must be validated before the first pointer press.";
    Logs => "browser_logs", "A bounded log history must never become an unbounded tracing stream.";
    Evaluate => "browser_evaluate", "Evaluation is page code and never host-process code.";
    Scroll => "browser_scroll", "Finite wheel deltas are distinct from untrusted key strings.";
    Resize => "browser_resize", "The target viewport and the pane's remembered size change together.";
    CloseTab => "browser_close_tab", "Closing a target must settle pending calls and frame watches.";
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
