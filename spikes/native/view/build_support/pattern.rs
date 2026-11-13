//! Walk Fluent's maintained AST to discover variables, never its source text.
//! References are expanded by the catalog validator before API generation.

use std::collections::BTreeSet;

use fluent_syntax::ast::{
    CallArguments, Expression, InlineExpression, Pattern, PatternElement, VariantKey,
};

use super::{BuildError, Kind, Reference, Shape, source};

pub(crate) fn scan(
    pattern: &Pattern<&str>,
    shape: &mut Shape,
    file: &str,
) -> Result<(), BuildError> {
    for element in &pattern.elements {
        match element {
            PatternElement::TextElement { .. } => {}
            PatternElement::Placeable { expression } => expression_shape(expression, shape, file)?,
        }
    }
    Ok(())
}

fn expression_shape(
    expression: &Expression<&str>,
    shape: &mut Shape,
    file: &str,
) -> Result<(), BuildError> {
    match expression {
        Expression::Inline(inline) => inline_shape(inline, shape, file)?,
        Expression::Select { selector, variants } => {
            inline_shape(selector, shape, file)?;
            if let InlineExpression::VariableReference { id } = selector {
                let choices: BTreeSet<_> = variants
                    .iter()
                    .filter_map(|v| match &v.key {
                        VariantKey::Identifier { name } => Some((*name).to_owned()),
                        VariantKey::NumberLiteral { .. } => None,
                    })
                    .collect();
                let numeric = variants
                    .iter()
                    .any(|v| matches!(v.key, VariantKey::NumberLiteral { .. }))
                    || choices.iter().all(|c| {
                        matches!(
                            c.as_str(),
                            "zero" | "one" | "two" | "few" | "many" | "other"
                        )
                    });
                merge(
                    shape,
                    id.name,
                    if numeric {
                        Kind::Number
                    } else {
                        Kind::Choice(choices)
                    },
                );
            }
            for variant in variants {
                if let VariantKey::NumberLiteral { value } = variant.key {
                    numeric_literal(value, file)?;
                }
                scan(&variant.value, shape, file)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn merge(shape: &mut Shape, name: &str, kind: Kind) {
    let current = shape.variables.entry(name.to_owned()).or_insert(Kind::Text);
    match (current, kind) {
        (Kind::Text | Kind::Number | Kind::Date | Kind::Choice(_), Kind::Text)
        | (Kind::Number, Kind::Number)
        | (Kind::Date, Kind::Date) => {}
        (Kind::Choice(left), Kind::Choice(right)) => left.extend(right),
        (slot @ Kind::Text, value @ (Kind::Number | Kind::Date | Kind::Choice(_))) => *slot = value,
        (Kind::Number, Kind::Date | Kind::Choice(_))
        | (Kind::Date, Kind::Number | Kind::Choice(_))
        | (Kind::Choice(_), Kind::Number | Kind::Date) => {
            let _new = shape.conflicts.insert(name.to_owned());
        }
    }
}

fn arguments(args: &CallArguments<&str>, shape: &mut Shape, file: &str) -> Result<(), BuildError> {
    for argument in &args.positional {
        inline_shape(argument, shape, file)?;
    }
    for argument in &args.named {
        inline_shape(&argument.value, shape, file)?;
    }
    Ok(())
}

fn inline_shape(
    inline: &InlineExpression<&str>,
    shape: &mut Shape,
    file: &str,
) -> Result<(), BuildError> {
    match inline {
        InlineExpression::VariableReference { id } => merge(shape, id.name, Kind::Text),
        InlineExpression::MessageReference { id, attribute } => {
            if attribute.is_some() {
                return Err(source(
                    file,
                    "message attributes are not supported; use a contextual message key",
                ));
            }
            shape.references.push(Reference {
                key: id.name.to_owned(),
                bound: BTreeSet::new(),
            });
        }
        InlineExpression::TermReference {
            id,
            attribute,
            arguments: args,
        } => {
            if attribute.is_some() {
                return Err(source(
                    file,
                    "term attributes are not supported; use a named term",
                ));
            }
            if let Some(args) = args {
                arguments(args, shape, file)?;
            }
            shape.references.push(Reference {
                key: format!("-{}", id.name),
                bound: args
                    .as_ref()
                    .map(|a| a.named.iter().map(|n| n.name.name.to_owned()).collect())
                    .unwrap_or_default(),
            });
        }
        InlineExpression::FunctionReference {
            id,
            arguments: args,
        } => {
            check_call(id.name, args, file)?;
            if id.name == "DATETIME" && file.ends_with("/formats.ftl") {
                return Err(source(
                    file,
                    "formatting patterns cannot call DATETIME recursively",
                ));
            }
            match id.name {
                "NUMBER" => {
                    if let Some(InlineExpression::VariableReference { id }) =
                        args.positional.first()
                    {
                        merge(shape, id.name, Kind::Number);
                    }
                }
                "DATETIME" => {
                    if let Some(InlineExpression::VariableReference { id }) =
                        args.positional.first()
                    {
                        merge(shape, id.name, Kind::Date);
                    }
                }
                "PARTICLE" => {}
                name => return Err(source(file, format!("unknown function {name}"))),
            }
            arguments(args, shape, file)?;
        }
        InlineExpression::Placeable { expression } => expression_shape(expression, shape, file)?,
        InlineExpression::StringLiteral { .. } => {}
        InlineExpression::NumberLiteral { value } => numeric_literal(value, file)?,
    }
    Ok(())
}

fn check_call(name: &str, args: &CallArguments<&str>, file: &str) -> Result<(), BuildError> {
    let arity = if name == "PARTICLE" { 2 } else { 1 };
    if args.positional.len() != arity || (name != "NUMBER" && !args.named.is_empty()) {
        return Err(source(
            file,
            format!("{name} needs {arity} positional arguments and no named options"),
        ));
    }
    if name == "DATETIME"
        && !matches!(
            args.positional.first(),
            Some(InlineExpression::VariableReference { .. })
        )
    {
        return Err(source(file, "DATETIME requires a typed date variable"));
    }
    if name == "NUMBER" {
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
            return Err(source(
                file,
                "minimumFractionDigits exceeds maximumFractionDigits",
            ));
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
                ) => value.parse::<usize>().is_ok_and(|n| n <= 9),
                _ => false,
            };
            if !valid {
                return Err(source(
                    file,
                    "unsupported NUMBER option; use grouping, integer/fraction digits (0–9), or plural type",
                ));
            }
        }
    }
    if name == "PARTICLE" {
        let valid = matches!(
            args.positional.get(1),
            Some(InlineExpression::StringLiteral {
                value: "object" | "subject" | "topic" | "with" | "direction"
            })
        );
        if !valid {
            return Err(source(
                file,
                "PARTICLE's second argument must name object, subject, topic, with, or direction",
            ));
        }
    }
    Ok(())
}

// Fluent's numeric plural operands are finite bounded values. Reject catalog
// literals at the same boundary as typed decimal callers, before a malformed
// translation can reach the upstream operand conversion.
fn numeric_literal(value: &str, file: &str) -> Result<(), BuildError> {
    let number = value
        .parse::<f64>()
        .map_err(|error| source(file, format!("invalid numeric literal: {error}")))?;
    if !number.is_finite()
        || number.abs() > f64::from(u32::MAX)
        || value
            .split_once('.')
            .is_some_and(|(_, fraction)| fraction.len() > 9)
    {
        return Err(source(
            file,
            "numeric literals require u32 magnitude and at most nine decimal places",
        ));
    }
    Ok(())
}
