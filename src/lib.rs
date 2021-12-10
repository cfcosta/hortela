use chrono::prelude::*;
use anyhow::Result;

pub mod account;
pub mod ledger;
pub mod money;
pub mod parser;
pub mod validate;
pub mod utils;

use ledger::{Ledger, Transaction};
use money::{ Movement, Money };
use parser::Expr;
use account::Account;

#[derive(Debug)]
pub struct BalanceVerification {
    pub account: Account,
    pub date: NaiveDate,
    pub expected: Money,
}

impl BalanceVerification {
    pub fn new(account: Account, date: NaiveDate, expected: Money) -> Self {
        Self {
            account,
            date,
            expected
        }
    }
}

#[derive(Default)]
pub struct LedgerContext {
    pub balance_verifications: Vec<BalanceVerification>,
}

pub fn compute_program(program: Vec<Expr>) -> Result<(Ledger, LedgerContext)> {
    let mut context = LedgerContext::default();
    let mut result: Vec<Transaction> = vec![];
    let mut id: u64 = 1;

    for expr in program.into_iter() {
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
                context.balance_verifications.push(BalanceVerification {
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

    Ok((result.into(), context))
}
