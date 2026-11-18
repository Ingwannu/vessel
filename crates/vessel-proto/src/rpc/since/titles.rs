//! TRACKER 801: a call asking a title longer than an older node decodes
//! needs `LONG_TITLES_PROTOCOL`, wherever the title sits and through every
//! envelope; one within the old limit keeps its method's requirement.

use super::*;
use crate::rpc::samples;
use crate::{Forward, Method, NodeId, Request as _};

fn long() -> SessionTitle {
    "t".repeat(150).parse().unwrap()
}

fn short() -> SessionTitle {
    "t".repeat(SessionTitle::LEGACY_MAX_CHARS).parse().unwrap()
}

/// `method`'s sample with its title (or its spec's) set to `title`.
fn with_title(method: Method, title: &SessionTitle) -> Call {
    let mut call = samples::call(method).unwrap();
    if let Call::RenameSession(request) = &mut call {
        request.title = title.clone();
    }
    if let Call::CreateTerminal(request) = &mut call {
        request.title = Some(title.clone());
    }
    if let Call::RenameTerminal(request) = &mut call {
        request.title = title.clone();
    }
    if let Call::RenameWorkspace(request) = &mut call {
        request.title = Some(title.clone());
    }
    if let Call::CreateSession(request) = &mut call {
        request.spec.title = Some(title.clone());
    }
    call
}

#[test]
fn a_long_title_needs_the_long_titles_protocol_and_a_short_one_does_not() {
    let long_titles = crate::LONG_TITLES_PROTOCOL.unwrap();
    for method in [
        Method::CreateSession,
        Method::RenameSession,
        Method::CreateTerminal,
        Method::RenameTerminal,
        Method::RenameWorkspace,
    ] {
        // The same call with a 120-character title: what every peer decodes.
        let before = with_title(method, &short()).since();
        assert!(
            before.is_some_and(|since| since < long_titles),
            "{method:?}: 120 characters needs no new protocol"
        );
        let asked = with_title(method, &long());
        assert_eq!(
            asked.since(),
            before.map(|since| since.max(long_titles)),
            "{method:?}"
        );
        let forwarded = Forward {
            node: NodeId::random(),
            call: Box::new(asked),
        }
        .into_call();
        assert!(
            forwarded.since() >= Some(long_titles),
            "{method:?} through an envelope"
        );
    }
}
