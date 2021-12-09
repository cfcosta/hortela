use std::ops::{Not, BitAnd};

use anyhow::bail;
use chrono::{NaiveDate, NaiveDateTime};
use polars::prelude::*;

use crate::{account::Account, money::Money, validate::ALL_VALIDATORS};

#[derive(Debug)]
pub struct BalanceVerification {
    pub account: Account,
    pub date: NaiveDate,
    pub expected: Money,
}

pub struct Transaction {
    pub id: u64,
    pub date: NaiveDate,
    pub description: String,
    pub account_kind: String,
    pub account_name: String,
    pub amount: f64,
    pub currency: String,
    pub signed_amount: f64,
    pub is_credit: bool,
}

#[derive(Clone)]
pub struct Ledger {
    pub id: Series,
    pub date: Series,
    pub description: Series,
    pub account_kind: Series,
    pub account_name: Series,
    pub amount: Series,
    pub currency: Series,
    pub signed_amount: Series,
    pub is_credit: Series,
}

impl From<Vec<Transaction>> for Ledger {
    fn from(list: Vec<Transaction>) -> Self {
        let iter = list.iter();

        Self {
            id: UInt64Chunked::new_from_slice(
                "ledger.id",
                &iter.clone().map(|x| x.id).collect::<Vec<_>>(),
            )
            .into_series(),
            date: DateChunked::new_from_naive_date(
                "ledger.date",
                &iter.clone().map(|x| x.date).collect::<Vec<_>>(),
            )
            .into_series(),
            description: Utf8Chunked::new_from_slice(
                "ledger.description",
                &iter
                    .clone()
                    .map(|x| x.description.clone())
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            account_name: Utf8Chunked::new_from_slice(
                "ledger.account_name",
                &iter
                    .clone()
                    .map(|x| x.account_name.clone())
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            account_kind: Utf8Chunked::new_from_slice(
                "ledger.account_kind",
                &iter
                    .clone()
                    .map(|x| x.account_kind.clone())
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            amount: Float64Chunked::new_from_slice(
                "ledger.amount",
                &iter.clone().map(|x| x.amount).collect::<Vec<_>>(),
            )
            .into_series(),
            currency: Utf8Chunked::new_from_slice(
                "ledger.currency",
                &iter.clone().map(|x| x.currency.clone()).collect::<Vec<_>>(),
            )
            .into_series(),
            signed_amount: Float64Chunked::new_from_slice(
                "ledger.signed_amount",
                &iter.clone().map(|x| x.signed_amount).collect::<Vec<_>>(),
            )
            .into_series(),
            is_credit: BooleanChunked::new_from_slice(
                "ledger.is_credit",
                &iter.clone().map(|x| x.is_credit).collect::<Vec<_>>(),
            )
            .into_series(),
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

        DataFrame::new(vec![
            data.id,
            data.date,
            data.account_kind,
            data.account_name,
            data.amount,
            data.currency,
            data.signed_amount,
            data.is_credit,
        ])
    }

    pub fn validate(&self) -> Result<()> {
        for (name, validator) in ALL_VALIDATORS {
            print!("Running validator: {}...", name);
            match validator(self) {
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
        for verifier in list {
            let df = self.all()?;

            let (name, kind) = verifier.account.parts().clone();
            let (name, kind): (&str, &str) = (&name, &kind);

            let kind_mask = df
                .column("ledger.account_kind")?
                .equal(kind);

            let name_mask = df
                .column("ledger.account_name")?
                .equal(name);
            
            let date_mask = df.column("ledger.date")?.date()?.lt_eq(date_to_arrow_datatype(verifier.date));

            let filtered = df.filter(&kind_mask.bitand(name_mask).bitand(date_mask))?;

            let sum = filtered.column("ledger.signed_amount")?.f64()?.sum().unwrap_or(0.0);

            if sum != verifier.expected.amount {
                panic!("Balances do not match, expected {}, got {}", verifier.expected.amount, sum);
            }
        }

        Ok(())
    }
}
