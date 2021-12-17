use std::path::{Path, PathBuf};

use anyhow::Result;
use polars::prelude::*;
use structopt::StructOpt;

use hortela::{compute_program, syntax};

#[derive(StructOpt)]
pub struct Options {
    #[structopt(subcommand)]
    reporter: Reporter,
}

#[derive(StructOpt)]
pub struct GlobalOptions {
    #[structopt(name = "file")]
    file: PathBuf,
}

#[derive(StructOpt)]
pub enum Reporter {
    #[structopt(name = "balance")]
    BalanceSheet {
        #[structopt(flatten)]
        global: GlobalOptions,
    },
}

impl Reporter {
    pub fn file(&self) -> &Path {
        match self {
            Self::BalanceSheet {
                global: GlobalOptions { file },
            } => file,
        }
    }
}

fn sums_by_account(df: &DataFrame, amount_column_name: &str) -> Result<DataFrame> {
    let mut sums = df
        .clone()
        .select(&[
            "transaction.account_name",
            "transaction.amount",
            "transaction.signed_amount",
        ])?
        .groupby("transaction.account_name")?
        .sum()?;

    sums.rename(
        "transaction.amount_sum",
        &format!("transaction.{}", amount_column_name),
    )?;

    sums.rename(
        "transaction.signed_amount_sum",
        &format!("transaction.signed_{}", amount_column_name),
    )?;

    Ok(sums)
}

fn main() -> Result<()> {
    let options = Options::from_args();
    let ledger = compute_program(syntax::parse_file(options.reporter.file())?)?;
    let transactions = ledger.transactions;

    let credits = sums_by_account(&transactions.credits()?, "credits")?;
    let debits = sums_by_account(&transactions.debits()?, "debits")?;

    dbg!(credits.left_join(
        &debits,
        "transaction.account_name",
        "transaction.account_name"
    )?);

    Ok(())
}
