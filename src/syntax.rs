use chrono::prelude::*;

use crate::{account::*, money::*};

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Debug, PartialEq, Clone)]
pub enum CleanExpr {
    Open(NaiveDate, Account, Currency),
    Balance(NaiveDate, Account, i64, Currency),
    Transaction(NaiveDate, String, Vec<Movement>),
}

impl From<Expr> for CleanExpr {
    fn from(from: Expr) -> Self {
        match from {
            Expr::Open(a, b, c) => Self::Open(a.0, b.0, c.0),
            Expr::Balance(a, b, c, d) => Self::Balance(a.0, b.0, c.0, d.0),
            Expr::Transaction(a, b, c) => {
                Self::Transaction(a.0, b.0, c.0.into_iter().map(|(x, _)| x).collect())
            }
        }
    }
}

impl From<Spanned<Expr>> for CleanExpr {
    fn from((from, _): Spanned<Expr>) -> Self {
        from.into()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Open(Spanned<NaiveDate>, Spanned<Account>, Spanned<Currency>),
    Balance(
        Spanned<NaiveDate>,
        Spanned<Account>,
        Spanned<i64>,
        Spanned<Currency>,
    ),
    Transaction(
        Spanned<NaiveDate>,
        Spanned<String>,
        Spanned<Vec<Spanned<Movement>>>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Keyword {
    Open,
    Balance,
    Transaction,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Comment(String),
    Date(NaiveDate),
    Amount(u64),
    NegativeAmount(i64),
    Description(String),
    Currency(Currency),
    Keyword(Keyword),
    Account(AccountType, Vec<String>),
    Movement(MovementKind),
}

impl Token {
    pub fn amount(&self) -> Option<u64> {
        match self {
            Self::Amount(a) => Some(*a),
            _ => None,
        }
    }

    pub fn currency(&self) -> Option<Currency> {
        match self {
            Self::Currency(c) => Some(c.clone()),
            _ => None,
        }
    }
}
