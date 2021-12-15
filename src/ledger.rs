use std::ops::{BitAnd, Not};

use chrono::{NaiveDate, NaiveDateTime};
use num::ToPrimitive;
use polars::prelude::*;

use crate::{
    account::Account,
    money::{Currency, Money, MovementKind},
    syntax::Span,
};

#[derive(Clone, Debug)]
pub struct Transaction {
    pub id: u64,
    pub date: NaiveDate,
    pub description: String,
    pub kind: MovementKind,
    pub account: Account,
    pub amount: Money,
    pub span: Span,
    pub from_amount: Option<Money>,
    pub parent_id: Option<u64>,
}

impl Transaction {
    pub fn is_credit(&self) -> bool {
        self.kind == MovementKind::Credit
    }
}

#[derive(Clone, Default)]
pub struct Transactions {
    pub id: Series,
    pub date: Series,
    pub description: Series,
    pub account_name: Series,
    pub account_name_0: Series,
    pub account_name_1: Series,
    pub account_name_2: Series,
    pub account_name_3: Series,
    pub amount_numerator: Series,
    pub amount_denominator: Series,
    pub currency: Series,
    pub amount_from_numerator: Series,
    pub amount_from_denominator: Series,
    pub currency_from: Series,
    pub signed_amount: Series,
    pub is_credit: Series,
    pub parent_id: Series,
    pub span_start: Series,
    pub span_end: Series,
}

impl Transactions {
    pub fn credits(&self) -> Result<DataFrame> {
        let df = self.all()?;
        Ok(df.filter(df.column("transaction.is_credit")?.bool()?)?)
    }

    pub fn debits(&self) -> Result<DataFrame> {
        let df = self.all()?;
        Ok(df.filter(&df.column("transaction.is_credit")?.bool()?.not())?)
    }

    pub fn transaction_type_mask(
        &self,
        df: &DataFrame,
    ) -> Result<(BooleanChunked, BooleanChunked)> {
        let column = df.column("transaction.is_credit")?.bool()?;

        Ok((column.clone(), column.not()))
    }

    pub fn all(&self) -> Result<DataFrame> {
        let data = self.clone();

        let mut amount = data
            .amount_numerator
            .cast(&DataType::Float64)?
            .divide(&data.amount_denominator.cast(&DataType::Float64)?)?;

        amount.rename("transaction.amount");

        DataFrame::new(vec![
            data.id,
            data.date,
            data.description,
            data.account_name,
            data.account_name_0,
            data.account_name_1,
            data.account_name_2,
            data.account_name_3,
            amount,
            data.amount_numerator,
            data.amount_denominator,
            data.currency,
            data.amount_from_numerator,
            data.amount_from_denominator,
            data.currency_from,
            data.signed_amount,
            data.is_credit,
            data.parent_id,
            data.span_start,
            data.span_end,
        ])
    }

    pub fn validate_balances(&self, list: Vec<BalanceVerification>) -> Result<()> {
        for verification in list {
            print!(
                "Verifying balance for {} on {}...",
                verification.account, verification.date
            );

            let df = self.all()?;

            let acc: &str = &verification.account.to_string();
            let filter_mask = df.column("transaction.account_name")?.equal(acc);

            let date_mask = df
                .column("transaction.date")?
                .date()?
                .lt_eq(date_to_arrow_datatype(verification.date));

            let filtered = df.filter(&filter_mask.bitand(date_mask))?;

            let sum = filtered
                .column("transaction.signed_amount")?
                .sum()
                .unwrap_or(0.0);

            // TODO: Make proper rounding for the numbers to avoid those kinds of hacks
            if !verification.amount.equals(sum, 2) {
                println!(" ERROR");
                panic!(
                    "Balances do not match, expected {}, got {}",
                    verification.amount, sum
                );
            }

            println!(" OK");
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BalanceVerification {
    pub id: u64,
    pub account: Account,
    pub date: NaiveDate,
    pub amount: Money,
    pub span: Span,
}

impl BalanceVerification {
    pub fn new(id: u64, account: Account, date: NaiveDate, amount: Money, span: Span) -> Self {
        Self {
            id,
            account,
            date,
            amount,
            span,
        }
    }
}

#[derive(Clone)]
pub struct BalanceVerifications {
    pub id: Series,
    pub date: Series,
    pub account_name: Series,
    pub amount_numerator: Series,
    pub amount_denominator: Series,
    pub currency: Series,
    pub span_start: Series,
    pub span_end: Series,
}

impl From<Vec<BalanceVerification>> for BalanceVerifications {
    fn from(list: Vec<BalanceVerification>) -> Self {
        let iter = list.iter();

        Self {
            id: Series::new("verification.id", iter.clone().map(|x| x.id).collect::<Vec<_>>()),
            date: DateChunked::new_from_naive_date(
                "verification.date",
                &iter.clone().map(|x| x.date).collect::<Vec<_>>(),
            )
            .into_series(),
            account_name: Series::new(
                "verification.account_name",
                iter.clone()
                    .map(|x| x.account.to_string())
                    .collect::<Vec<_>>(),
            ),
            span_start: Series::new(
                "verification.span_start",
                iter.clone()
                    .map(|x| x.span.start as u64)
                    .collect::<Vec<_>>(),
            ),
            span_end: Series::new(
                "verification.span_end",
                iter.clone().map(|x| x.span.end as u64).collect::<Vec<_>>(),
            ),
            amount_numerator: Series::new(
                "verification.amount_numerator",
                iter.clone().map(|x| x.amount.numer()).collect::<Vec<_>>(),
            ),
            amount_denominator: Series::new(
                "verification.amount_denominator",
                iter.clone().map(|x| x.amount.denom()).collect::<Vec<_>>(),
            ),
            currency: Series::new(
                "verification.currency",
                iter.clone().map(|x| x.amount.currency()).collect::<Vec<_>>(),
            ),
        }
    }
}

pub struct AccountOpening {
    pub id: u64,
    pub date: NaiveDate,
    pub account: Account,
    pub currency: Currency,
    pub span: Span,
}

impl AccountOpening {
    pub fn new(id: u64, account: Account, date: NaiveDate, currency: Currency, span: Span) -> Self {
        Self {
            id,
            account,
            date,
            currency,
            span,
        }
    }
}

#[derive(Clone)]
pub struct AccountOpenings {
    pub id: Series,
    pub date: Series,
    pub account_name: Series,
    pub currency: Series,
    pub span_start: Series,
    pub span_end: Series,
}

impl From<Vec<AccountOpening>> for AccountOpenings {
    fn from(list: Vec<AccountOpening>) -> Self {
        let iter = list.iter();

        Self {
            id: Series::new("account.id", iter.clone().map(|x| x.id).collect::<Vec<_>>()),
            date: DateChunked::new_from_naive_date(
                "account.date",
                &iter.clone().map(|x| x.date).collect::<Vec<_>>(),
            )
            .into_series(),
            account_name: Series::new(
                "account.account_name",
                iter.clone()
                    .map(|x| x.account.to_string())
                    .collect::<Vec<_>>(),
            ),
            currency: Series::new(
                "account.currency",
                iter.clone().map(|x| x.currency.0.clone()).collect::<Vec<_>>(),
            ),
            span_start: Series::new(
                "account.span_start",
                iter.clone()
                    .map(|x| x.span.start as u64)
                    .collect::<Vec<_>>(),
            ),
            span_end: Series::new(
                "account.span_end",
                iter.clone().map(|x| x.span.end as u64).collect::<Vec<_>>(),
            ),
        }
    }
}

#[derive(Clone)]
pub struct Ledger {
    pub transactions: Transactions,
    pub balance_verifications: Vec<BalanceVerification>,
    pub account_openings: AccountOpenings,
}

impl From<Vec<Transaction>> for Transactions {
    fn from(list: Vec<Transaction>) -> Self {
        let iter = list.iter();

        Self {
            id: Series::new("transaction.id", iter.clone().map(|x| x.id).collect::<Vec<_>>()),
            date: DateChunked::new_from_naive_date(
                "transaction.date",
                &iter.clone().map(|x| x.date).collect::<Vec<_>>(),
            )
            .into_series(),
            description: Series::new(
                "transaction.description",
                iter.clone()
                    .map(|x| x.description.clone())
                    .collect::<Vec<_>>(),
            ),
            account_name: Series::new(
                "transaction.account_name",
                iter.clone()
                    .map(|x| x.account.to_string())
                    .collect::<Vec<_>>(),
            ),
            account_name_0: Series::new(
                "transaction.account_name_0",
                iter.clone()
                    .map(|x| x.account.parts().get(0).cloned())
                    .collect::<Vec<_>>(),
            ),
            account_name_1: Series::new(
                "transaction.account_name_1",
                iter.clone()
                    .map(|x| x.account.parts().get(1).cloned())
                    .collect::<Vec<_>>(),
            ),
            account_name_2: Series::new(
                "transaction.account_name_2",
                iter.clone()
                    .map(|x| x.account.parts().get(2).cloned())
                    .collect::<Vec<_>>(),
            ),
            account_name_3: Series::new(
                "transaction.account_name_3",
                iter.clone()
                    .map(|x| x.account.parts().get(3).cloned())
                    .collect::<Vec<_>>(),
            ),
            amount_numerator: Series::new(
                "transaction.amount_numerator",
                iter.clone().map(|x| x.amount.numer()).collect::<Vec<_>>(),
            ),
            amount_denominator: Series::new(
                "transaction.amount_denominator",
                iter.clone().map(|x| x.amount.denom()).collect::<Vec<_>>(),
            ),
            currency: Series::new(
                "transaction.currency",
                iter.clone()
                    .map(|x| x.amount.currency())
                    .collect::<Vec<_>>(),
            ),
            amount_from_numerator: Series::new(
                "transaction.amount_from_numerator",
                iter.clone().map(|x| x.amount.numer()).collect::<Vec<_>>(),
            ),
            amount_from_denominator: Series::new(
                "transaction.amount_from_denominator",
                iter.clone().map(|x| x.amount.denom()).collect::<Vec<_>>(),
            ),
            currency_from: Series::new(
                "transaction.currency_from",
                iter.clone()
                    .map(|x| x.amount.currency())
                    .collect::<Vec<_>>(),
            ),
            is_credit: Series::new(
                "transaction.is_credit",
                iter.clone().map(|x| x.is_credit()).collect::<Vec<_>>(),
            ),
            parent_id: Series::new(
                "transaction.parent_id",
                iter.clone().map(|x| x.parent_id).collect::<Vec<_>>(),
            ),
            span_start: Series::new(
                "transaction.span_start",
                iter.clone()
                    .map(|x| x.span.start as u64)
                    .collect::<Vec<_>>(),
            ),
            span_end: Series::new(
                "transaction.span_end",
                iter.clone().map(|x| x.span.end as u64).collect::<Vec<_>>(),
            ),
            signed_amount: Series::new(
                "transaction.signed_amount",
                iter.clone()
                    .map(|x| {
                        x.amount
                            .amount
                            .to_f64()
                            .map(|m| m * x.account.signed_factor(x.kind).to_f64().unwrap())
                    })
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

fn date_to_arrow_datatype(date: NaiveDate) -> i32 {
    let unix_epoch = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
    let time = date.and_hms(0, 0, 0);

    let duration = time - NaiveDateTime::from(unix_epoch);

    duration.num_days() as i32
}
