//! What a node can start: its harnesses, its profiles, and the options a
//! harness offers before a session exists. Not starting one.
//!
//! `profile.list` returns names, labels and which variables a profile
//! sets, never a value. `model.list` is the node's routing table, so it
//! is not offered here. Models a harness offers are in `harness_options`.
//! The probe behind that call is the node's.
//!
//! `tools` builds MCP's JSON Schema, a tree. Each call returns a protocol
//! type, and the server encodes it.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::{
    GetHarnessOptions, Harness, HarnessOptions, HarnessRef, ListHarnesses, ListProfiles,
    ProfileInfo, ProfileName,
};

use super::{ToolError, VesselTools, text, tool};

impl VesselTools {
    pub(super) async fn list_harnesses(
        &self,
        args: &JsonObject,
    ) -> Result<Vec<Harness>, ToolError> {
        let node = self.target(args).await?;
        self.on(node, ListHarnesses).await
    }

    pub(super) async fn list_profiles(
        &self,
        args: &JsonObject,
    ) -> Result<Vec<ProfileInfo>, ToolError> {
        let node = self.target(args).await?;
        self.on(
            node,
            ListProfiles {
                harness: optional_harness(args)?,
            },
        )
        .await
    }

    pub(super) async fn harness_options(
        &self,
        args: &JsonObject,
    ) -> Result<HarnessOptions, ToolError> {
        let node = self.target(args).await?;
        self.on(
            node,
            GetHarnessOptions {
                harness: harness(args)?,
                profile: optional_profile(args)?,
            },
        )
        .await
    }
}

fn harness(args: &JsonObject) -> Result<HarnessRef, ToolError> {
    parse(super::required(args, "harness")?)
}

fn optional_harness(args: &JsonObject) -> Result<Option<HarnessRef>, ToolError> {
    text(args, "harness")?.map(parse).transpose()
}

fn optional_profile(args: &JsonObject) -> Result<Option<ProfileName>, ToolError> {
    match text(args, "profile")? {
        None => Ok(None),
        Some(profile) => profile
            .parse()
            .map(Some)
            .map_err(|error| ToolError::Arguments(format!("profile: {error}"))),
    }
}

fn parse(text: &str) -> Result<HarnessRef, ToolError> {
    text.parse()
        .map_err(|error| ToolError::Arguments(format!("harness: {error}")))
}

pub(super) fn tools(node: &Value) -> Vec<Tool> {
    let harness = json!({
        "type": "string",
        "description": "claude, codex, gemini, opencode, or an agent added on the node."
    });
    vec![
        tool(
            "list_harnesses",
            "The agents this node can start, and whether each is installed.",
            json!({ "node": node }),
            &[],
        ),
        tool(
            "list_profiles",
            "The node's profiles: a name, a label and which variables it sets. Never a \
             value. Leave `harness` out for every profile.",
            json!({ "node": node, "harness": harness }),
            &[],
        ),
        tool(
            "harness_options",
            "What an agent offers before a session of it exists (its models, efforts and \
             modes), under `profile` or its default. The node may answer from a recent \
             probe. This does not start a session you own.",
            json!({
                "node": node,
                "harness": harness,
                "profile": { "type": "string", "description": "One of the node's profiles." }
            }),
            &["harness"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn args(value: &Value) -> JsonObject {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn a_harness_name_parses_and_a_blank_one_does_not() {
        let alias = harness(&args(&json!({ "harness": "claude" }))).unwrap();
        let wire = harness(&args(&json!({ "harness": "claude-code" }))).unwrap();
        assert_eq!(alias, wire);
        assert!(alias.known().is_some());
        assert!(harness(&args(&json!({ "harness": "" }))).is_err());
        assert!(optional_harness(&args(&json!({}))).unwrap().is_none());
    }
}
