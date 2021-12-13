use std::ops::{BitAnd, Not};

use chrono::{NaiveDate, NaiveDateTime};
use polars::prelude::*;

use crate::{money::Money, validate::ALL_VALIDATORS, BalanceVerification};

pub struct Transaction {
    pub id: u64,
    pub date: NaiveDate,
    pub description: String,
    pub account_name: String,
    pub account_parts: Vec<String>,
    pub amount: u64,
    pub currency: String,
    pub signed_amount: i64,
    pub is_credit: bool,
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
                    .map(|x| x.account_parts.join(":"))
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            account_name_0: Utf8Chunked::new_from_slice(
                "ledger.account_name_0",
                &iter
                    .clone()
                    .map(|x| x.account_parts.get(0).cloned().unwrap_or("".into()))
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            account_name_1: Utf8Chunked::new_from_slice(
                "ledger.account_name_1",
                &iter
                    .clone()
                    .map(|x| x.account_parts.get(1).cloned().unwrap_or("".into()))
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            account_name_2: Utf8Chunked::new_from_slice(
                "ledger.account_name_2",
                &iter
                    .clone()
                    .map(|x| x.account_parts.get(2).cloned().unwrap_or("".into()))
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            account_name_3: Utf8Chunked::new_from_slice(
                "ledger.account_name_3",
                &iter
                    .clone()
                    .map(|x| x.account_parts.get(3).cloned().unwrap_or("".into()))
                    .collect::<Vec<_>>(),
            )
            .into_series(),
            amount: UInt64Chunked::new_from_slice(
                "ledger.amount",
                &iter.clone().map(|x| x.amount).collect::<Vec<_>>(),
            )
            .into_series(),
            currency: Utf8Chunked::new_from_slice(
                "ledger.currency",
                &iter.clone().map(|x| x.currency.clone()).collect::<Vec<_>>(),
            )
            .into_series(),
            signed_amount: Int64Chunked::new_from_slice(
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
            data.account_name,
            data.account_name_0,
            data.account_name_1,
            data.account_name_2,
            data.account_name_3,
            data.amount,
            data.currency,
            data.signed_amount,
            data.is_credit,
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

            let sum = Money::from_int(
                filtered
                    .column("ledger.signed_amount")?
                    .i64()?
                    .sum()
                    .unwrap_or(0),
                verification.clone().amount.currency,
            );

            // TODO: Make proper rounding for the numbers to avoid those kinds of hacks
            if sum != verification.amount {
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
