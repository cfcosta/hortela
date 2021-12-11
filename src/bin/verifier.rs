use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(name = "file")]
    file: PathBuf,
}

use hortela::{compute_program, parser};

fn main() -> Result<()> {
    let options = Options::from_args();

    let parsed = parser::parse_file(options.file)?;

    let (ledger, context) = compute_program(parsed)?;
    ledger.all()?;
    println!("Validating transactions internal state...");
    ledger.validate()?;
    println!("Validating balance statements...");
    ledger.validate_balances(context.balance_verifications)?;

    Ok(())
}
