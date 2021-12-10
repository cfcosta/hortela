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
