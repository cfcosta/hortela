use chrono::NaiveDate;
use num::{BigRational, ToPrimitive};

use crate::{account::Account, ledger::Transaction, syntax::Span};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Money {
    pub amount: BigRational,
    pub currency: Currency,
}

impl Money {
    pub fn new<C: Into<Currency>>(amount: BigRational, currency: C) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }

    pub fn numer(&self) -> Option<u64> {
        self.amount.numer().to_u64()
    }

    pub fn denom(&self) -> Option<u64> {
        self.amount.denom().to_u64()
    }

    pub fn currency(&self) -> String {
        self.currency.0.clone()
    }

    pub fn equals(&self, amount: f64, precision: i32) -> bool {
        let factor = 10f64.powi(precision);
        let expected = (amount * factor).round() as u64;

        self.amount
            .to_f64()
            .map(|f| (f * factor).round() as u64 == expected)
            .unwrap_or(false)
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

#[derive(Debug, PartialEq, Clone, Eq, Hash, Copy)]
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

    pub fn to_transaction(
        self,
        id: u64,
        date: NaiveDate,
        description: String,
        span: Span,
        parent_id: Option<u64>,
    ) -> Transaction {
        Transaction {
            id,
            date,
            description,
            kind: self.0,
            account: self.2,
            amount: self.1,
            span,
            from_amount: None,
            parent_id,
        }
    }
}
