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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
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
    Description(String)
}

impl Expr {
    pub fn get_date(&self) -> Option<NaiveDate> {
        match self {
            Expr::Date(d) => Some(*d),
            _ => None
        }
    }

    pub fn get_account(&self) -> Option<Account> {
        match self {
            Expr::Account(a) => Some(a.clone()),
            _ => None
        }
    }

    pub fn get_currency(&self) -> Option<String> {
        match self {
            Expr::Currency(c) => Some(c.clone()),
            _ => None
        }
    }

    pub fn get_money(&self) -> Option<Money> {
        match self {
            Expr::Amount(a, c) => Some(Money::new(a.clone(), c.clone())),
            _ => None
        }
    }

    pub fn get_description(&self) -> Option<String> {
        match self {
            Expr::Description(d) => Some(d.clone()),
            _ => None
        }
    }
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

    pub fn is_comment(&self) -> bool {
        match self {
            Token::Comment(_) => true,
            _ => false
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            Token::Identifier(_) => true,
            _ => false
        }
    }

    pub fn is_movement(&self) -> bool {
        match self {
            Token::Movement(_) => true,
            _ => false
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            Token::String(_) => true,
            _ => false
        }
    }

    pub fn is_currency(&self) -> bool {
        match self {
            Token::Currency(_) => true,
            _ => false
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            Token::Number(_) => true,
            _ => false
        }
    }

    pub fn is_separator(&self) -> bool {
        match self {
            Token::Separator(_) => true,
            _ => false
        }
    }

    pub fn get_number(&self) -> Option<BigRational> {
        match self {
            Token::Number(n) => Some(n.clone()),
            _ => None
        }
    }

    pub fn get_string(&self) -> Option<String> {
        match self {
            Token::String(s) => Some(s.clone()),
            _ => None
        }
    }

    pub fn get_movement_kind(&self) -> Option<MovementKind> {
        match self {
            Token::Movement(m) => Some(m.clone()),
            _ => None
        }
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
