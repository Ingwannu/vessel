//! Fixed native output; no external template, runtime or package download.
use super::scaffold::InitError;
use serde::Serialize;
use vessel_proto::{
    PROTOCOL_VERSION, ProtocolVersion,
    plugins::{ClientDocument, PluginId},
};

#[derive(Serialize)]
struct Manifest<'a> {
    format: &'static str,
    id: &'a PluginId,
    requirements: Requirements,
    client: &'static str,
}
#[derive(Serialize)]
struct Requirements {
    node: ProtocolVersion,
    client: ProtocolVersion,
}

const DOCUMENT: &str = r#"{
  "methods": [],
  "contributions": [{
    "id": "welcome",
    "title": "Welcome",
    "content": {
      "kind": "screen",
      "surface": {"controls": [{
        "id": "greeting",
        "content": {"kind": "text", "text": "Hello from your native plugin."}
      }]}
    }
  }],
  "themes": []
}"#;

pub(super) fn files(id: &PluginId) -> Result<Vec<(&'static str, Vec<u8>)>, InitError> {
    let manifest = Manifest {
        format: "vessel.plugin/v1",
        id,
        requirements: Requirements {
            node: PROTOCOL_VERSION,
            client: PROTOCOL_VERSION,
        },
        client: "client.json",
    };
    let document = ClientDocument::decode(DOCUMENT.as_bytes()).map_err(InitError::Document)?;
    Ok(vec![
        ("vessel-plugin.json", json(&manifest)?),
        ("client.json", json(&document)?),
        ("README.md", README.as_bytes().to_vec()),
        ("HELPER.md", HELPER.as_bytes().to_vec()),
    ])
}

fn json(value: &impl Serialize) -> Result<Vec<u8>, InitError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(InitError::Encode)?;
    bytes.push(b'\n');
    Ok(bytes)
}

const README: &str = "# Native Vessel plugin\n\n\
This directory is an author scaffold. Creating it does not install, enable or\n\
execute a plugin, contact a node, download packages, or run a build command.\n\n\
`vessel-plugin.json` declares the native vessel.plugin/v1 format. Its client\n\
entry is `client.json`, a complete validated declarative document. The Welcome\n\
screen has no callbacks and needs no helper. Controls use Vessel's shared kit;\n\
React, JavaScript callbacks, CSS and executable client code are not supported.\n\n\
Keep stable contribution/control identities when editing labels. A declaration\n\
is a complete replacement, not a patch. Unknown fields and unsupported kinds\n\
refuse the complete document. This example uses literal English labels; it\n\
does not claim dynamic localization, workspace access or server capabilities.\n\n\
Installation requires a node build with the native plugin feature assigned and\n\
enabled. This scaffold does not imply that the installed CLI/node already has\n\
that capability. The protocol floors describe this generator's value format.\n\n\
If you add a helper, read HELPER.md and explicitly build/declare its exact\n\
platform executable. Helpers and preparation commands are trusted unsandboxed\n\
code running with the node account's filesystem and network access, including\n\
credentials accessible to that account. Review dependencies and updates too.\n";

const HELPER: &str = "# Optional helper contract\n\n\
This static scaffold declares no helper and registers no methods. This file\n\
is passive protocol guidance; no interpreter, adapter or compiler is included.\n\n\
A helper is a separately prepared executable, explicitly declared in manifest\n\
helpers with target, protocol 1, package-relative program and argument list.\n\
Currently only x86_64-unknown-linux-gnu and x86_64-pc-windows-msvc may activate;\n\
other author target labels do not claim release/runtime support. A Node/TS\n\
adapter must be supplied and tested separately, without implicit downloads.\n\n\
Stdin/stdout carry private V1 JSON messages, each prefixed by a four-byte\n\
unsigned big-endian body length, 1 through 4194304 bytes. Stdout contains only\n\
protocol frames. Stderr is bounded private management diagnostics. Read each\n\
complete frame; do not retry a partially written frame on the same pipe.\n\n\
First read initialize with protocol 1 and the node-assigned installation\n\
address. Reply with ready, protocol 1, a complete client document and hooks\n\
array (empty for this example). Do not choose your own installation identity.\n\
All client methods must be explicitly declared. A request carries its nonzero\n\
request correlation and a closed invoke/hook call. Return result with that\n\
same correlation and bounded output, or error with a safe failure kind and\n\
private diagnostic. Observe cancel and shutdown; cancellation alone is not\n\
an acknowledgement of completion. Never return a never-issued request id.\n\n\
The host bounds initialization and callbacks to 30 seconds and retains\n\
admission until actual settlement or retirement. Semantic registration\n\
changes replace the complete document and rotate the public generation.\n\
Tools, provider sessions, usage reports and node API broker operations require\n\
their actual owning adapters; these initial messages do not grant them.\n\
The helper runs unsandboxed. The host supplies a cleared minimal environment;\n\
neither the protocol nor a client action grants additional node permissions.\n";
