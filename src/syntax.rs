use chrono::prelude::*;
use num::BigRational;

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
    Balance(Spanned<NaiveDate>, Spanned<Account>, Spanned<Money>),
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
}

impl Keyword {
    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "open" => Some(Self::Open),
            "balance" => Some(Self::Balance),
            "transaction" => Some(Self::Transaction),
            _ => None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Date(NaiveDate),
    Account(Account),
    Currency(String),
    Amount(BigRational, String),
    Keyword(Keyword),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Comment(String),
    Identifier(String),
    Movement(MovementKind),
    String(String),
    Currency(String),
    Number(BigRational),
    Separator(char),
}

impl Token {
    pub fn number(n: f64) -> Self {
        Self::Number(BigRational::from_float(n).unwrap())
    }

    pub fn identifier<T: Into<String>>(id: T) -> Self {
        Self::Identifier(id.into())
    }

    pub fn currency<T: Into<String>>(id: T) -> Self {
        Self::Currency(id.into())
    }
}

impl std::fmt::Display for Token {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Token::Comment(_) => write!(f, "comment"),
            Token::Identifier(id) => write!(f, "{}", id),
            Token::String(id) => write!(f, "{:?}", id),
            Token::Movement(mov) => write!(
                f,
                "{}",
                match mov {
                    MovementKind::Credit => ">",
                    MovementKind::Debit => "<",
                }
            ),
            Token::Currency(cur) => write!(f, "{}", cur),
            Token::Number(n) => write!(f, "{}", n),
            Token::Separator(c) => write!(f, "{}", c),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sign {
    Positive,
    Negative,
}

impl From<f64> for Sign {
    fn from(num: f64) -> Self {
        if num < 0.0 {
            Sign::Negative
        } else {
            Sign::Positive
        }
    }
}

impl From<i64> for Sign {
    fn from(num: i64) -> Self {
        if num < 0 {
            Sign::Negative
        } else {
            Sign::Positive
        }
    }
}

impl Sign {
    pub fn flip(&self) -> Self {
        match self {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Sign::Negative => -1.0,
            Sign::Positive => 1.0,
        }
    }

    pub fn to_i64(&self) -> i64 {
        match self {
            Sign::Negative => -1,
            Sign::Positive => 1,
        }
    }
}
