use anyhow::Result;

use hortela::{
    account::Account,
    ledger::{Ledger, Transaction},
    money::Movement,
    parser::{self, Expr},
};

fn main() -> Result<()> {
    let parsed = parser::parse_file("test_cases/01-index.hortela")?;
    let mut result: Vec<Transaction> = vec![];
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
            Expr::Balance(_date, _acc, _expected) => {
                eprintln!("Not implemented yet, ignoring");
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
    dbg!(ledger.all()?);
    ledger.validate()?;

    println!("Ok");

    Ok(())
}
