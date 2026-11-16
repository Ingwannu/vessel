use super::*;
use crate::{devices::Tickets, voice::settings::Page};
use vessel_proto::{NodeId, dictation::DictationSettings};

fn values() -> Snapshot {
    Snapshot {
        dictation: DictationSettings::default(),
        conversation: vessel_proto::voice::VoiceSettings::default(),
    }
}
fn opened(site: Site, values: Snapshot) -> Draft {
    let node = NodeId::random();
    let mut tickets = Tickets::default();
    let read = tickets.next_ticket().unwrap();
    let mut page = Page::reading(node, read);
    assert_eq!(
        page.answered(node, read, Ok(values)),
        crate::voice::Applied::Current
    );
    let editor = page
        .open_editor(page.edit_action(site).unwrap())
        .unwrap()
        .unwrap();
    page.editor_draft(editor).unwrap()
}

#[test]
fn unchanged_and_reverted_fields_produce_no_write_or_provider_switch() {
    for site in [Site::Dictation, Site::Recognition, Site::Synthesis] {
        let mut draft = opened(site, values());
        assert!(draft.change().is_none());
        draft.edit(Input::Provider(Provider::Endpoint)).unwrap();
        assert!(draft.change().is_some());
        draft.edit(Input::Provider(Provider::Local)).unwrap();
        assert!(draft.change().is_none());
        assert_eq!(draft.key_presence(), KeyPresence::Absent);
    }
}

#[test]
fn recognition_provider_switch_preserves_language_and_never_replays_dormant_endpoint_data() {
    let mut original = values();
    original.conversation.stt.language = "ko".parse().unwrap();
    let mut draft = opened(Site::Recognition, original);
    draft.edit(Input::Provider(Provider::Endpoint)).unwrap();
    draft
        .edit(Input::RecognitionModel(Some("custom-asr".parse().unwrap())))
        .unwrap();
    draft.edit(Input::Provider(Provider::Local)).unwrap();
    assert!(draft.change().is_none());
    draft.edit(Input::Provider(Provider::Endpoint)).unwrap();
    let Change::Conversation(change) = draft.change().unwrap() else {
        panic!("recognition only");
    };
    assert!(change.switch.is_none() && change.tts.is_none());
    let effective = change
        .over(SetVoice::default())
        .unwrap()
        .resolve(KeyPresence::Absent, KeyPresence::Absent);
    assert_eq!(effective.stt.language.as_str(), "ko");
    assert_eq!(effective.stt.endpoint_model.as_str(), "custom-asr");
    assert_eq!(effective.tts, values().conversation.tts);
}

#[test]
fn clearing_endpoint_fields_is_reset_and_each_role_routes_only_its_own_site() {
    for site in [Site::Dictation, Site::Recognition, Site::Synthesis] {
        let mut original = values();
        original.dictation.provider = DictationProvider::Endpoint;
        original.conversation.stt.provider = DictationProvider::Endpoint;
        original.conversation.tts.provider = DictationProvider::Endpoint;
        let mut draft = opened(site, original);
        draft.edit(Input::Base(None)).unwrap();
        match draft.change().unwrap() {
            Change::Dictation(patch) => {
                assert_eq!(site, Site::Dictation);
                assert_eq!(patch.endpoint_base, Some(DictationChange::Reset));
                assert!(patch.switch.is_none() && patch.key.is_none());
                assert!(patch.over(SetDictation::default()).is_ok());
            }
            Change::Conversation(patch) => {
                assert!(patch.switch.is_none());
                match site {
                    Site::Recognition => {
                        assert!(patch.tts.is_none());
                        let Some(DictationChange::Set(stt)) = &patch.stt else {
                            panic!("STT only");
                        };
                        assert_eq!(stt.endpoint_base, Some(DictationChange::Reset));
                        assert!(stt.key.is_none());
                    }
                    Site::Synthesis => {
                        assert!(patch.stt.is_none());
                        let Some(DictationChange::Set(tts)) = &patch.tts else {
                            panic!("TTS only");
                        };
                        assert_eq!(tts.endpoint_base, Some(DictationChange::Reset));
                        assert!(tts.key.is_none());
                    }
                    Site::Dictation => panic!("not conversation"),
                }
                assert!(patch.over(SetVoice::default()).is_ok());
            }
        }
    }
}

#[test]
fn wrong_role_and_future_models_are_refused_without_changing_the_draft() {
    let mut stt = opened(Site::Dictation, values());
    assert_eq!(stt.edit(Input::Voice(None)), Err(Problem::WrongRole));
    assert_eq!(
        stt.edit(Input::LocalRecognition("future-asr".parse().unwrap())),
        Err(Problem::UnknownModel)
    );
    assert!(stt.change().is_none());
    let mut tts = opened(Site::Synthesis, values());
    assert_eq!(tts.edit(Input::Language(None)), Err(Problem::WrongRole));
    assert_eq!(
        tts.edit(Input::LocalSynthesis("future-tts".parse().unwrap())),
        Err(Problem::UnknownModel)
    );
    assert!(tts.change().is_none());
}

#[test]
fn key_presence_never_becomes_a_key_write_and_debug_omits_attempted_metadata() {
    let mut original = values();
    original.conversation.tts.provider = DictationProvider::Endpoint;
    original.conversation.tts.key = KeyPresence::Present;
    let mut draft = opened(Site::Synthesis, original);
    assert_eq!(draft.key_presence(), KeyPresence::Present);
    let input = Input::SynthesisModel(Some("private-canary-model".parse().unwrap()));
    assert!(!format!("{input:?}").contains("private-canary"));
    draft.edit(input).unwrap();
    assert!(!format!("{draft:?}").contains("private-canary"));
    let Change::Conversation(change) = draft.change().unwrap() else {
        panic!("TTS only");
    };
    let Some(DictationChange::Set(tts)) = change.tts else {
        panic!("TTS only");
    };
    assert!(tts.key.is_none());
    draft.edit(Input::Provider(Provider::Local)).unwrap();
    assert_eq!(draft.key_presence(), KeyPresence::Absent);
}

#[test]
fn candidate_diagnostics_redact_metadata_on_all_three_site_routes() {
    for site in [Site::Dictation, Site::Recognition, Site::Synthesis] {
        let mut draft = opened(site, values());
        draft.edit(Input::Provider(Provider::Endpoint)).unwrap();
        let input = match site {
            Site::Dictation | Site::Recognition => {
                Input::RecognitionModel(Some("private-canary-model".parse().unwrap()))
            }
            Site::Synthesis => Input::SynthesisModel(Some("private-canary-model".parse().unwrap())),
        };
        draft.edit(input).unwrap();
        let candidate = draft.change().unwrap();
        assert!(!format!("{candidate:?}").contains("private-canary"));
    }
}
