use anyhow::Result;
use polars::prelude::*;
use thiserror::Error;

use crate::{ledger::Ledger, syntax::Span, utils};

mod runner;
pub use runner::Runner;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Validation error")]
    WithTrace(Vec<Trace>),

    #[error("Something happened with the Dataframe...")]
    DataframeError(#[from] polars::error::PolarsError),

    #[error("Some other weird error happened.")]
    OtherError(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    message: String,
    details: String,
    span: Option<Span>,
    found: Option<String>,
    expected: Option<String>,
}

type Validator = fn(&Ledger) -> Result<(), ValidationError>;

pub static ALL_VALIDATORS: &[(&'static str, Validator)] = &[
    (
        "validate that credits and debits balance",
        validate_credits_and_debits_balance,
    ),
    (
        "validate that all isolated transactions are properly balanced",
        validate_all_isolated_transactions_balance,
    ),
    (
        "validate that all balance statements are correct",
        validate_balance_statements,
    ),
];

fn validate_credits_and_debits_balance(ledger: &Ledger) -> Result<(), ValidationError> {
    let transactions = &ledger.transactions;
    let credit_sum: u64 = transactions
        .credits()?
        .column("transaction.amount")?
        .sum()
        .unwrap_or(0);
    let debit_sum: u64 = transactions
        .debits()?
        .column("transaction.amount")?
        .sum()
        .unwrap_or(0);

    if credit_sum == debit_sum {
        return Ok(());
    }

    Err(ValidationError::WithTrace(vec![Trace {
        message: "Budget does not balance".to_string(),
        details:
            "In a double-entry accounting system, all credits and debits should balance in the end."
                .into(),
        span: None,
        found: Some(format!("{:.1$}", credit_sum as i64 - debit_sum as i64, 2)),
        expected: Some(format!("0.0")),
    }]))
}

fn credit_factor_series(df: &DataFrame) -> Result<Series> {
    Ok(df
        .column("transaction.is_credit")?
        .bool()?
        .branch_apply_cast_numeric_no_null::<_, Float64Type>(|x| {
            if x == Some(true) {
                1.0
            } else {
                -1.0
            }
        })
        .into_series())
}

fn validate_all_isolated_transactions_balance(ledger: &Ledger) -> Result<(), ValidationError> {
    let mut df = ledger.transactions.all()?;

    df.replace(
        "transaction.amount",
        df.column("transaction.amount")?
            .multiply(&credit_factor_series(&df)?)?,
    )?;

    let grouped = df.groupby("transaction.parent_id")?;

    let sums = grouped.clone().sum()?;
    let mins = grouped.clone().min()?;
    let maxes = grouped.clone().max()?;

    let amount_sum = sums.column("transaction.amount_sum")?;
    let span_start = mins.column("transaction.span_start_min")?;
    let span_end = maxes.column("transaction.span_end_max")?;

    let df = DataFrame::new(vec![
        sums.column("transaction.parent_id")?.clone(),
        amount_sum.clone(),
        span_start.clone(),
        span_end.clone(),
    ])?;

    let unbalanced = df.column("transaction.amount_sum")?.f64()?.not_equal(0.0);

    let result = df.filter(&unbalanced)?;

    if result.shape().0 == 0 {
        return Ok(());
    }

    let mut errors = vec![];

    for i in 0..result.shape().0 {
        let item = result.get(i).unwrap();

        errors.push(Trace {
            message: "Transaction does not balance".into(),
            details: "Inside a transaction, all debits and credits must balance in the end."
                .to_string(),
            found: Some(format!("{:.1$}", item.get(1).unwrap(), 2)),
            expected: Some("0.0".to_string()),
            span: item.get(2).and_then(anyvalue_to_number).and_then(|start| {
                item.get(3)
                    .and_then(anyvalue_to_number)
                    .map(|end| (start as usize)..((end - 1) as usize))
            }),
        })
    }

    Err(ValidationError::WithTrace(errors))
}

fn anyvalue_to_number(v: &AnyValue) -> Option<u64> {
    match v {
        AnyValue::UInt64(v) => Some(*v),
        _ => None,
    }
}

fn validate_balance_statements(ledger: &Ledger) -> Result<(), ValidationError> {
    let mut df = ledger.transactions.all()?;
    let verifications = ledger.balance_verifications.all()?;

    df.replace(
        "transaction.amount",
        df.column("transaction.amount")?
            .multiply(&credit_factor_series(&df)?)?,
    )?;

    let per_day = df
        .groupby(&["transaction.date", "transaction.account_name"])?
        .agg(&[
            ("transaction.amount", &["cumsum"]),
            ("transaction.span_start", &["min"]),
            ("transaction.span_end", &["max"]),
        ])?
        .sort("transaction.date", false)?;

    let exploded_date = utils::explode_date_series(df.column("transaction.date")?)?;
    let accounts = df.column("transaction.account_name")?;
    let foo = DataFrame::new(vec![exploded_date])?;
    let bar = DataFrame::new(vec![accounts.clone()])?;
    let account_and_date = foo.cross_join(&bar)?;

    let balances_per_day = account_and_date.left_join(
        &per_day,
        &["transaction.date", "transaction.account_name"],
        &["transaction.date", "transaction.account_name"],
    )?;

    let with_verifications = dbg!(verifications.inner_join(
        &balances_per_day,
        &["verification.date", "verification.account_name"],
        &["transaction.date", "transaction.account_name"],
    )?);

    let unbalanced_mask = dbg!(with_verifications
        .column("transaction.amount_sum")?
        .not_equal(with_verifications.column("verification.amount")?));

    let unbalanced_transactions = dbg!(with_verifications.filter(&unbalanced_mask)?);

    if unbalanced_transactions.shape().0 == 0 {
        return Ok(());
    }

    let mut errors = vec![];

    for i in 0..unbalanced_transactions.shape().0 {
        let item = unbalanced_transactions.get(i).unwrap();

        errors.push(Trace {
            message: "Transaction does not balance".into(),
            details: "Inside a transaction, all debits and credits must balance in the end."
                .to_string(),
            found: Some(format!("{:.1$}", item.get(1).unwrap(), 2)),
            expected: Some("0.0".to_string()),
            span: item.get(2).and_then(anyvalue_to_number).and_then(|start| {
                item.get(3)
                    .and_then(anyvalue_to_number)
                    .map(|end| (start as usize)..((end - 1) as usize))
            }),
        })
    }

    Err(ValidationError::WithTrace(errors))
}
