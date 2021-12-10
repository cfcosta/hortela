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
        match self {
            Self::Assets(name) => write!(f, "assets:{}", name),
            Self::Liabilities(name) => write!(f, "liabilities:{}", name),
            Self::Income(name) => write!(f, "income:{}", name),
            Self::Equity(name) => write!(f, "equity:{}", name),
            Self::Expenses(name) => write!(f, "expenses:{}", name),
        }
    }
}

impl Account {
    pub fn parts(&self) -> (String, String) {
        let (kind, name) = match self {
            Self::Assets(name) => (String::from("assets"), name.clone()),
            Self::Liabilities(name) => (String::from("liabilities"), name.clone()),
            Self::Income(name) => (String::from("income"), name.clone()),
            Self::Equity(name) => (String::from("equity"), name.clone()),
            Self::Expenses(name) => (String::from("expenses"), name.clone()),
        };

        (name.clone(), kind.clone())
    }

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
