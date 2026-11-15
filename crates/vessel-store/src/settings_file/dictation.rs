//! Lenient file projection beside the strict wire patch, sharing its value types.
//!
//! Unknown fields cannot invalidate all saved settings. The existing settings
//! owner detects what was ignored and copies that later file before replacing it.

use serde::Deserialize;
use vessel_proto::dictation::{
    DictationChange, DictationProvider, DictationSwitch, EndpointBase, EndpointModel, Language,
    LocalModelId, SecretReference, SetDictation,
};
use vessel_proto::{
    parking::ParkAfter,
    settings::{SetAgentDrivers, SetNodeSettings},
};

// Flatten buffers foreign values before decoding the baseline settings and
// loses their serde_path_to_error location. Direct fields preserve that single
// parse; exhaustive construction makes every future setting update this view.
#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct FileSettings {
    park_after: Option<ParkAfter>,
    drivers: Option<SetAgentDrivers>,
    dictation: Option<FileDictation>,
}
impl FileSettings {
    pub(super) fn into_settings(self) -> SetNodeSettings {
        SetNodeSettings {
            park_after: self.park_after,
            drivers: self.drivers,
            // An ignored future-only block contributes no known overrides.
            // Keep its original bytes through the existing later-file backup,
            // rather than introducing a spurious empty block on the next write.
            dictation: self
                .dictation
                .map(FileDictation::into_overrides)
                .filter(|overrides| overrides != &SetDictation::default()),
        }
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
