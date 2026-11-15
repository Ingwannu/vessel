//! English is the argument contract; locale completeness is a caller policy.
use super::{Contract, ContractError, Key, Kind};

/// Core Korean is exact; partial installation translations may omit variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariableCoverage {
    Exact,
    Subset,
}

pub(crate) fn compare(
    key: &Key,
    expected: &Contract,
    actual: &Contract,
    coverage: VariableCoverage,
) -> Result<(), ContractError> {
    if actual
        .variables()
        .keys()
        .any(|name| !expected.variables().contains_key(name))
    {
        return Err(ContractError::ExtraVariables(key.clone()));
    }
    if coverage == VariableCoverage::Exact
        && !actual.variables().keys().eq(expected.variables().keys())
    {
        return Err(ContractError::MissingVariables(key.clone()));
    }
    for (name, kind) in expected.variables() {
        if *kind == Kind::Date && actual.raw_reads.contains(name) {
            return Err(ContractError::DateTextRead {
                key: key.clone(),
                name: name.clone(),
            });
        }
    }
    for (name, kind) in actual.variables() {
        match kind {
            Kind::Date if expected.variables().get(name) != Some(&Kind::Date) => {
                return Err(ContractError::DateRequired {
                    key: key.clone(),
                    name: name.clone(),
                });
            }
            Kind::Number if expected.variables().get(name) != Some(&Kind::Number) => {
                return Err(ContractError::NumberRequired {
                    key: key.clone(),
                    name: name.clone(),
                });
            }
            Kind::Text | Kind::Number | Kind::Date | Kind::Choice(_) => {}
        }
    }
    Ok(())
}
