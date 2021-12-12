use anyhow::Result;
use chrono::prelude::*;

pub mod account;
pub mod ledger;
pub mod lexer;
pub mod money;
pub mod parser;
pub mod syntax;
pub mod utils;
pub mod validate;

use ledger::{Ledger, Transaction};
use money::Currency;
use syntax::{Expr, Span, Spanned};

#[derive(Debug)]
pub struct BalanceVerification {
    pub account: String,
    pub date: NaiveDate,
    pub amount: i64,
    pub currency: Currency,
    pub span: Span,
}

impl BalanceVerification {
    pub fn new(
        account: String,
        date: NaiveDate,
        amount: i64,
        currency: Currency,
        span: Span,
    ) -> Self {
        Self {
            account,
            date,
            amount,
            currency,
            span,
        }
    }
}

#[derive(Default)]
pub struct LedgerContext {
    pub balance_verifications: Vec<BalanceVerification>,
}

pub fn compute_program(program: Vec<Spanned<Expr>>) -> Result<(Ledger, LedgerContext)> {
    let mut context = LedgerContext::default();
    let mut result: Vec<Transaction> = vec![];
    let mut id: u64 = 1;

    for expr in program.into_iter() {
        match expr {
            (Expr::Open(_date, _acc, _balance), _) => {}
            (Expr::Balance((date, _), (account, _), (amount, _), (currency, _)), span) => {
                context.balance_verifications.push(BalanceVerification::new(
                    account.parts().join(":"),
                    date,
                    amount,
                    currency,
                    span,
                ));
            }
            (Expr::Transaction((date, _), (desc, _), (movements, _)), _) => {
                for (movement, _) in movements.into_iter() {
                    let transaction = movement.to_transaction(id, date, desc.clone());

                    result.push(transaction);

                    id += 1;
                }
            }
        }
    }

    Ok((result.into(), context))
}
