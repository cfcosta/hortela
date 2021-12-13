use account::Account;
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
use money::Money;
use syntax::{CleanOp, Op, Span, Spanned};

#[derive(Debug, Clone)]
pub struct BalanceVerification {
    pub account: Account,
    pub date: NaiveDate,
    pub amount: Money,
    pub span: Span,
}

impl BalanceVerification {
    pub fn new(account: Account, date: NaiveDate, amount: Money, span: Span) -> Self {
        Self {
            account,
            date,
            amount,
            span,
        }
    }
}

#[derive(Default)]
pub struct LedgerContext {
    pub balance_verifications: Vec<BalanceVerification>,
}

pub fn compute_program(program: Vec<Spanned<Op>>) -> Result<(Ledger, LedgerContext)> {
    let mut context = LedgerContext::default();
    let mut result: Vec<Transaction> = vec![];
    let mut id: u64 = 1;

    for (expr, span) in program.into_iter() {
        match expr.into() {
            CleanOp::Open(_date, _acc, _balance) => {}
            CleanOp::Balance(date, account, amount) => {
                context
                    .balance_verifications
                    .push(BalanceVerification::new(account, date, amount, span));
            }
            CleanOp::Transaction(date, desc, movements) => {
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
