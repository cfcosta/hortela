use chrono::{prelude::*, Duration};
use polars::prelude::*;

pub fn repeater<T: Clone>(value: T, amount: usize) -> Series
where
    Series: FromIterator<T>,
{
    Series::from_iter(std::iter::repeat(value).take(amount))
}

pub fn round_to_fixed<T: Into<f64>>(series: &Series, precision: T) -> Result<Series> {
    let repeat100 = repeater(10_f64.powf(precision.into()), series.len());

    Ok(series
        .f64()?
        .multiply(&repeat100)?
        .cast(&DataType::Int64)?
        .cast(&DataType::Float64)?
        .divide(&repeat100)?)
}

fn unix_epoch() -> NaiveDateTime {
    NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0)
}

fn date_to_arrow_datatype(date: NaiveDate) -> i32 {
    let time = date.and_hms(0, 0, 0);

    let duration = time - NaiveDateTime::from(unix_epoch());

    duration.num_days() as i32
}

fn arrow_datatype_to_date(date: i32) -> NaiveDate {
    (unix_epoch() + Duration::days(date as i64)).date()
}

fn chunked_date_range(name: &str, start: NaiveDate, end: NaiveDate) -> Result<Series> {
    Ok(DateChunked::new_from_naive_date(
        name,
        &(0..(end - (start - Duration::days(1))).num_days())
            .map(|i| start + Duration::days(i))
            .collect::<Vec<_>>(),
    )
    .into_series())
}

pub fn explode_date_series(input: &Series) -> Result<Series> {
    let min = input
        .date()?
        .min()
        .map(arrow_datatype_to_date)
        .ok_or(anyhow::anyhow!("not found min"))?;
    let max = input
        .date()?
        .max()
        .map(arrow_datatype_to_date)
        .ok_or(anyhow::anyhow!("not found max"))?;

    chunked_date_range(input.name(), min, max)
}
