//! Exact nonnegative cost arithmetic, before the wire's finite-number boundary.
//!
//! [Decision Log]
//! - Purpose: a small report must survive addition to a much larger report.
//! - Constraint: Decimal's ordinary parser and rescaling arithmetic can round;
//!   converting each partial sum to f64 also loses independently reported costs.
//! - Choice: accept only exact Decimal inputs, accumulate their integer multiples
//!   of 1e-28, then check the final JSON decimal spelling once. num-bigint avoids
//!   both scale alignment loss and an order-dependent intermediate overflow.
//! - Bound: 100,000 ledger rows and 32 units keep each accumulator below 210 bits.
//!   A result outside Decimal or losslessly representable JSON numbers is absent
//!   with explicit coverage, never a silently rounded amount.

use num_bigint::BigUint;
use num_traits::CheckedSub as _;
use rust_decimal::Decimal;
use std::ops::AddAssign as _;
use vessel_proto::conversation::FiniteNumber;

const SCALE: u32 = Decimal::MAX_SCALE;

#[derive(Debug, Default)]
pub(super) struct Amount(BigUint);

impl Amount {
    pub(super) fn reported(value: FiniteNumber) -> Option<Self> {
        let amount = exact_decimal(value)?;
        let mantissa = u128::try_from(amount.mantissa()).ok()?;
        let scale = BigUint::from(10_u8).pow(SCALE.checked_sub(amount.scale())?);
        Some(Self(std::ops::Mul::mul(BigUint::from(mantissa), scale)))
    }

    pub(super) fn add(&mut self, other: Self) {
        self.0.add_assign(other.0);
    }

    pub(super) fn difference(self, previous: &Self) -> Option<Self> {
        self.0.checked_sub(&previous.0).map(Self)
    }

    pub(super) fn wire(self) -> Option<FiniteNumber> {
        let scale = usize::try_from(SCALE).ok()?;
        let width = scale.checked_add(1)?;
        let digits = format!("{:0>width$}", self.0.to_str_radix(10));
        let (whole, fraction) = digits.split_at_checked(digits.len().checked_sub(scale)?)?;
        let fraction = fraction.trim_end_matches('0');
        let spelling = if fraction.is_empty() {
            whole.to_owned()
        } else {
            format!("{whole}.{fraction}")
        };
        let expected = Decimal::from_str_exact(&spelling).ok()?;
        let number = spelling.parse::<f64>().ok().and_then(FiniteNumber::new)?;
        // Compare the domain chosen at input (the finite JSON number's decimal
        // spelling), not a binary approximation and not a rounded Decimal parse.
        (exact_decimal(number)? == expected).then_some(number)
    }
}

fn exact_decimal(value: FiniteNumber) -> Option<Decimal> {
    Decimal::from_str_exact(&value.get().to_string()).ok()
}
