//! One generated-name grammar for admission and the book's write preflight.
//!
//! Reserving a whole human-readable prefix would reject ordinary destinations
//! such as desktop-pc. Canonical round-trip equality reserves only names the
//! controller can actually produce, without trimming or case folding aliases.

use crate::machine_book::Name;
use crate::machines::NameProblem;
use vessel_proto::NodeId;

const PREFIX: &str = "desktop-";
const UUID_DISPLAY_BYTES: usize = 36;

/// Deterministic across launches for the same authenticated managed home.
///
/// # Errors
/// The existing book-name grammar refuses the generated destination.
pub fn generated_name(node: NodeId) -> Result<Name, NameProblem> {
    Name::parse(&format!("{PREFIX}{node}"))
}

/// Only the exact lowercase, hyphenated UUID form belongs to local admission.
#[must_use]
pub fn is_reserved(name: &Name) -> bool {
    let Some(value) = name.as_str().strip_prefix(PREFIX) else {
        return false;
    };
    value.len() == UUID_DISPLAY_BYTES
        && value
            .parse::<NodeId>()
            .is_ok_and(|node| node.to_string() == value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_generated_canonical_form_is_reserved() {
        let node = NodeId::random();
        let generated = generated_name(node).unwrap();
        assert!(is_reserved(&generated));
        assert_eq!(generated_name(node).unwrap(), generated);
        for ordinary in [
            "desktop-pc".to_owned(),
            "desktop-home".to_owned(),
            format!("Desktop-{node}"),
            "desktop-ABCDEFAB-1234-5678-9ABC-ABCDEFABCDEF".to_owned(),
            format!("desktop-{}", node.to_string().replace('-', "")),
        ] {
            assert!(!is_reserved(&Name::parse(&ordinary).unwrap()));
        }
        for spaced in [
            format!(" {}", generated.as_str()),
            format!("{} ", generated.as_str()),
        ] {
            assert!(Name::parse(&spaced).is_err());
        }
    }
}
