use vessel_proto::ts::{Declarations, NameClash};

use super::{ClientFrame, ServerFrame, Status};

/// Where the web UI keeps them, from the workspace's root.
pub const BRIDGE_FILE: &str = "web/src/wire/bridge.ts";

/// The bridge's module: its frames, importing the protocol's types
/// from `./proto`.
///
/// # Errors
/// Two types share a TypeScript name.
pub fn module() -> Result<String, NameClash> {
    let proto = Declarations::wire()?;
    let mut bridge = Declarations::new();
    bridge.add::<ClientFrame>()?;
    bridge.add::<ServerFrame>()?;
    bridge.add::<Status>()?;
    Ok(bridge.render(Some((&proto, "./proto"))))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use vessel_proto::ts::{ALIASES, check_file, check_strict};

    use super::*;

    #[test]
    fn every_primitive_alias_is_classified_and_every_entry_still_declared() {
        let proto = Declarations::wire().unwrap();
        let mut bridge = Declarations::new();
        bridge.add::<ClientFrame>().unwrap();
        bridge.add::<ServerFrame>().unwrap();
        bridge.add::<Status>().unwrap();
        let unclassified: Vec<&str> = bridge.unclassified().collect();
        assert!(
            unclassified.is_empty(),
            "classify these in vessel-proto's ts/aliases.rs: {unclassified:?}"
        );
        let declared: Vec<&str> = proto.aliases().chain(bridge.aliases()).collect();
        let stale: Vec<&str> = ALIASES
            .iter()
            .map(|(name, _alias)| *name)
            .filter(|name| !declared.contains(name))
            .collect();
        assert!(
            stale.is_empty(),
            "no longer declared, remove from ts/aliases.rs: {stale:?}"
        );
    }

    #[test]
    fn typescript_declarations_are_current() {
        let generated = module().unwrap();
        check_strict(&generated).unwrap();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(BRIDGE_FILE);
        check_file(&path, &generated).unwrap();
    }
}
