use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(name = "file")]
    file: PathBuf,
}

use hortela::{compute_program, syntax, validate::ValidationRunner};

fn main() -> Result<()> {
    let options = Options::from_args();
    let input = std::fs::read_to_string(&options.file)?;

    let parsed = syntax::parse_string(&options.file, &input)?;
    let (ledger, context) = compute_program(parsed)?;

    println!("Validating transactions internal state...");
    ValidationRunner::run_all(&options.file, &input, &ledger)?;
    println!("Validating balance statements...");
    ledger.validate_balances(context.balance_verifications)?;

    Ok(())
}
