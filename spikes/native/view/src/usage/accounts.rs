//! Account identities are configured bindings, never discovered credential paths.
use vessel_proto::limits::{InvalidLimitText, LimitName};
use vessel_proto::{HarnessRef, ProfileInfo, ProfileName};
/// One inventory is refused as a whole instead of silently dropping configured accounts.
pub const MAX_CONFIGURED_USAGE_ACCOUNTS: usize = 256;
/// The existing profile API has a broader payload; usage validates its own presentation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidAccountInventory {
    /// No local/vendor reads may start from an oversized profile list.
    #[error("too many configured usage accounts")]
    TooMany,
    /// Invalid metadata is not shortened into a different label.
    #[error("invalid configured usage account label")]
    Label(#[from] InvalidLimitText),
}

/// One configured account, with only presentation metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    harness: HarnessRef,
    profile: ProfileName,
    label: Option<LimitName>,
}
impl Account {
    /// Existing harness identity is preserved through calls and selection changes.
    #[must_use]
    pub const fn harness(&self) -> &HarnessRef {
        &self.harness
    }
    /// The node lists implicit defaults too; no second default is invented.
    #[must_use]
    pub const fn profile(&self) -> &ProfileName {
        &self.profile
    }
    /// A human label is presentation only, never an account identity.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_ref().map(LimitName::as_str)
    }
}

/// profile.list includes defaults, followed by named profiles. Preserve that order.
/// Different profiles remain independent even when their labels happen to match.
/// # Errors
/// Excess profiles or invalid metadata refuse the complete inventory before any usage calls.
pub fn configured_accounts(
    profiles: &[ProfileInfo],
) -> Result<Vec<Account>, InvalidAccountInventory> {
    if profiles.len() > MAX_CONFIGURED_USAGE_ACCOUNTS {
        return Err(InvalidAccountInventory::TooMany);
    }
    let mut result = Vec::new();
    for profile in profiles {
        if !result
            .iter()
            .any(|a: &Account| a.harness == profile.harness && a.profile == profile.name)
        {
            result.push(Account {
                harness: profile.harness.clone(),
                profile: profile.name.clone(),
                label: profile.label.as_deref().map(str::parse).transpose()?,
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(name: &str, harness: HarnessRef) -> ProfileInfo {
        ProfileInfo {
            name: name.parse().unwrap(),
            label: Some("Same display label".into()),
            harness,
            sets: Vec::new(),
            model: None,
        }
    }
    #[test]
    fn labels_do_not_merge_distinct_profiles_and_defaults_are_not_duplicated() {
        let claude: HarnessRef = "claude".parse().unwrap();
        let codex: HarnessRef = "codex".parse().unwrap();
        let default = profile("claude", claude.clone());
        let profiles = [
            default.clone(),
            profile("first", claude.clone()),
            profile("second", claude),
            profile("third", codex),
            default,
        ];
        let accounts = configured_accounts(&profiles).unwrap();
        let names: Vec<_> = accounts
            .iter()
            .map(Account::profile)
            .map(ToString::to_string)
            .collect();
        assert_eq!(names, ["claude", "first", "second", "third"]);
        assert!(
            accounts
                .iter()
                .all(|a| a.label() == Some("Same display label"))
        );
    }
    #[test]
    fn an_empty_profile_list_invents_no_accounts() {
        assert!(configured_accounts(&[]).unwrap().is_empty());
    }
    #[test]
    fn excess_or_invalid_account_metadata_is_refused_without_omission() {
        let mut item = profile("codex", "codex".parse().unwrap());
        assert_eq!(
            configured_accounts(&vec![item.clone(); MAX_CONFIGURED_USAGE_ACCOUNTS + 1]),
            Err(InvalidAccountInventory::TooMany)
        );
        item.label = Some("control\nlabel".to_owned());
        assert!(matches!(
            configured_accounts(&[item]),
            Err(InvalidAccountInventory::Label(_))
        ));
    }
}
