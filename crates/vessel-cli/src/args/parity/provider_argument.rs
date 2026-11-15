//! Provider/model shorthand decodes into existing supported driver/model inputs once.
//! Full provider identity/configuration remains W22's node contract.
use std::str::FromStr;
use vessel_proto::{HarnessRef, ModelId};
const MAX_MODEL_CHARS: usize = 1024;
#[derive(Debug, Clone)]
pub(crate) struct ProviderArgument {
    driver: HarnessRef,
    model: Option<ModelId>,
}
#[derive(Debug, thiserror::Error)]
pub(crate) enum InvalidProviderArgument {
    #[error("provider must identify a supported node driver")]
    Driver(#[source] vessel_proto::agents::UnknownAgent),
    #[error("provider/model shorthand must contain a bounded nonempty model without controls")]
    Model,
}
impl FromStr for ProviderArgument {
    type Err = InvalidProviderArgument;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (driver, model) = text
            .split_once('/')
            .map_or((text, None), |(driver, model)| (driver, Some(model)));
        let driver = driver.parse().map_err(InvalidProviderArgument::Driver)?;
        let model = model
            .map(|text| {
                if text.is_empty()
                    || text.encode_utf16().count() > MAX_MODEL_CHARS
                    || text.chars().any(char::is_control)
                {
                    Err(InvalidProviderArgument::Model)
                } else {
                    Ok(ModelId::new(text))
                }
            })
            .transpose()?;
        Ok(Self { driver, model })
    }
}
impl ProviderArgument {
    pub(crate) fn into_parts(self) -> (HarnessRef, Option<ModelId>) {
        (self.driver, self.model)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shorthand_splits_once_and_preserves_the_entire_model() {
        let (driver, model) = "codex/team/model"
            .parse::<ProviderArgument>()
            .unwrap()
            .into_parts();
        assert_eq!(driver, HarnessRef::from(vessel_proto::HarnessKind::Codex));
        assert_eq!(model.unwrap().as_str(), "team/model");
        assert!("codex/".parse::<ProviderArgument>().is_err());
    }
}
