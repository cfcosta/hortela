use std::path::Path;

use anyhow::{bail, Result};
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use polars::prelude::*;
use thiserror::Error;

use crate::{ledger::Ledger, syntax::Span};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Validation error")]
    WithTrace(Vec<ValidationTrace>),

    #[error("Something happened with the Dataframe...")]
    DataframeError(#[from] polars::error::PolarsError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationTrace {
    message: String,
    details: String,
    span: Option<Span>,
    found: Option<String>,
    expected: Option<String>,
}

pub struct ValidationRunner;

impl ValidationRunner {
    pub fn run_all(filename: &Path, input: &str, ledger: &Ledger) -> Result<()> {
        for (name, validator) in ALL_VALIDATORS {
            print!("Running validator: {}...", name);

            match validator(&ledger.clone()) {
                Ok(_) => {
                    println!(" OK");
                }
                Err(ValidationError::WithTrace(traces)) => {
                    println!(" ERROR");

                    traces.into_iter().for_each(|t| {
                        let span = t.span.clone().unwrap_or(0..1);
                        let report = Report::build(ReportKind::Error, (), span.start);

                        let message_parts = vec![
                            Some(t.message),
                            t.expected
                                .map(|x| format!("`{}`", x).fg(Color::Red).to_string()),
                            t.found.map(|f| format!("found {}", f.fg(Color::Blue))),
                        ];

                        let message = message_parts
                            .into_iter()
                            .filter_map(|x| x)
                            .collect::<Vec<String>>();

                        let mut report = report.with_message(message.join(", "));

                        if t.span.is_some() {
                            report = report.with_label(
                                Label::new(span)
                                    .with_message(t.details)
                                    .with_color(Color::Blue),
                            );
                        }

                        report.finish().eprint(Source::from(&input)).unwrap();
                    });

                    bail!("Running validation `{}` failed.", name.fg(Color::Green));
                }
                Err(e) => bail!(e),
            }
        }

        Ok(())
    }
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

    Err(ValidationError::WithTrace(vec![ValidationTrace {
        message: "Budget does not balance".to_string(),
        details:
            "In a double-entry accounting system, all credits and debits should balance in the end."
                .into(),
        span: None,
        found: Some(format!("{:.1$}", credit_sum as i64 - debit_sum as i64, 2)),
        expected: Some(format!("0.0")),
    }]))
}

fn validate_all_isolated_transactions_balance(ledger: &Ledger) -> Result<(), ValidationError> {
    let mut df = ledger.transactions.all()?;

    let credit_factor = df
        .column("transaction.is_credit")?
        .bool()?
        .branch_apply_cast_numeric_no_null::<_, Float64Type>(|x| {
            if x == Some(true) {
                1.0
            } else {
                -1.0
            }
        })
        .into_series();

    df.replace(
        "transaction.amount",
        df.column("transaction.amount")?.multiply(&credit_factor)?,
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

    let to_number = |v: &AnyValue| -> Option<u64> {
        match v {
            AnyValue::UInt64(v) => Some(*v),
            _ => None,
        }
    };

    for i in 0..result.shape().0 {
        let item = result.get(i).unwrap();

        errors.push(ValidationTrace {
            message: "Transaction does not balance".into(),
            details: "Inside a transaction, all debits and credits must balance in the end."
                .to_string(),
            found: Some(format!("{:.1$}", item.get(1).unwrap(), 2)),
            expected: Some("0.0".to_string()),
            span: item.get(2).and_then(to_number).and_then(|start| {
                item.get(3)
                    .and_then(to_number)
                    .map(|end| (start as usize)..((end - 1) as usize))
            }),
        })
    }

    Err(ValidationError::WithTrace(errors))
}
