use std::collections::BTreeMap;

use anyhow::{bail, Result};
use chrono::prelude::*;

use hortela::{
    ledger::{Transaction, Ledger},
    money::Money,
    parser::{self, Expr},
};

struct AccountRef {
    since: NaiveDate,
    balance: Money,
}

fn main() -> Result<()> {
    let parsed = parser::parse_file("test_cases/01-index.hortela")?;
    let mut result: Vec<Transaction> = vec![];

    for (id, expr) in parsed.into_iter().enumerate() {
        match expr {
            Expr::Open(_date, _acc, _balance) => {
                eprintln!("Not implemented yet, ignoring");
            }
            Expr::Balance(_date, _acc, _expected) => {
                eprintln!("Not implemented yet, ignoring");
            }
            Expr::Transaction(date, desc, movements) => {
                for (mov_id, movement) in movements.into_iter().enumerate() {
                    let transaction = movement.to_transaction(
                        (id * 100 + mov_id) as u64,
                        date,
                        desc.clone(),
                    );

                    result.push(transaction);
                }
            }
        }
    }

    let ledger: Ledger = result.into();
    ledger.validate()?;

    println!("Ok");

    Ok(())
}
