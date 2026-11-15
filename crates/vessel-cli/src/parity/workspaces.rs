//! Consume only W11's announced typed requests; never derive names or placement from cwd.
use super::output::{self, Row};
use crate::{
    args::{
        Target,
        parity::{Presentation, WorkspaceCommand},
    },
    connect::connect,
    error::CliError,
};
use serde::Serialize;
use vessel_proto::{SessionTitle, WorkspaceId, registry::RenameWorkspace};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    command: WorkspaceCommand,
) -> Result<(), CliError> {
    command.validate()?;
    let WorkspaceCommand::Rename { id, title, reset } = command else {
        return Err(CliError::ParityBlocked(
            "workspace operation awaits its canonical placement/lifecycle/display contract",
        ));
    };
    let request = rename_request(&id, title, reset)?;
    let client = connect(target).await?.client;
    // The shared Request gate refuses None/older protocol before this sends any
    // mutation. No generic-unsupported fallback or local title store is allowed.
    let applied = client.call(request.clone()).await?;
    let receipt = Renamed::checked(&request, applied.id, applied.title)?;
    output::record(presentation, &receipt).await
}

fn rename_request(
    id: &str,
    title: Option<SessionTitle>,
    reset: bool,
) -> Result<RenameWorkspace, CliError> {
    let title = match (title, reset) {
        (Some(title), false) => Some(title),
        (None, true) => None,
        _ => {
            return Err(CliError::Usage(
                "give a workspace title or --reset, not both",
            ));
        }
    };
    let workspace = id.parse().map_err(|_invalid| {
        CliError::Usage("workspace rename requires a complete workspace UUID on the selected node")
    })?;
    Ok(RenameWorkspace { workspace, title })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Renamed {
    workspace_id: WorkspaceId,
    title: Option<SessionTitle>,
}
impl Renamed {
    fn checked(
        expected: &RenameWorkspace,
        workspace_id: WorkspaceId,
        title: Option<SessionTitle>,
    ) -> Result<Self, CliError> {
        if workspace_id != expected.workspace || title != expected.title {
            return Err(CliError::ParityInvalidReceipt(
                "workspace rename target/title differ",
            ));
        }
        Ok(Self {
            workspace_id,
            title,
        })
    }
}
impl Row for Renamed {
    fn id(&self) -> String {
        self.workspace_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["WORKSPACE ID", "TITLE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.id(),
            self.title
                .as_ref()
                .map_or_else(String::new, ToString::to_string),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::parity::output::Headers;
    #[test]
    fn rename_receipt_requires_the_acknowledged_target_and_raw_title_override() {
        let id = WorkspaceId::random();
        let request =
            rename_request(&id.to_string(), Some("  renamed  ".parse().unwrap()), false).unwrap();
        assert_eq!(request.title.as_ref().unwrap().to_string(), "renamed");
        assert!(matches!(
            Renamed::checked(&request, WorkspaceId::random(), request.title.clone()),
            Err(CliError::ParityInvalidReceipt(_))
        ));
        assert!(matches!(
            Renamed::checked(&request, id, None),
            Err(CliError::ParityInvalidReceipt(_))
        ));
        let reset = rename_request(&id.to_string(), None, true).unwrap();
        let receipt = Renamed::checked(&reset, id, None).unwrap();
        let json = serde_json::to_string(&receipt).unwrap();
        assert!(json.contains("\"workspaceId\""));
        assert!(json.contains("\"title\":null"));
        let table = output::render_list(Presentation::Table(Headers::Show), &[receipt]).unwrap();
        assert!(table.starts_with(b"WORKSPACE ID  TITLE\n"));
        assert!(rename_request("a-prefix", None, true).is_err());
        assert!(rename_request(&id.to_string(), None, false).is_err());
        assert!(rename_request(&id.to_string(), Some("name".parse().unwrap()), true).is_err());
    }
}
