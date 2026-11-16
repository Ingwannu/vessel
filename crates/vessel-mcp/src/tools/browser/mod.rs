//! MCP's browser JSON edge, staged until shared policy and qualified node wiring.
//!
//! Removal of outer cfg(test): `browser: wire the qualified node browser actor`,
//! together with W18 `ToolId::Browser` admission. This module does not register or
//! advertise tools. Names come only from `BrowserTool`; JSON is inspected only at
//! this foreign edge, then converted into the node's closed command vocabulary.

mod output;
mod schema;

use rmcp::model::JsonObject;
use serde_json::Value;
use vessel_proto::browser::{
    BrowserCommand, BrowserId, BrowserTool, BrowserUrl, ClickArgs, ClickCount, CssDelta, DragArgs,
    ElementArgs, ElementRef, EvaluateArgs, InputText, InvalidBrowserBound, InvalidBrowserIdentity,
    InvalidBrowserInput, InvalidBrowserUrl, InvalidUpload, KeyModifier, KeyModifiers, KeypressArgs,
    LogCount, LogsArgs, MouseButton, NavigateArgs, NewTabArgs, NoBrowserArgs, RequiredInput,
    ResizeArgs, ScreenshotArgs, ScreenshotExtent, ScrollArgs, TabArgs, TypeArgs, UploadArgs,
    UploadFiles, UploadPath, ValueArgs, ViewportSide, ViewportSize, WaitArgs, WaitBudget, WaitFor,
    WaitTimeout,
};
use vessel_proto::conversation::FiniteNumber;

#[derive(Debug, thiserror::Error)]
pub(super) enum ArgumentError {
    #[error("browser argument `{0}` is missing or has the wrong type")]
    Field(&'static str),
    #[error("invalid browser identity")]
    Identity(#[source] InvalidBrowserIdentity),
    #[error("invalid browser input")]
    Input(#[source] InvalidBrowserInput),
    #[error("invalid browser quantity")]
    Bound(#[source] InvalidBrowserBound),
    #[error("invalid browser URL")]
    Url(#[source] InvalidBrowserUrl),
    #[error("invalid untrusted upload intent")]
    Upload(#[source] InvalidUpload),
    #[error("browser_wait requires exactly one nonempty text or URL condition")]
    Wait,
}

/// Unknown extra selectors are discarded as in Paseo; none become node authority.
pub(super) fn decode(
    tool: BrowserTool,
    args: &JsonObject,
) -> Result<BrowserCommand, ArgumentError> {
    use BrowserCommand as C;
    use BrowserTool as T;
    Ok(match tool {
        T::ListTabs => C::ListTabs(NoBrowserArgs {}),
        T::NewTab => C::NewTab(NewTabArgs {
            url: optional_string(args, "url")?
                .map(|url| BrowserUrl::from_tool(url.trim()).map_err(ArgumentError::Url))
                .transpose()?,
        }),
        T::Snapshot => C::Snapshot(tab(args)?),
        T::Click => C::Click(ClickArgs {
            target: element(args)?,
            button: button(args)?,
            count: if flag(args, "doubleClick")? {
                ClickCount::Double
            } else {
                ClickCount::Single
            },
            modifiers: modifiers(args)?,
        }),
        T::Fill => C::Fill(value(args)?),
        T::Wait => C::Wait(wait(args)?),
        T::Type => C::Type(TypeArgs {
            browser_id: id(args)?,
            element: optional_ref(args)?,
            text: input(args, "text")?,
        }),
        T::Keypress => C::Keypress(KeypressArgs {
            browser_id: id(args)?,
            element: optional_ref(args)?,
            key: required_input(args, "key")?,
        }),
        T::Navigate => C::Navigate(NavigateArgs {
            browser_id: id(args)?,
            url: BrowserUrl::from_tool(string(args, "url")?.trim()).map_err(ArgumentError::Url)?,
        }),
        T::Back => C::Back(tab(args)?),
        T::Forward => C::Forward(tab(args)?),
        T::Reload => C::Reload(tab(args)?),
        T::Screenshot => C::Screenshot(ScreenshotArgs {
            browser_id: id(args)?,
            extent: if flag(args, "fullPage")? {
                ScreenshotExtent::Page
            } else {
                ScreenshotExtent::Viewport
            },
        }),
        T::Upload => C::Upload(upload(args)?),
        T::Select => C::Select(value(args)?),
        T::Hover => C::Hover(element(args)?),
        T::Drag => C::Drag(DragArgs {
            browser_id: id(args)?,
            source: reference(args, "sourceRef")?,
            destination: reference(args, "targetRef")?,
        }),
        T::Logs => C::Logs(LogsArgs {
            browser_id: id(args)?,
            count: integer(args, "maxEntries")?
                .map(LogCount::try_from)
                .transpose()
                .map_err(ArgumentError::Bound)?
                .unwrap_or_default(),
        }),
        T::Evaluate => C::Evaluate(EvaluateArgs {
            browser_id: id(args)?,
            function: required_input(args, "function")?,
            element: optional_ref(args)?,
        }),
        T::Scroll => C::Scroll(ScrollArgs {
            browser_id: id(args)?,
            element: optional_ref(args)?,
            delta_x: delta(args, "deltaX")?,
            delta_y: delta(args, "deltaY")?,
        }),
        T::Resize => C::Resize(ResizeArgs {
            browser_id: id(args)?,
            size: ViewportSize {
                width: side(args, "width")?,
                height: side(args, "height")?,
            },
        }),
        T::CloseTab => C::CloseTab(tab(args)?),
    })
}
fn string<'a>(args: &'a JsonObject, key: &'static str) -> Result<&'a str, ArgumentError> {
    optional_string(args, key)?.ok_or(ArgumentError::Field(key))
}
fn optional_string<'a>(
    args: &'a JsonObject,
    key: &'static str,
) -> Result<Option<&'a str>, ArgumentError> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(ArgumentError::Field(key)),
    }
}
fn id(args: &JsonObject) -> Result<BrowserId, ArgumentError> {
    string(args, "browserId")?
        .parse()
        .map_err(ArgumentError::Identity)
}
fn tab(args: &JsonObject) -> Result<TabArgs, ArgumentError> {
    Ok(TabArgs {
        browser_id: id(args)?,
    })
}
fn reference(args: &JsonObject, key: &'static str) -> Result<ElementRef, ArgumentError> {
    string(args, key)?.parse().map_err(ArgumentError::Identity)
}
fn optional_ref(args: &JsonObject) -> Result<Option<ElementRef>, ArgumentError> {
    optional_string(args, "ref")?
        .map(|value| value.parse().map_err(ArgumentError::Identity))
        .transpose()
}
fn element(args: &JsonObject) -> Result<ElementArgs, ArgumentError> {
    Ok(ElementArgs {
        browser_id: id(args)?,
        element: reference(args, "ref")?,
    })
}
fn input(args: &JsonObject, key: &'static str) -> Result<InputText, ArgumentError> {
    InputText::try_from(string(args, key)?.to_owned()).map_err(ArgumentError::Input)
}
fn required_input(args: &JsonObject, key: &'static str) -> Result<RequiredInput, ArgumentError> {
    RequiredInput::try_from(string(args, key)?.to_owned()).map_err(ArgumentError::Input)
}
fn value(args: &JsonObject) -> Result<ValueArgs, ArgumentError> {
    Ok(ValueArgs {
        target: element(args)?,
        value: input(args, "value")?,
    })
}
fn flag(args: &JsonObject, key: &'static str) -> Result<bool, ArgumentError> {
    match args.get(key) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(ArgumentError::Field(key)),
    }
}
fn integer(args: &JsonObject, key: &'static str) -> Result<Option<u32>, ArgumentError> {
    args.get(key)
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(ArgumentError::Field(key))
        })
        .transpose()
}
fn side(args: &JsonObject, key: &'static str) -> Result<ViewportSide, ArgumentError> {
    ViewportSide::try_from(integer(args, key)?.ok_or(ArgumentError::Field(key))?)
        .map_err(ArgumentError::Bound)
}
fn delta(args: &JsonObject, key: &'static str) -> Result<CssDelta, ArgumentError> {
    args.get(key)
        .and_then(Value::as_f64)
        .and_then(FiniteNumber::new)
        .map(CssDelta)
        .ok_or(ArgumentError::Field(key))
}
fn button(args: &JsonObject) -> Result<MouseButton, ArgumentError> {
    match optional_string(args, "button")? {
        None | Some("left") => Ok(MouseButton::Left),
        Some("right") => Ok(MouseButton::Right),
        Some("middle") => Ok(MouseButton::Middle),
        Some(_) => Err(ArgumentError::Field("button")),
    }
}
fn modifiers(args: &JsonObject) -> Result<KeyModifiers, ArgumentError> {
    let mut modifiers = KeyModifiers::default();
    let Some(value) = args.get("modifiers") else {
        return Ok(modifiers);
    };
    for value in value.as_array().ok_or(ArgumentError::Field("modifiers"))? {
        let modifier = match value.as_str() {
            Some("Alt") => KeyModifier::Alt,
            Some("Control") => KeyModifier::Control,
            Some("Meta") => KeyModifier::Meta,
            Some("Shift") => KeyModifier::Shift,
            _ => return Err(ArgumentError::Field("modifiers")),
        };
        modifiers.include(modifier);
    }
    Ok(modifiers)
}
fn wait(args: &JsonObject) -> Result<WaitArgs, ArgumentError> {
    let condition = match (
        optional_string(args, "text")?,
        optional_string(args, "url")?,
    ) {
        (Some(_), None) => WaitFor::Text(required_input(args, "text")?),
        (None, Some(_)) => WaitFor::Url(required_input(args, "url")?),
        (Some(_), Some(_)) | (None, None) => return Err(ArgumentError::Wait),
    };
    let budget = match integer(args, "timeoutMs")? {
        Some(value) => {
            WaitBudget::Explicit(WaitTimeout::try_from(value).map_err(ArgumentError::Bound)?)
        }
        None => WaitBudget::Default,
    };
    Ok(WaitArgs {
        browser_id: id(args)?,
        condition,
        budget,
    })
}
fn upload(args: &JsonObject) -> Result<UploadArgs, ArgumentError> {
    let values = args
        .get("filePaths")
        .and_then(Value::as_array)
        .ok_or(ArgumentError::Field("filePaths"))?;
    UploadFiles::check_count(values.len()).map_err(ArgumentError::Upload)?;
    let files = values
        .iter()
        .map(|value| {
            let path = value.as_str().ok_or(ArgumentError::Field("filePaths"))?;
            UploadPath::try_from(path.to_owned()).map_err(ArgumentError::Upload)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UploadArgs {
        target: element(args)?,
        files: UploadFiles::try_from(files).map_err(ArgumentError::Upload)?,
    })
}

#[cfg(test)]
mod tests;
