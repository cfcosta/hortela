use std::ops::{BitAnd, Not};

use chrono::{NaiveDate, NaiveDateTime};
use num::{BigRational, ToPrimitive};
use polars::prelude::*;

use crate::{
    account::Account,
    money::{Money, MovementKind},
    syntax::Span,
    validate::ALL_VALIDATORS,
    BalanceVerification,
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

#[derive(Clone)]
pub struct Ledger {
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

impl From<Vec<Transaction>> for Ledger {
    fn from(list: Vec<Transaction>) -> Self {
        let iter = list.iter();

        Self {
            id: Series::new("ledger.id", iter.clone().map(|x| x.id).collect::<Vec<_>>()),
            date: DateChunked::new_from_naive_date(
                "ledger.date",
                &iter.clone().map(|x| x.date).collect::<Vec<_>>(),
            )
            .into_series(),
            description: Series::new(
                "ledger.description",
                iter.clone()
                    .map(|x| x.description.clone())
                    .collect::<Vec<_>>(),
            ),
            account_name: Series::new(
                "ledger.account_name",
                iter.clone()
                    .map(|x| x.account.to_string())
                    .collect::<Vec<_>>(),
            ),
            account_name_0: Series::new(
                "ledger.account_name_0",
                iter.clone()
                    .map(|x| x.account.parts().get(0).cloned())
                    .collect::<Vec<_>>(),
            ),
            account_name_1: Series::new(
                "ledger.account_name_1",
                iter.clone()
                    .map(|x| x.account.parts().get(1).cloned())
                    .collect::<Vec<_>>(),
            ),
            account_name_2: Series::new(
                "ledger.account_name_2",
                iter.clone()
                    .map(|x| x.account.parts().get(2).cloned())
                    .collect::<Vec<_>>(),
            ),
            account_name_3: Series::new(
                "ledger.account_name_3",
                iter.clone()
                    .map(|x| x.account.parts().get(3).cloned())
                    .collect::<Vec<_>>(),
            ),
            amount_numerator: Series::new(
                "ledger.amount_numerator",
                iter.clone().map(|x| x.amount.numer()).collect::<Vec<_>>(),
            ),
            amount_denominator: Series::new(
                "ledger.amount_denominator",
                iter.clone().map(|x| x.amount.denom()).collect::<Vec<_>>(),
            ),
            currency: Series::new(
                "ledger.currency",
                iter.clone()
                    .map(|x| x.amount.currency())
                    .collect::<Vec<_>>(),
            ),
            amount_from_numerator: Series::new(
                "ledger.amount_from_numerator",
                iter.clone().map(|x| x.amount.numer()).collect::<Vec<_>>(),
            ),
            amount_from_denominator: Series::new(
                "ledger.amount_from_denominator",
                iter.clone().map(|x| x.amount.denom()).collect::<Vec<_>>(),
            ),
            currency_from: Series::new(
                "ledger.currency_from",
                iter.clone()
                    .map(|x| x.amount.currency())
                    .collect::<Vec<_>>(),
            ),
            is_credit: Series::new(
                "ledger.is_credit",
                iter.clone().map(|x| x.is_credit()).collect::<Vec<_>>(),
            ),
            parent_id: Series::new(
                "ledger.parent_id",
                iter.clone().map(|x| x.parent_id).collect::<Vec<_>>(),
            ),
            span_start: Series::new(
                "ledger.span_start",
                iter.clone()
                    .map(|x| x.span.start as u64)
                    .collect::<Vec<_>>(),
            ),
            span_end: Series::new(
                "ledger.span_end",
                iter.clone().map(|x| x.span.end as u64).collect::<Vec<_>>(),
            ),
            signed_amount: Series::new(
                "ledger.signed_amount",
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

impl Ledger {
    pub fn credits(&self) -> Result<DataFrame> {
        let df = self.all()?;
        Ok(df.filter(df.column("ledger.is_credit")?.bool()?)?)
    }

    pub fn debits(&self) -> Result<DataFrame> {
        let df = self.all()?;
        Ok(df.filter(&df.column("ledger.is_credit")?.bool()?.not())?)
    }

    pub fn all(&self) -> Result<DataFrame> {
        let data = self.clone();

        let mut amount = data
            .amount_numerator
            .cast(&DataType::Float64)?
            .divide(&data.amount_denominator.cast(&DataType::Float64)?)?;

        amount.rename("ledger.amount");

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

    pub fn validate(&self) -> Result<()> {
        for (name, validator) in ALL_VALIDATORS {
            print!("Running validator: {}...", name);
            match validator(&self.clone()) {
                Ok(_) => {
                    println!(" OK");
                }
                e => {
                    println!(" ERROR");
                    e?
                }
            }
        }

        Ok(())
    }

    pub fn validate_balances(&self, list: Vec<BalanceVerification>) -> Result<()> {
        for verification in list {
            print!(
                "Verifying balance for {} on {}...",
                verification.account, verification.date
            );

            let df = self.all()?;

            let acc: &str = &verification.account.to_string();
            let filter_mask = df.column("ledger.account_name")?.equal(acc);

            let date_mask = df
                .column("ledger.date")?
                .date()?
                .lt_eq(date_to_arrow_datatype(verification.date));

            let filtered = df.filter(&filter_mask.bitand(date_mask))?;

            let sum = filtered
                .column("ledger.signed_amount")?
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
