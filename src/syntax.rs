use chrono::prelude::*;

use crate::{ money::*, account::* };

type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Open(NaiveDate, Account, Currency),
    Balance(NaiveDate, Account, Money),
    Transaction(NaiveDate, String, Vec<Movement>),
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
