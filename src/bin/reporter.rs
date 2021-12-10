use std::path::{Path, PathBuf};

use anyhow::Result;
use polars::prelude::*;
use structopt::StructOpt;

use hortela::{compute_program, parser};

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

fn repeater<T: Clone>(value: T, amount: usize) -> Series
where
    Series: FromIterator<T>,
{
    Series::from_iter(std::iter::repeat(value).take(amount))
}

fn round_to_fixed<T: Into<f64>>(series: &Series, precision: T) -> Result<Series> {
    let repeat100 = repeater(10_f64.powf(precision.into()), series.len());

    Ok(series
        .f64()?
        .multiply(&repeat100)?
        .cast(&DataType::Int64)?
        .cast(&DataType::Float64)?
        .divide(&repeat100)?)
}

fn main() -> Result<()> {
    let options = Options::from_args();

    let (ledger, _) = compute_program(parser::parse_file(options.reporter.file())?)?;

    let all = ledger.all()?;

    let transactions = all.select(&[
        "ledger.account_kind",
        "ledger.account_name",
        "ledger.amount",
        "ledger.currency",
        "ledger.signed_amount",
    ])?;

    let mut by_account = transactions
        .groupby(&["ledger.account_kind", "ledger.account_name"])?
        .sum()?;

    let mut by_account_kind = transactions.groupby("ledger.account_kind")?.sum()?;

    let void_account = by_account
        .column("ledger.account_kind")?
        .not_equal(&repeater("void", all.shape().0));
    let void_account_kind = by_account_kind
        .column("ledger.account_kind")?
        .not_equal(&repeater("void", all.shape().0));

    by_account.replace(
        "ledger.amount_sum",
        round_to_fixed(by_account.column("ledger.amount_sum")?, 2)?,
    )?;
    by_account_kind.replace(
        "ledger.amount_sum",
        round_to_fixed(by_account_kind.column("ledger.amount_sum")?, 2)?,
    )?;
    by_account.replace(
        "ledger.signed_amount_sum",
        round_to_fixed(by_account.column("ledger.signed_amount_sum")?, 2)?,
    )?;
    by_account_kind.replace(
        "ledger.signed_amount_sum",
        round_to_fixed(by_account_kind.column("ledger.signed_amount_sum")?, 2)?,
    )?;

    by_account = by_account.filter(&void_account)?;
    by_account_kind = by_account_kind.filter(&void_account_kind)?;

    dbg!(by_account);
    dbg!(by_account_kind);

    Ok(())
}
