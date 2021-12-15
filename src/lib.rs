use anyhow::Result;

pub mod account;
pub mod ledger;
pub mod money;
pub mod syntax;
pub mod utils;
pub mod validate;

use ledger::{AccountOpening, BalanceVerification, Ledger, Transaction};
use syntax::{Op, Spanned};

pub fn compute_program(program: Vec<Spanned<Op>>) -> Result<Ledger> {
    let mut transactions: Vec<Transaction> = vec![];
    let mut verifications: Vec<BalanceVerification> = vec![];
    let mut openings: Vec<AccountOpening> = vec![];
    let mut id: u64 = 1;

    for (expr, span) in program.into_iter() {
        match expr {
            Op::Open((date, _), (account, _), (currency, _)) => {
                openings.push(AccountOpening::new(id, account, date, currency, span));
                id += 1;
            }
            Op::Balance((date, _), (account, _), (amount, _)) => {
                verifications.push(BalanceVerification::new(id, account, date, amount, span));
                id += 1;
            }
            Op::Transaction((date, _), (desc, _), (movements, _)) => {
                let parent = Some(id);

                for (movement, span) in movements.into_iter() {
                    let transaction = movement.to_transaction(id, date, desc.clone(), span, parent);

                    transactions.push(transaction);

                    id += 1;
                }
            }
        }
    }

    Ok(Ledger {
        transactions: transactions.into(),
        balance_verifications: verifications.into(),
        account_openings: openings.into(),
    })
}
