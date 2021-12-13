use chrono::NaiveDate;

use crate::{account::Account, ledger::Transaction, syntax::Sign};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Money {
    pub amount: u64,
    pub sign: Sign,
    pub currency: Currency
}

impl Money {
    pub fn new<T: Into<Currency>>(amount: u64, sign: Sign, currency: T) -> Self {
        Self { amount, sign, currency: currency.into() }
    }

    pub fn from_float<T: Into<Currency>>(amount: f64, currency: T) -> Self {
        let sign = if amount < 0.0 { Sign::Negative } else { Sign::Positive };
        Self {
            amount: (amount * 10f64.powi(8)) as u64,
            sign,
            currency: currency.into()
        }
    }

    pub fn from_int<T: Into<Currency>>(amount: i64, currency: T) -> Self {
        let sign = if amount < 0 { Sign::Negative } else { Sign::Positive };
        Self {
            amount: (amount.abs() as i64 * 10i64.pow(8)) as u64,
            sign,
            currency: currency.into()
        }
    }

    pub fn to_float(&self) -> f64 {
        (self.amount as f64) / 10f64.powi(8)
    }
}

impl std::fmt::Display for Money {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let sign = if self.sign == Sign::Negative  { -1.0 } else { 1.0 };

        let amount: f64 = self.amount as f64 / 10f64.powi(8) * sign;

        write!(f, "{} {}", amount, self.currency.0)
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
        let Money { amount, sign, currency } = self.1.clone();
        let parts = self.2.parts();

        let sign = if sign == Sign::Negative  { -1 } else { 1 };

        Transaction {
            id,
            date,
            description,
            account_parts: parts.clone(),
            account_name: parts.join(":"),
            amount,
            currency: currency.into(),
            signed_amount: amount as i64 * (self.2.signed_factor(self.0.clone())) * sign,
            is_credit: self.is_credit(),
        }
    }
}
