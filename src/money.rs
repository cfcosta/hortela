use chrono::NaiveDate;
use num::{BigRational, BigInt, FromPrimitive};

use crate::{account::Account, ledger::Transaction};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Money {
    pub amount: BigRational,
    pub currency: Currency
}

impl Money {
    pub fn new<C: Into<Currency>>(amount: BigRational, currency: C) -> Self {
        Self {
            amount,
            currency: currency.into()
        }
    }
}

impl std::fmt::Display for Money {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, self.currency.0)
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Currency(pub String);

impl From<Currency> for String {
    fn from(val: Currency) -> Self {
        val.0
    }
}

impl From<String> for Currency {
    fn from(val: String) -> Self {
        Self(val.to_string())
    }
}

impl From<&str> for Currency {
    fn from(val: &str) -> Self {
        Self::from(val.to_string())
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub enum MovementKind {
    Credit,
    Debit,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Movement(pub MovementKind, pub Money, pub Account);

impl Movement {
    pub fn credit(acc: Account, money: Money) -> Self {
        Self(MovementKind::Credit, money, acc)
    }

    pub fn debit(acc: Account, money: Money) -> Self {
        Self(MovementKind::Debit, money, acc)
    }

    pub fn acc<'a>(&'a self) -> Account {
        self.2.clone()
    }

    pub fn amount<'a>(&'a self) -> Money {
        self.1.clone()
    }

    pub fn is_credit(&self) -> bool {
        self.0 == MovementKind::Credit
    }

    pub fn to_transaction(self, id: u64, date: NaiveDate, description: String) -> Transaction {
        let Money { amount, currency } = self.1.clone();
        let parts = self.2.parts();
        let factor = BigInt::from_i64(self.2.signed_factor(self.0.clone())).unwrap();

        Transaction {
            id,
            date,
            description,
            account_parts: parts.clone(),
            account_name: parts.join(":"),
            amount: amount.clone(),
            currency: currency.into(),
            signed_amount: amount * factor,
            is_credit: self.is_credit(),
        }
    }
}
