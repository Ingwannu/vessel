//! Table/structured/ID output shares one typed result, never a JSON value tree.

use crate::{
    args::parity::{Presentation, output::Headers},
    error::CliError,
    io,
};
use serde::Serialize;

pub(crate) trait Row: Serialize {
    fn id(&self) -> String;
    fn headers() -> &'static [&'static str];
    fn cells(&self) -> Vec<String>;
}

pub(crate) async fn list<T: Row>(presentation: Presentation, rows: &[T]) -> Result<(), CliError> {
    let bytes = render_list(presentation, rows)?;
    Ok(io::out(&bytes).await?)
}

pub(crate) fn render_list<T: Row>(
    presentation: Presentation,
    rows: &[T],
) -> Result<Vec<u8>, CliError> {
    match presentation {
        Presentation::Json => json(rows),
        // JSON's flow collection syntax is also valid YAML 1.2. This avoids
        // a second serializer/dependency and keeps typed field fidelity.
        Presentation::Yaml => {
            let mut bytes = b"---\n".to_vec();
            bytes.extend(json(rows)?);
            Ok(bytes)
        }
        Presentation::Quiet => {
            let mut text = String::new();
            for row in rows {
                text.push_str(&row.id());
                text.push('\n');
            }
            Ok(text.into_bytes())
        }
        Presentation::Table(headers) => {
            let mut text = String::new();
            if headers == Headers::Show && !rows.is_empty() {
                text.push_str(&T::headers().join("  "));
                text.push('\n');
            }
            for row in rows {
                text.push_str(
                    &row.cells()
                        .iter()
                        .map(|cell| escape(cell))
                        .collect::<Vec<_>>()
                        .join("  "),
                );
                text.push('\n');
            }
            Ok(text.into_bytes())
        }
    }
}

pub(crate) async fn single<T: Serialize>(
    presentation: Presentation,
    id: &str,
    value: &T,
) -> Result<(), CliError> {
    let mut bytes = match presentation {
        Presentation::Quiet if id.is_empty() => return Ok(()),
        Presentation::Quiet => format!("{id}\n").into_bytes(),
        Presentation::Yaml => {
            let mut bytes = b"---\n".to_vec();
            bytes.extend(json(value)?);
            bytes
        }
        Presentation::Json | Presentation::Table(_) => json(value)?,
    };
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    Ok(io::out(&bytes).await?)
}
fn json<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, CliError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(CliError::ParityEncode)?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

/// A success receipt can only be constructed after the command's typed ack.
pub(crate) struct Success;
impl Serialize for Success {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(true)
    }
}

/// Runtime errors preserve a stable envelope instead of corrupting structured stdout.
pub(crate) async fn error(presentation: Presentation, error: &CliError) -> std::io::Result<()> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        error: Failure<'a>,
    }
    #[derive(Serialize)]
    struct Failure<'a> {
        code: vessel_proto::ErrorCode,
        message: &'a str,
    }
    let code = if matches!(error, CliError::ParityBlocked(_)) {
        vessel_proto::ErrorCode::Unsupported
    } else if matches!(
        error,
        CliError::Usage(_)
            | CliError::ParityLabel(_)
            | CliError::ParityHostText(_)
            | CliError::ParityCron(_)
            | CliError::ParityAutomationText(_)
            | CliError::ParityCloneUrl(_)
    ) {
        vessel_proto::ErrorCode::InvalidRequest
    } else if matches!(
        error,
        CliError::ParitySnapshotTimeout(_)
            | CliError::ParityPromptReadTimeout(_)
            | CliError::ParityCloneTimeout { .. }
            | CliError::ParityImageTimeout { .. }
    ) {
        vessel_proto::ErrorCode::Unavailable
    } else if matches!(error, CliError::NoSuchSession(_)) {
        vessel_proto::ErrorCode::NotFound
    } else if let CliError::Call(vessel_client::CallError::Remote(failure)) = error {
        failure.code
    } else {
        vessel_proto::ErrorCode::Internal
    };
    let message = crate::chain(error);
    match presentation {
        Presentation::Json | Presentation::Yaml => {
            let envelope = Envelope {
                error: Failure {
                    code,
                    message: &message,
                },
            };
            let text = serde_json::to_string(&envelope).map_err(std::io::Error::other)?;
            let prefix = if presentation == Presentation::Yaml {
                "---\n"
            } else {
                ""
            };
            io::note(&format!("{prefix}{text}\n")).await
        }
        Presentation::Quiet | Presentation::Table(_) => {
            io::note(&format!("error: {message}\n")).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Serialize)]
    struct Item {
        id: u32,
        text: &'static str,
    }
    impl Row for Item {
        fn id(&self) -> String {
            self.id.to_string()
        }
        fn headers() -> &'static [&'static str] {
            &["ID", "TEXT"]
        }
        fn cells(&self) -> Vec<String> {
            vec![self.id.to_string(), self.text.to_owned()]
        }
    }
    #[test]
    fn output_keeps_structured_fields_and_quiet_ignores_format() {
        let rows = [Item {
            id: 7,
            text: "한글\n\u{1b}[31m",
        }];
        assert_eq!(render_list(Presentation::Quiet, &rows).unwrap(), b"7\n");
        let json = render_list(Presentation::Json, &rows).unwrap();
        assert!(String::from_utf8(json).unwrap().contains("한글"));
        let table =
            String::from_utf8(render_list(Presentation::Table(Headers::Hide), &rows).unwrap())
                .unwrap();
        assert!(!table.contains('\u{1b}'));
        assert!(!table.contains("ID"));
        let yaml = render_list(Presentation::Yaml, &rows).unwrap();
        assert!(yaml.starts_with(b"---\n["));
    }
}
