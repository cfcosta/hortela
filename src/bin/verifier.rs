use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(name = "file")]
    file: PathBuf
}

use hortela::{
    account::Account,
    ledger::{BalanceVerification, Ledger, Transaction},
    money::Movement,
    parser::{self, Expr},
};

fn main() -> Result<()> {
    let options = Options::from_args();

    let parsed = parser::parse_file(options.file)?;
    let mut result: Vec<Transaction> = vec![];
    let mut verifications: Vec<BalanceVerification> = vec![];
    let mut id: u64 = 1;

    for expr in parsed.into_iter() {
        match expr {
            Expr::Open(date, acc, balance) => {
                if balance.amount != 0.0 {
                    result.push(
                        Movement::debit(Account::void(), balance.clone()).to_transaction(
                            id as u64,
                            date,
                            String::from("Account opening"),
                        ),
                    );
                    id += 1;

                    result.push(Movement::credit(acc, balance).to_transaction(
                        id as u64,
                        date,
                        String::from("Account opening"),
                    ));
                    id += 1;
                }
            }
            Expr::Balance(date, account, expected) => {
                verifications.push(BalanceVerification {
                    date,
                    account,
                    expected,
                });
            }
            Expr::Transaction(date, desc, movements) => {
                for movement in movements.into_iter() {
                    let transaction = movement.to_transaction(id, date, desc.clone());

                    result.push(transaction);

                    id += 1;
                }
            }
        }
    }

    let ledger: Ledger = result.into();
    ledger.all()?;
    println!("Validating transactions internal state...");
    ledger.validate()?;
    println!("Validating balance statements...");
    ledger.validate_balances(verifications)?;

    Ok(())
}
