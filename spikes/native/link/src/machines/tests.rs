//! Browser refusal must settle the same pending operation as a native reply.
use super::{Answer, Ask};
use vessel_app_view::machine_book::{Edit, Failure, Name};
#[test]
fn browser_refusal_preserves_every_request_shape() {
    for ask in [Ask::Read, Ask::Retry] {
        assert_eq!(
            ask.refused(Failure::Unavailable),
            Answer::Read(Err(Failure::Unavailable))
        );
    }
    let name = Name::parse("laptop").unwrap();
    for edit in [
        Edit::Forget(name.clone()),
        Edit::MakeDefault(name.clone()),
        Edit::Rename {
            from: name,
            to: Name::parse("renamed").unwrap(),
        },
    ] {
        assert_eq!(
            Ask::Edit(edit.clone()).refused(Failure::Unavailable),
            Answer::Edited {
                edit,
                result: Err(Failure::Unavailable)
            }
        );
    }
}
