use chrono::prelude::*;

use crate::{account::*, money::*};

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Debug, PartialEq, Clone)]
pub enum CleanOp {
    Open(NaiveDate, Account, Currency),
    Balance(NaiveDate, Account, Money),
    Transaction(NaiveDate, String, Vec<Movement>),
}

impl From<Op> for CleanOp {
    fn from(from: Op) -> Self {
        match from {
            Op::Open(a, b, c) => Self::Open(a.0, b.0, c.0),
            Op::Balance(a, b, m) => Self::Balance(a.0, b.0, m.0),
            Op::Transaction(a, b, c) => {
                Self::Transaction(a.0, b.0, c.0.into_iter().map(|(x, _)| x).collect())
            }
        }
    }
}

impl From<Spanned<Op>> for CleanOp {
    fn from((from, _): Spanned<Op>) -> Self {
        from.into()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Op {
    Open(Spanned<NaiveDate>, Spanned<Account>, Spanned<Currency>),
    Balance(
        Spanned<NaiveDate>,
        Spanned<Account>,
        Spanned<Money>
    ),
    Transaction(
        Spanned<NaiveDate>,
        Spanned<String>,
        Spanned<Vec<Spanned<Movement>>>,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    Open,
    Balance,
    Transaction,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Date(NaiveDate),
    Account(Account),
    Currency(String),
    Amount(u64, Sign, String),
    Keyword(Keyword)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Comment(String),
    Identifier(String),
    Movement(MovementKind),
    String(String),
    Currency(String),
    Number(u64, Sign),
    Separator(char),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sign {
    Positive,
    Negative,
}

impl Sign {
    pub fn flip(&self) -> Self {
        match self {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
        }
    }
}
