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
            "ledger.account_name",
            "ledger.amount",
            "ledger.signed_amount",
        ])?
        .groupby("ledger.account_name")?
        .sum()?;

    sums.rename(
        "ledger.amount_sum",
        &format!("ledger.{}", amount_column_name),
    )?;

    sums.rename(
        "ledger.signed_amount_sum",
        &format!("ledger.signed_{}", amount_column_name),
    )?;

    Ok(sums)
}

fn main() -> Result<()> {
    let options = Options::from_args();
    let (ledger, _) = compute_program(syntax::parse_file(options.reporter.file())?)?;

    let credits = sums_by_account(&ledger.credits()?, "credits")?;
    let debits = sums_by_account(&ledger.debits()?, "debits")?;

    dbg!(credits.left_join(&debits, "ledger.account_name", "ledger.account_name")?);

    Ok(())
}
