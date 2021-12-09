use std::ops::Not;

use chrono::NaiveDate;
use polars::prelude::*;

use crate::validate::ALL_VALIDATORS;

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
}
