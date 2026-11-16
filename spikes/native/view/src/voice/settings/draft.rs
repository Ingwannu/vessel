//! Non-secret provider form decisions over the canonical bounded metadata types.
//!
//! Raw text belongs to the shared fields; parsing uses the canonical types before
//! this boundary. The draft never receives an API key or saved reference name.
//! It produces a candidate patch, not permission to send a node request.

use super::{Change, Site, Snapshot};
use vessel_proto::dictation::{
    DictationChange, DictationProvider, EndpointBase, EndpointModel, KeyPresence, Language,
    LocalModelId, SetDictation,
};
use vessel_proto::voice::{
    LocalTtsModelId, SetVoice, SetVoiceStt, SetVoiceTts, TtsModel, TtsVoice, VoiceSttSettings,
    VoiceTtsSettings,
};

/// Only the two providers this build understands may be newly selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Selection metadata only; local inference remains unavailable.
    Local,
    /// An explicitly configured endpoint, never an implicit fallback.
    Endpoint,
}
impl Provider {
    const fn wire(self) -> DictationProvider {
        match self {
            Self::Local => DictationProvider::Local,
            Self::Endpoint => DictationProvider::Endpoint,
        }
    }
    const fn from_wire(value: DictationProvider) -> Option<Self> {
        match value {
            DictationProvider::Local => Some(Self::Local),
            DictationProvider::Endpoint => Some(Self::Endpoint),
            DictationProvider::Unknown => None,
        }
    }
}

/// A validated edit; None means the person explicitly cleared that field.
/// The secure key input is intentionally outside this non-secret model.
#[derive(Clone)]
pub enum Input {
    /// Change the provider without losing unsaved edits when switching back.
    Provider(Provider),
    /// Recognition only; None resets to the canonical default language.
    Language(Option<Language>),
    /// A known recognition catalogue member, with no asset action.
    LocalRecognition(LocalModelId),
    /// A known synthesis catalogue member, with no asset action.
    LocalSynthesis(LocalTtsModelId),
    /// A bounded custom recognition endpoint model.
    RecognitionModel(Option<EndpointModel>),
    /// A bounded custom synthesis endpoint model.
    SynthesisModel(Option<TtsModel>),
    /// Synthesis speaker identity, distinct from its model.
    Voice(Option<TtsVoice>),
    /// Both endpoint roles use the same validated credential-free URL type.
    Base(Option<EndpointBase>),
}
impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeechFormInput").finish_non_exhaustive()
    }
}

/// Form failures contain only rules, never attempted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// The field belongs to another speech role.
    #[error("this field belongs to another speech role")]
    WrongRole,
    /// Future model metadata may be displayed, never newly selected.
    #[error("this local model is not known by this build")]
    UnknownModel,
}

/// One opening's metadata draft; the Page/editor token owns its lifetime.
pub struct Draft {
    site: Site,
    provider: Provider,
    fields: Fields,
}
enum Fields {
    Recognition(VoiceSttSettings, SetVoiceStt),
    Synthesis(VoiceTtsSettings, SetVoiceTts),
}
impl std::fmt::Debug for Draft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeechFormDraft")
            .field("site", &self.site)
            .finish_non_exhaustive()
    }
}
impl Draft {
    pub(super) fn new(site: Site, snapshot: &Snapshot) -> Option<Self> {
        let fields = match site {
            Site::Dictation => {
                let value = &snapshot.dictation;
                Fields::Recognition(
                    VoiceSttSettings {
                        provider: value.provider,
                        language: value.language.clone(),
                        local_model: value.local_model.clone(),
                        endpoint_model: value.endpoint_model.clone(),
                        endpoint_base: value.endpoint_base.clone(),
                        key: value.key,
                    },
                    SetVoiceStt::default(),
                )
            }
            Site::Recognition => {
                Fields::Recognition(snapshot.conversation.stt.clone(), SetVoiceStt::default())
            }
            Site::Synthesis => {
                Fields::Synthesis(snapshot.conversation.tts.clone(), SetVoiceTts::default())
            }
        };
        let original = match &fields {
            Fields::Recognition(value, _) => value.provider,
            Fields::Synthesis(value, _) => value.provider,
        };
        Some(Self {
            site,
            provider: Provider::from_wire(original)?,
            fields,
        })
    }

    /// Current editable provider; this never claims local engine readiness.
    #[must_use]
    pub const fn provider(&self) -> Provider {
        self.provider
    }

    /// Saved presence applies only to the original endpoint provider.
    /// Changing to an endpoint never fabricates a key from another provider.
    #[must_use]
    pub fn key_presence(&self) -> KeyPresence {
        let (provider, key) = match &self.fields {
            Fields::Recognition(value, _) => (value.provider, value.key),
            Fields::Synthesis(value, _) => (value.provider, value.key),
        };
        if self.provider == Provider::Endpoint && provider == DictationProvider::Endpoint {
            key
        } else {
            KeyPresence::Absent
        }
    }

    /// Retain valid edits within this form; inactive fields are filtered at build.
    ///
    /// # Errors
    /// Wrong-role fields and unknown newly selected local models are refused.
    pub fn edit(&mut self, input: Input) -> Result<(), Problem> {
        if let Input::Provider(provider) = input {
            self.provider = provider;
            return Ok(());
        }
        match &mut self.fields {
            Fields::Recognition(initial, patch) => recognition_edit(initial, patch, input),
            Fields::Synthesis(initial, patch) => synthesis_edit(initial, patch, input),
        }
    }

    /// A candidate touches exactly this site and never a feature switch or key.
    /// A caller must still recheck the Page's current editor and write admission.
    #[must_use]
    pub fn change(&self) -> Option<Change> {
        match &self.fields {
            Fields::Recognition(initial, patch) => {
                let patch = recognition_patch(self.provider, initial, patch)?;
                Some(match self.site {
                    Site::Dictation => Change::Dictation(SetDictation {
                        provider: patch.provider,
                        language: patch.language,
                        local_model: patch.local_model,
                        endpoint_model: patch.endpoint_model,
                        endpoint_base: patch.endpoint_base,
                        ..SetDictation::default()
                    }),
                    Site::Recognition => Change::Conversation(SetVoice {
                        stt: Some(DictationChange::Set(patch)),
                        ..SetVoice::default()
                    }),
                    Site::Synthesis => return None,
                })
            }
            Fields::Synthesis(initial, patch) => {
                let patch = synthesis_patch(self.provider, initial, patch)?;
                Some(Change::Conversation(SetVoice {
                    tts: Some(DictationChange::Set(patch)),
                    ..SetVoice::default()
                }))
            }
        }
    }
}

fn changed<T: PartialEq>(value: Option<T>, initial: &T) -> Option<DictationChange<T>> {
    match value {
        None => Some(DictationChange::Reset),
        Some(value) if value == *initial => None,
        Some(value) => Some(DictationChange::Set(value)),
    }
}

mod patch;
use patch::{recognition_edit, recognition_patch, synthesis_edit, synthesis_patch};

#[cfg(test)]
mod tests;
