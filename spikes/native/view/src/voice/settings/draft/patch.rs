//! Candidate construction filters inactive fields; canonical overlay owns validation.
use super::{
    DictationChange, Input, Problem, Provider, SetVoiceStt, SetVoiceTts, VoiceSttSettings,
    VoiceTtsSettings, changed,
};

pub(super) fn recognition_edit(
    initial: &VoiceSttSettings,
    patch: &mut SetVoiceStt,
    input: Input,
) -> Result<(), Problem> {
    match input {
        Input::Language(value) => patch.language = changed(value, &initial.language),
        Input::LocalRecognition(value) => {
            if !value.known() {
                return Err(Problem::UnknownModel);
            }
            patch.local_model = changed(Some(value), &initial.local_model);
        }
        Input::RecognitionModel(value) => {
            patch.endpoint_model = changed(value, &initial.endpoint_model);
        }
        Input::Base(value) => patch.endpoint_base = changed(value, &initial.endpoint_base),
        Input::Provider(_)
        | Input::LocalSynthesis(_)
        | Input::SynthesisModel(_)
        | Input::Voice(_) => return Err(Problem::WrongRole),
    }
    Ok(())
}

pub(super) fn synthesis_edit(
    initial: &VoiceTtsSettings,
    patch: &mut SetVoiceTts,
    input: Input,
) -> Result<(), Problem> {
    match input {
        Input::LocalSynthesis(value) => {
            if !value.known() {
                return Err(Problem::UnknownModel);
            }
            patch.local_model = changed(Some(value), &initial.local_model);
        }
        Input::SynthesisModel(value) => {
            patch.endpoint_model = changed(value, &initial.endpoint_model);
        }
        Input::Voice(value) => patch.voice = changed(value, &initial.voice),
        Input::Base(value) => patch.endpoint_base = changed(value, &initial.endpoint_base),
        Input::Provider(_)
        | Input::Language(_)
        | Input::LocalRecognition(_)
        | Input::RecognitionModel(_) => return Err(Problem::WrongRole),
    }
    Ok(())
}

pub(super) fn recognition_patch(
    provider: Provider,
    initial: &VoiceSttSettings,
    edits: &SetVoiceStt,
) -> Option<SetVoiceStt> {
    let switching = provider.wire() != initial.provider;
    let mut patch = SetVoiceStt {
        language: edits.language.clone(),
        ..SetVoiceStt::default()
    };
    match provider {
        Provider::Local => patch.local_model.clone_from(&edits.local_model),
        Provider::Endpoint => {
            patch.endpoint_model.clone_from(&edits.endpoint_model);
            patch.endpoint_base.clone_from(&edits.endpoint_base);
        }
    }
    if !switching && patch == SetVoiceStt::default() {
        return None;
    }
    patch.provider = Some(DictationChange::Set(provider.wire()));
    // The canonical provider change clears old site options. Preserve language
    // explicitly, as the reference form does, without reviving old endpoint data.
    if switching && patch.language.is_none() {
        patch.language = Some(DictationChange::Set(initial.language.clone()));
    }
    Some(patch)
}

pub(super) fn synthesis_patch(
    provider: Provider,
    initial: &VoiceTtsSettings,
    edits: &SetVoiceTts,
) -> Option<SetVoiceTts> {
    let mut patch = SetVoiceTts::default();
    match provider {
        Provider::Local => patch.local_model.clone_from(&edits.local_model),
        Provider::Endpoint => {
            patch.endpoint_model.clone_from(&edits.endpoint_model);
            patch.endpoint_base.clone_from(&edits.endpoint_base);
            patch.voice.clone_from(&edits.voice);
        }
    }
    if provider.wire() == initial.provider && patch == SetVoiceTts::default() {
        return None;
    }
    patch.provider = Some(DictationChange::Set(provider.wire()));
    Some(patch)
}
