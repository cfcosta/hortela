use chrono::NaiveDate;

use crate::{account::Account, ledger::Transaction};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Money {
    pub amount: f64,
    pub currency: String,
}

impl Money {
    pub fn new(amount: f64, currency: &str) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Movement {
    Credit(Account, Money),
    Debit(Account, Money),
}

impl Movement {
    pub fn credit(acc: Account, money: Money) -> Self {
        Self::Credit(acc, money)
    }

    pub fn debit(acc: Account, money: Money) -> Self {
        Self::Debit(acc, money)
    }

    pub fn acc<'a>(&'a self) -> Account {
        match self {
            Self::Credit(acc, _) => acc,
            Self::Debit(acc, _) => acc,
        }
        .clone()
    }

    pub fn amount<'a>(&'a self) -> Money {
        match self {
            Self::Credit(_, money) => money,
            Self::Debit(_, money) => money,
        }
        .clone()
    }

    pub fn is_credit(&self) -> bool {
        match self {
            Movement::Credit(..) => true,
            Movement::Debit(..) => false,
        }
    }

    fn kind(&self) -> MovementKind {
        match self {
            Self::Credit(..) => MovementKind::Credit,
            Self::Debit(..) => MovementKind::Debit,
        }
    }

    pub fn to_transaction(self, id: u64, date: NaiveDate, description: String) -> Transaction {
        let money = self.amount();
        let (currency, amount) = (money.currency, money.amount);
        let (account_name, account_kind) = self.acc().parts();

        Transaction {
            id,
            date,
            description,
            account_name,
            account_kind,
            amount,
            currency,
            signed_amount: amount * (self.acc().signed_factor(self.kind()) as f64),
            is_credit: self.is_credit(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum MovementKind {
    Credit,
    Debit,
}
