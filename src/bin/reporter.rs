use std::path::{Path, PathBuf};

use anyhow::Result;
use polars::prelude::*;
use structopt::StructOpt;

use hortela::{compute_program, parser, utils};

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
        .not_equal(&utils::repeater("void", all.shape().0));
    let void_account_kind = by_account_kind
        .column("ledger.account_kind")?
        .not_equal(&utils::repeater("void", all.shape().0));

    by_account.replace(
        "ledger.amount_sum",
        utils::round_to_fixed(by_account.column("ledger.amount_sum")?, 2)?,
    )?;
    by_account_kind.replace(
        "ledger.amount_sum",
        utils::round_to_fixed(by_account_kind.column("ledger.amount_sum")?, 2)?,
    )?;
    by_account.replace(
        "ledger.signed_amount_sum",
        utils::round_to_fixed(by_account.column("ledger.signed_amount_sum")?, 2)?,
    )?;
    by_account_kind.replace(
        "ledger.signed_amount_sum",
        utils::round_to_fixed(by_account_kind.column("ledger.signed_amount_sum")?, 2)?,
    )?;

    by_account = by_account.filter(&void_account)?;
    by_account_kind = by_account_kind.filter(&void_account_kind)?;

    dbg!(by_account);
    dbg!(by_account_kind);

    Ok(())
}
