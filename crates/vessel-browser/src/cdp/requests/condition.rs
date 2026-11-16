//! A fixed read-only condition probe; caller text is serialized data, never source.
use super::{Evaluated, Method, Request, sealed};
use crate::cdp::identity::ContextId;
use serde::Serialize;
use vessel_proto::browser::WaitFor;

// Independently authored from the wait behavior, not reference implementation.
const SOURCE: &str = r#"function(condition) {
    const observed = condition.kind === "url"
        ? location.href
        : (document.body ? document.body.innerText : "");
    return observed.includes(condition.value);
}"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConditionProbe<'a> {
    function_declaration: &'static str,
    execution_context_id: ContextId,
    arguments: [Argument<'a>; 1],
    return_by_value: bool,
    await_promise: bool,
    user_gesture: bool,
}
#[derive(Serialize)]
struct Argument<'a> {
    value: &'a WaitFor,
}
impl<'a> ConditionProbe<'a> {
    pub(crate) const fn new(context: ContextId, condition: &'a WaitFor) -> Self {
        Self {
            function_declaration: SOURCE,
            execution_context_id: context,
            arguments: [Argument { value: condition }],
            return_by_value: true,
            await_promise: false,
            user_gesture: false,
        }
    }
}
impl std::fmt::Debug for ConditionProbe<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BrowserConditionProbe([private])")
    }
}
impl sealed::Sealed for ConditionProbe<'_> {}
impl Request for ConditionProbe<'_> {
    type Response = Evaluated;
    const METHOD: Method = Method::CallFunction;
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::browser::RequiredInput;

    #[test]
    fn hostile_condition_is_one_data_argument_without_source_or_gesture_authority() {
        let needle = "'); throw new Error('fixture only'); //";
        let condition = WaitFor::Text(RequiredInput::try_from(needle.to_owned()).unwrap());
        let context = serde_json::from_str("7").unwrap();
        let request = ConditionProbe::new(context, &condition);
        assert!(!format!("{request:?}").contains(needle));
        let encoded = serde_json::to_value(request).unwrap();
        assert_eq!(encoded["functionDeclaration"], SOURCE);
        assert!(!SOURCE.contains(needle));
        assert_eq!(encoded["arguments"][0]["value"]["value"], needle);
        assert_eq!(encoded["userGesture"], false);
        assert_eq!(encoded["awaitPromise"], false);
        assert_eq!(encoded["returnByValue"], true);
    }
}
