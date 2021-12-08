use std::collections::BTreeMap;

use anyhow::{ Result, bail };
use chrono::prelude::*;

use hortela::parser::{self, Expr, Account, Money};

struct AccountRef {
    since: NaiveDate,
    balance: Money
}

fn main() -> Result<()> {
    let parsed = parser::parse_file("test_cases/01-index.hortela")?;
    let mut accounts = BTreeMap::new();

    for expr in parsed.into_iter() {
        match expr {
            Expr::Open(date, acc, balance) => {
                accounts.insert(acc.clone(), AccountRef { since: date, balance });
            },
            Expr::Balance(_date, acc, expected) => {
                match accounts.get(&acc) {
                    Some(acc_ref) => {
                        if acc_ref.balance != expected {
                            bail!("Account balance does not match, expected {:?}, got {:?}", expected, acc_ref.balance);
                        }
                    }
                    None => bail!("Account {:?} is not initialized", acc)
                }
            }
            _ => {}
        }
    }

    println!("Ok");

    Ok(())
}
