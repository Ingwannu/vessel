//! Existing model routes are the first consumer of the shared output surface.
//! This renders configured routes, not provider availability or provider discovery.

use crate::{
    args::{
        Target,
        parity::{Presentation, output::Headers},
    },
    connect::connect,
    error::CliError,
    io,
    parity::output::{self, Row},
    render,
};
use serde::Serialize;
use vessel_proto::ModelRoute;

#[derive(Serialize)]
#[serde(transparent)]
struct ModelRow<'a>(&'a ModelRoute);
impl Row for ModelRow<'_> {
    fn id(&self) -> String {
        self.0.name.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["NAME", "PROVIDER", "API", "MODEL", "SMALL_MODEL"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.0.name.to_string(),
            self.0.provider.to_string(),
            render::api(self.0.api).to_owned(),
            self.0.model.to_string(),
            self.0
                .small_model
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        ]
    }
}

pub(crate) async fn run(target: &Target, presentation: Presentation) -> Result<(), CliError> {
    let routes = connect(target).await?.client.models().await?;
    // Preserve the native command's default text, including its empty explanation.
    // New output modes go through the same formatter used by parity commands.
    match presentation {
        Presentation::Table(Headers::Show) => {
            Ok(io::out(render::models(&routes).as_bytes()).await?)
        }
        Presentation::Json | Presentation::Yaml => output::single(presentation, "", &routes).await,
        Presentation::Table(Headers::Hide) | Presentation::Quiet => {
            let rows: Vec<_> = routes.iter().map(ModelRow).collect();
            output::list(presentation, &rows).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_output_model_rows_keep_wire_fields_and_route_identity() {
        let route: ModelRoute = serde_json::from_str(r#"{"name":"work","provider":"gateway","api":"openai_responses","model":"model","small_model":"small"}"#).unwrap();
        let rows = [ModelRow(&route)];
        assert_eq!(
            output::render_list(Presentation::Quiet, &rows).unwrap(),
            b"work\n"
        );
        assert_eq!(
            serde_json::to_string(&rows[0]).unwrap(),
            serde_json::to_string(&route).unwrap()
        );
        assert_eq!(
            rows[0].cells(),
            ["work", "gateway", "openai-responses", "model", "small"]
        );
    }
}
