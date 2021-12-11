use chrono::NaiveDate;

use crate::{account::Account, ledger::Transaction};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Money(pub u64, pub Currency);

impl Money {
    pub fn new(amount: u64, currency: &str) -> Self {
        Self(amount, currency.into())
    }

    pub fn from_float(amount: f64, currency: &str) -> Self {
        Self((amount * 10f64.powi(8)) as u64, currency.into())
    }

    pub fn to_float(&self) -> f64 {
        (self.0 as f64) / 10f64.powi(8) 
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Currency(String);

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
        let Money(amount, currency) = self.1.clone();
        let parts = self.2.parts();

        Transaction {
            id,
            date,
            description,
            account_parts: parts.clone(),
            account_name: parts.join(":"),
            amount,
            currency: currency.into(),
            signed_amount: amount as i64 * (self.2.signed_factor(self.0.clone())),
            is_credit: self.is_credit(),
        }
    }
}
