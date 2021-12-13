use std::{
    collections::VecDeque,
    fmt::{self, Display},
};

use crate::money::MovementKind;

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AccountType {
    Assets,
    Liabilities,
    Income,
    Equity,
    Expenses,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Account(pub AccountType, pub Vec<String>);

impl Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            AccountType::Assets => write!(f, "assets:{}", self.1.join(":")),
            AccountType::Liabilities => write!(f, "liabilities:{}", self.1.join(":")),
            AccountType::Income => write!(f, "income:{}", self.1.join(":")),
            AccountType::Equity => write!(f, "equity:{}", self.1.join(":")),
            AccountType::Expenses => write!(f, "expenses:{}", self.1.join(":")),
        }
    }
}

impl Account {
    pub fn parts(&self) -> Vec<String> {
        let mut parts = VecDeque::from(self.1.clone());

        let kind = match self.0 {
            AccountType::Assets => "assets",
            AccountType::Liabilities => "liabilities",
            AccountType::Income => "income",
            AccountType::Equity => "equity",
            AccountType::Expenses => "expenses",
        }
        .into();

        parts.push_front(kind);

        parts.into()
    }

    pub fn to_string(&self) -> String {
        self.parts().join(":")
    }

    pub fn signed_factor(&self, movement_kind: MovementKind) -> i64 {
        match movement_kind {
            MovementKind::Debit => match self.0 {
                AccountType::Assets => 1,
                AccountType::Liabilities => -1,
                AccountType::Income => -1,
                AccountType::Equity => -1,
                AccountType::Expenses => 1,
            },
            MovementKind::Credit => match self.0 {
                AccountType::Assets => -1,
                AccountType::Liabilities => 1,
                AccountType::Income => 1,
                AccountType::Equity => 1,
                AccountType::Expenses => -1,
            },
        }
    }
}
