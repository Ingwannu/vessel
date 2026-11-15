//! Walk Fluent's maintained AST to discover variables, never its source text.
//! References are expanded by the catalog validator before API generation.

use std::collections::BTreeSet;

use fluent_syntax::ast::{
    CallArguments, Expression, InlineExpression, Pattern, PatternElement, VariantKey,
};

use super::{ContractError, Key, Kind, Name, Reference, Shape, types::CallArity};

const MAX_FORMAT_DIGITS: usize = 9;

/// Formatting patterns cannot call DATETIME because that would re-enter them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternRole {
    Message,
    Formatting,
}

pub(crate) fn scan(
    pattern: &Pattern<&str>,
    shape: &mut Shape,
    role: PatternRole,
) -> Result<(), ContractError> {
    for element in &pattern.elements {
        match element {
            PatternElement::TextElement { .. } => {}
            PatternElement::Placeable { expression } => expression_shape(expression, shape, role)?,
        }
    }
    Ok(())
}

fn expression_shape(
    expression: &Expression<&str>,
    shape: &mut Shape,
    role: PatternRole,
) -> Result<(), ContractError> {
    match expression {
        Expression::Inline(inline) => inline_shape(inline, shape, role)?,
        Expression::Select { selector, variants } => {
            inline_shape(selector, shape, role)?;
            if let InlineExpression::VariableReference { id } = selector {
                let choices: BTreeSet<_> = variants
                    .iter()
                    .filter_map(|v| match &v.key {
                        VariantKey::Identifier { name } => Some(Name::parse(name)),
                        VariantKey::NumberLiteral { .. } => None,
                    })
                    .collect::<Result<_, _>>()?;
                let numeric = variants
                    .iter()
                    .any(|v| matches!(v.key, VariantKey::NumberLiteral { .. }))
                    || choices.iter().all(|c| {
                        matches!(
                            c.as_str(),
                            "zero" | "one" | "two" | "few" | "many" | "other"
                        )
                    });
                shape.merge(
                    Name::parse(id.name)?,
                    if numeric {
                        Kind::Number
                    } else {
                        Kind::Choice(choices)
                    },
                );
            }
            for variant in variants {
                if let VariantKey::NumberLiteral { value } = variant.key {
                    numeric_literal(value)?;
                }
                scan(&variant.value, shape, role)?;
            }
        }
    }
    Ok(())
}

fn arguments(
    args: &CallArguments<&str>,
    shape: &mut Shape,
    role: PatternRole,
) -> Result<(), ContractError> {
    for argument in &args.positional {
        inline_shape(argument, shape, role)?;
    }
    for argument in &args.named {
        inline_shape(&argument.value, shape, role)?;
    }
    Ok(())
}

fn inline_shape(
    inline: &InlineExpression<&str>,
    shape: &mut Shape,
    role: PatternRole,
) -> Result<(), ContractError> {
    match inline {
        InlineExpression::VariableReference { id } => {
            shape.merge(Name::parse(id.name)?, Kind::Text)
        }
        InlineExpression::MessageReference { id, attribute } => {
            if attribute.is_some() {
                return Err(ContractError::MessageAttribute);
            }
            shape.references.push(Reference {
                key: Key::Message(Name::parse(id.name)?),
                bound: BTreeSet::new(),
            });
        }
        InlineExpression::TermReference {
            id,
            attribute,
            arguments: args,
        } => {
            if attribute.is_some() {
                return Err(ContractError::TermAttribute);
            }
            term_shape(id.name, args.as_ref(), shape, role)?;
        }
        InlineExpression::FunctionReference {
            id,
            arguments: args,
        } => {
            function_shape(id.name, args, shape, role)?;
        }
        InlineExpression::Placeable { expression } => expression_shape(expression, shape, role)?,
        InlineExpression::StringLiteral { .. } => {}
        InlineExpression::NumberLiteral { value } => numeric_literal(value)?,
    }
    Ok(())
}

fn term_shape(
    name: &str,
    args: Option<&CallArguments<&str>>,
    shape: &mut Shape,
    role: PatternRole,
) -> Result<(), ContractError> {
    if let Some(args) = args {
        arguments(args, shape, role)?;
    }
    shape.references.push(Reference {
        key: Key::Term(Name::parse(name)?),
        bound: args
            .map(|a| {
                a.named
                    .iter()
                    .map(|n| Name::parse(n.name.name))
                    .collect::<Result<_, _>>()
            })
            .transpose()?
            .unwrap_or_default(),
    });
    Ok(())
}

fn function_shape(
    name: &str,
    args: &CallArguments<&str>,
    shape: &mut Shape,
    role: PatternRole,
) -> Result<(), ContractError> {
    check_call(name, args)?;
    if name == "DATETIME" && role == PatternRole::Formatting {
        return Err(ContractError::RecursiveDate);
    }
    match name {
        "NUMBER" => {
            if let Some(InlineExpression::VariableReference { id }) = args.positional.first() {
                shape.merge(Name::parse(id.name)?, Kind::Number);
            }
        }
        "DATETIME" => {
            if let Some(InlineExpression::VariableReference { id }) = args.positional.first() {
                shape.merge(Name::parse(id.name)?, Kind::Date);
            }
        }
        "PARTICLE" => {}
        name => return Err(ContractError::UnknownFunction(Name::parse(name)?)),
    }
    arguments(args, shape, role)?;
    Ok(())
}

fn check_call(name: &str, args: &CallArguments<&str>) -> Result<(), ContractError> {
    let arity = if name == "PARTICLE" {
        CallArity::Particle
    } else {
        CallArity::Unary
    };
    if args.positional.len() != arity.count() || (name != "NUMBER" && !args.named.is_empty()) {
        return Err(ContractError::CallArguments {
            function: Name::parse(name)?,
            arity,
        });
    }
    if name == "DATETIME"
        && !matches!(
            args.positional.first(),
            Some(InlineExpression::VariableReference { .. })
        )
    {
        return Err(ContractError::DateArgument);
    }
    if name == "NUMBER" {
        check_number_options(args)?;
    }
    if name == "PARTICLE" {
        let valid = matches!(
            args.positional.last(),
            Some(InlineExpression::StringLiteral {
                value: "object" | "subject" | "topic" | "with" | "direction"
            })
        );
        if !valid {
            return Err(ContractError::ParticleArgument);
        }
    }
    Ok(())
}

fn check_number_options(args: &CallArguments<&str>) -> Result<(), ContractError> {
    let fraction = |name| {
        args.named
            .iter()
            .find(|a| a.name.name == name)
            .and_then(|a| {
                if let InlineExpression::NumberLiteral { value } = a.value {
                    value.parse::<usize>().ok()
                } else {
                    None
                }
            })
    };
    if let (Some(minimum), Some(maximum)) = (
        fraction("minimumFractionDigits"),
        fraction("maximumFractionDigits"),
    ) && minimum > maximum
    {
        return Err(ContractError::FractionOrder);
    }
    for option in &args.named {
        let valid = match (option.name.name, &option.value) {
            (
                "useGrouping",
                InlineExpression::StringLiteral {
                    value: "true" | "false",
                },
            )
            | (
                "type",
                InlineExpression::StringLiteral {
                    value: "cardinal" | "ordinal",
                },
            ) => true,
            (
                "minimumIntegerDigits" | "minimumFractionDigits" | "maximumFractionDigits",
                InlineExpression::NumberLiteral { value },
            ) => value.parse::<usize>().is_ok_and(|n| n <= MAX_FORMAT_DIGITS),
            _ => false,
        };
        if !valid {
            return Err(ContractError::NumberOption);
        }
    }
    Ok(())
}

// Fluent's numeric plural operands are finite bounded values. Reject catalog
// literals at the same boundary as typed decimal callers, before a malformed
// translation can reach the upstream operand conversion.
fn numeric_literal(value: &str) -> Result<(), ContractError> {
    let number = value.parse::<f64>().map_err(ContractError::NumberParse)?;
    if !number.is_finite()
        || number.abs() > f64::from(u32::MAX)
        || value
            .split_once('.')
            .is_some_and(|(_, fraction)| fraction.len() > MAX_FORMAT_DIGITS)
    {
        return Err(ContractError::NumberRange);
    }
    Ok(())
}
