//! Lenient file projection beside the strict wire patch, sharing its value types.
//!
//! Unknown fields cannot invalidate all saved settings. The existing settings
//! owner detects what was ignored and copies that later file before replacing it.

use serde::Deserialize;
use vessel_proto::dictation::{
    DictationChange, DictationProvider, DictationSwitch, EndpointBase, EndpointModel, Language,
    LocalModelId, SecretReference, SetDictation,
};
use vessel_proto::settings::SetNodeSettings;

#[derive(Deserialize)]
pub(super) struct FileSettings {
    #[serde(default)]
    dictation: Option<FileDictation>,
    #[serde(flatten)]
    settings: SetNodeSettings,
}
impl FileSettings {
    pub(super) fn into_settings(mut self) -> SetNodeSettings {
        self.settings.dictation = self.dictation.map(FileDictation::into_overrides);
        self.settings
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct FileDictation {
    switch: Option<DictationSwitch>,
    provider: Option<DictationProvider>,
    language: Option<Language>,
    local_model: Option<LocalModelId>,
    endpoint_model: Option<EndpointModel>,
    endpoint_base: Option<EndpointBase>,
    key: Option<SecretReference>,
}
impl FileDictation {
    fn into_overrides(self) -> SetDictation {
        // Intentionally exhaustive: adding a known override must update this
        // projection, rather than silently defaulting it away on the next write.
        SetDictation {
            switch: self.switch.map(DictationChange::Set),
            provider: self.provider.map(DictationChange::Set),
            language: self.language.map(DictationChange::Set),
            local_model: self.local_model.map(DictationChange::Set),
            endpoint_model: self.endpoint_model.map(DictationChange::Set),
            endpoint_base: self.endpoint_base.map(DictationChange::Set),
            key: self.key.map(DictationChange::Set),
        }
    }
}
