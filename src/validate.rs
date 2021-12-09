use anyhow::{bail, Result};
use thiserror::Error;

use crate::ledger::Ledger;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Credits and debits do not match, difference: {0}")]
    UnmatchedMovements(f64),
}

type Validator = fn(&Ledger) -> Result<()>;

pub static ALL_VALIDATORS: &[(&'static str, Validator)] = &[(
    "validate that credits and debits balance",
    validate_credits_and_debits_balance,
)];

fn validate_credits_and_debits_balance(ledger: &Ledger) -> Result<()> {
    let credit_sum: f64 = ledger
        .credits()?
        .column("ledger.amount")?
        .sum()
        .unwrap_or(0.0);
    let debit_sum: f64 = ledger
        .debits()?
        .column("ledger.amount")?
        .sum()
        .unwrap_or(0.0);

    if credit_sum != debit_sum {
        bail!(ValidationError::UnmatchedMovements(credit_sum - debit_sum));
    }

    Ok(())
}
