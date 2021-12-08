use std::fmt::{self, Display};

use crate::money::MovementKind;

#[derive(Clone, Copy)]
pub enum AccountType {
    Assets,
    Liabilities,
    Income,
    Equity,
    Expenses,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Account {
    Assets(String),
    Liabilities(String),
    Income(String),
    Equity(String),
    Expenses(String),
}

impl Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (kind, name) = match self {
            Self::Assets(name) => ("assets", name),
            Self::Liabilities(name) => ("liabilities", name),
            Self::Income(name) => ("income", name),
            Self::Equity(name) => ("equity", name),
            Self::Expenses(name) => ("expenses", name),
        };

        write!(f, "{}:{}", kind, name)
    }
}

impl Account {
    pub fn signed_factor(&self, movement_kind: MovementKind) -> isize {
        match movement_kind {
            MovementKind::Debit => match self {
                Account::Assets(_) => 1,
                Account::Liabilities(_) => -1,
                Account::Income(_) => -1,
                Account::Equity(_) => -1,
                Account::Expenses(_) => 1,
            },
            MovementKind::Credit => match self {
                Account::Assets(_) => -1,
                Account::Liabilities(_) => 1,
                Account::Income(_) => 1,
                Account::Equity(_) => 1,
                Account::Expenses(_) => -1,
            },
        }
    }
}
