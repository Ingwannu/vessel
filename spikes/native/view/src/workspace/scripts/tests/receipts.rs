//! Retained receipt projection cannot expose raw peer messages or config.
use super::*;

#[test]
fn retained_results_show_safe_typed_notices_without_peer_diagnostics_or_config() {
    use crate::localization::{ENGLISH, Translator};
    use crate::workspace::git::RequestKeys;
    use crate::workspace::problem::Problem;
    use vessel_proto::scripts::{ScriptRefusal, ScriptRunOutcome};
    let words = Translator::get(ENGLISH).unwrap();
    let target = target();
    let key = RequestKeys::default().issue().unwrap();
    let mut write = RetiredWrite::new(target.clone(), key, Action::Run("dev".parse().unwrap()));
    assert_eq!(
        words::receipt(words, &write),
        words.scripts().receipt_pending()
    );
    let private = "private diagnostic must never be rendered";
    assert!(write.accept(&target, key, Err(Problem::Gone(private.into()))));
    assert_eq!(
        words::receipt(words, &write),
        words.scripts().receipt_uncertain()
    );
    assert!(!words::receipt(words, &write).contains(private));

    let mut refused = RetiredWrite::new(target.clone(), key, Action::Run("dev".parse().unwrap()));
    assert!(refused.accept(
        &target,
        key,
        Ok(Some(ScriptRunOutcome::Refused {
            reason: ScriptRefusal::InvalidFile {
                message: private.into()
            },
        }))
    ));
    assert_eq!(
        words::receipt(words, &refused),
        words.scripts().invalid_file()
    );
    assert!(!words::receipt(words, &refused).contains(private));

    let mut approval = RetiredWrite::new(target.clone(), key, Action::Run("dev".parse().unwrap()));
    assert!(approval.accept(
        &target,
        key,
        Ok(Some(ScriptRunOutcome::NeedsApproval {
            commands: ProjectCommands::Invalid {
                message: private.into()
            },
        }))
    ));
    assert_eq!(words::receipt(words, &approval), words.scripts().approval());
    assert!(!words::receipt(words, &approval).contains(private));

    let mut started = RetiredWrite::new(target.clone(), key, Action::Run("dev".parse().unwrap()));
    assert!(started.accept(
        &target,
        key,
        Ok(Some(ScriptRunOutcome::Started {
            terminal: Box::new(run(&target)),
        }))
    ));
    assert_eq!(
        words::receipt(words, &started),
        words.scripts().receipt_started()
    );
    let mut acknowledged =
        RetiredWrite::new(target.clone(), key, Action::Stop(TerminalId::random()));
    assert!(acknowledged.accept(&target, key, Ok(None)));
    assert_eq!(
        words::receipt(words, &acknowledged),
        words.scripts().receipt_acknowledged()
    );
}
