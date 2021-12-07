use anyhow::Result;
use chrono::prelude::*;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while_m_n},
    character::{
        complete::{alphanumeric1, char, digit1, line_ending, space1},
        streaming::space0,
    },
    combinator::{map_res, recognize, value},
    multi::{many1, separated_list1},
    sequence::tuple,
    IResult,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Account {
    Assets(String),
    Liabilities(String),
    Income(String),
    Equity(String),
    Expenses(String),
}

#[derive(Clone)]
pub enum AccountType {
    Assets,
    Liabilities,
    Income,
    Equity,
    Expenses,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Money {
    amount: f64,
    currency: String,
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

#[derive(Debug, PartialEq, Clone)]
pub enum MovementKind {
    Credit,
    Debit,
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Open(NaiveDate, Account, Money),
    Balance(NaiveDate, Account, Money),
    Transaction(NaiveDate, String, Vec<Movement>),
}

fn date<'a>(input: &'a str) -> IResult<&'a str, NaiveDate> {
    map_res(
        recognize(tuple((digit1, char('-'), digit1, char('-'), digit1))),
        |date| NaiveDate::parse_from_str(date, "%Y-%m-%d"),
    )(input)
}

fn amount<'a>(input: &'a str) -> IResult<&'a str, f64> {
    alt((
        float,
        map_res(digit1, |out: &'a str| {
            anyhow::Ok(out.to_string().parse::<f64>()?.into())
        }),
    ))(input)
}

fn float<'a>(input: &'a str) -> IResult<&'a str, f64> {
    map_res(recognize(tuple((digit1, tag("."), digit1))), |out: &'a str| {
        anyhow::Ok(out.to_string().parse::<f64>()?.into())
    })(input)
}

fn currency<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
    let is_uppercase = |c: char| c.is_uppercase();
    recognize(take_while_m_n(3, 4, is_uppercase))(input)
}

fn money<'a>(input: &'a str) -> IResult<&'a str, Money> {
    map_res(
        tuple((amount, space1, currency)),
        |(amount, _, currency)| {
            anyhow::Ok(Money {
                amount,
                currency: currency.into(),
            })
        },
    )(input)
}

fn account_type<'a>(input: &'a str) -> IResult<&'a str, AccountType> {
    alt((
        value(AccountType::Assets, tag("assets")),
        value(AccountType::Liabilities, tag("liabilities")),
        value(AccountType::Income, tag("income")),
        value(AccountType::Expenses, tag("expenses")),
        value(AccountType::Equity, tag("equity")),
    ))(input)
}

fn identifier<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
    recognize(many1(alt((alphanumeric1, tag("_"), tag(":")))))(input)
}

fn account<'a>(input: &'a str) -> IResult<&'a str, Account> {
    map_res(
        tuple((account_type, tag(":"), identifier)),
        |(acc, _, id)| {
            anyhow::Ok(match acc {
                AccountType::Assets => Account::Assets(id.into()),
                AccountType::Liabilities => Account::Liabilities(id.into()),
                AccountType::Equity => Account::Equity(id.into()),
                AccountType::Expenses => Account::Expenses(id.into()),
                AccountType::Income => Account::Income(id.into()),
            })
        },
    )(input)
}

fn single_expr<'a>(
    keyword: &'static str,
) -> impl Fn(&'a str) -> IResult<&'a str, (NaiveDate, Account, Money)> {
    move |input: &'a str| {
        map_res(
            tuple((
                space0,
                date,
                space1,
                tag(keyword),
                space1,
                account,
                space1,
                money,
            )),
            |(_, date, _, _, _, acc, _, money)| anyhow::Ok((date, acc, money)),
        )(input)
    }
}

fn movement_list<'a>(input: &'a str) -> IResult<&'a str, Vec<Movement>> {
    separated_list1(line_ending, movement)(input)
}

fn movement<'a>(input: &'a str) -> IResult<&'a str, Movement> {
    let kind = |s| {
        alt((
            value(MovementKind::Credit, tag("+")),
            value(MovementKind::Debit, tag("-")),
        ))(s)
    };

    map_res(
        tuple((space0, kind, space1, money, space1, account)),
        |(_, kind, _, money, _, acc)| match kind {
            MovementKind::Credit => anyhow::Ok(Movement::Credit(acc, money)),
            MovementKind::Debit => anyhow::Ok(Movement::Debit(acc, money)),
        },
    )(input)
}

fn transaction_header<'a>(input: &'a str) -> IResult<&'a str, (NaiveDate, String)> {
    map_res(
        tuple((
            space0,
            date,
            space1,
            tag("transaction"),
            space1,
            alt((is_not("\r\n"), is_not("\n"))),
        )),
        |(_, date, _, _, _, description)| anyhow::Ok((date, description.into())),
    )(input)
}

fn expr_transaction<'a>(input: &'a str) -> IResult<&'a str, Expr> {
    map_res(
        tuple((transaction_header, line_ending, movement_list)),
        |((date, desc), _, movements)| anyhow::Ok(Expr::Transaction(date, desc, movements)),
    )(input)
}

fn expr_open<'a>(input: &'a str) -> IResult<&'a str, Expr> {
    map_res(single_expr("open"), |(date, acc, money)| {
        anyhow::Ok(Expr::Open(date, acc, money))
    })(input)
}

fn expr_balance<'a>(input: &'a str) -> IResult<&'a str, Expr> {
    map_res(single_expr("balance"), |(date, acc, money)| {
        anyhow::Ok(Expr::Balance(date, acc, money))
    })(input)
}

fn expr<'a>(input: &'a str) -> IResult<&'a str, Expr> {
    alt((expr_open, expr_balance, expr_transaction))(input)
}

fn program<'a>(input: &'a str) -> IResult<&'a str, Vec<Expr>> {
    separated_list1(many1(line_ending), expr)(input)
}

pub fn parse_program<'a>(input: &'static str) -> Result<Box<Vec<Expr>>> {
    let (_, expr_list) = program(input)?;

    Ok(Box::new(expr_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_parse_date() -> Result<()> {
        assert_eq!(date("2020-01-01")?, ("", NaiveDate::from_ymd(2020, 01, 01)));
        assert_eq!(date("1002-12-03")?, ("", NaiveDate::from_ymd(1002, 12, 03)));
        assert_eq!(date("2-12-03")?, ("", NaiveDate::from_ymd(2, 12, 03)));
        assert!(date("2-13-03").is_err());
        assert!(date("2-12-32").is_err());
        Ok(())
    }

    #[test]
    fn test_parse_currency() -> Result<()> {
        assert_eq!(currency("BRL")?, ("", "BRL"));
        assert_eq!(currency("USD")?, ("", "USD"));
        assert_eq!(currency("USDT")?, ("", "USDT"));
        assert!(currency("omg").is_err());
        assert!(currency("this is").is_err());
        //assert!(currency("COOL_STUFF").is_err());
        //assert!(currency("COOL STUFF").is_err());
        Ok(())
    }

    #[test]
    fn test_parse_money() -> Result<()> {
        assert_eq!(money("0 BRL")?, ("", Money::new(0.0, "BRL")));
        assert_eq!(money("100 BRL")?, ("", Money::new(100.0, "BRL")));
        assert_eq!(money("100.01 BRL")?, ("", Money::new(100.01, "BRL")));
        Ok(())
    }

    #[test]
    fn test_parse_open() -> Result<()> {
        assert_eq!(
            expr("2020-01-01 open income:salary 0 USD")?,
            (
                "",
                Expr::Open(
                    NaiveDate::from_ymd(2020, 01, 01),
                    Account::Income("salary".into()),
                    Money {
                        amount: 0.0,
                        currency: "USD".into()
                    }
                )
            )
        );
        Ok(())
    }

    #[test]
    fn test_parse_balance() -> Result<()> {
        assert_eq!(
            expr("2020-01-01 balance income:salary 0 USD")?,
            (
                "",
                Expr::Balance(
                    NaiveDate::from_ymd(2020, 01, 01),
                    Account::Income("salary".into()),
                    Money {
                        amount: 0.0,
                        currency: "USD".into()
                    }
                )
            )
        );
        Ok(())
    }

    #[test]
    fn test_parse_movement() -> Result<()> {
        assert_eq!(
            movement("- 300 BRL equity:initial_balance")?,
            (
                "",
                Movement::Debit(
                    Account::Equity("initial_balance".into()),
                    Money {
                        amount: 300.0,
                        currency: "BRL".into()
                    }
                )
            )
        );

        assert_eq!(
            movement("+ 300 BRL assets:cash_account")?,
            (
                "",
                Movement::Credit(
                    Account::Assets("cash_account".into()),
                    Money {
                        amount: 300.0,
                        currency: "BRL".into()
                    }
                )
            )
        );
        Ok(())
    }

    #[test]
    fn test_parse_movement_list() -> Result<()> {
        assert_eq!(
            movement_list("- 300 BRL equity:initial_balance\r\n + 300 BRL assets:cash_account")?,
            (
                "",
                vec![
                    Movement::Debit(
                        Account::Equity("initial_balance".into()),
                        Money {
                            amount: 300.0,
                            currency: "BRL".into()
                        }
                    ),
                    Movement::Credit(
                        Account::Assets("cash_account".into()),
                        Money {
                            amount: 300.0,
                            currency: "BRL".into()
                        }
                    )
                ]
            )
        );

        Ok(())
    }

    #[test]
    fn test_parse_transaction_header() -> Result<()> {
        assert_eq!(
            transaction_header("2020-01-02 transaction Set up initial cash account balance")?,
            (
                "",
                (
                    NaiveDate::from_ymd(2020, 01, 02),
                    "Set up initial cash account balance".into()
                )
            )
        );
        Ok(())
    }

    #[test]
    fn test_parse_transaction() -> Result<()> {
        let transactions = vec![
            Movement::Debit(
                Account::Equity("initial_balance".into()),
                Money {
                    amount: 300.0,
                    currency: "BRL".into(),
                },
            ),
            Movement::Credit(
                Account::Assets("cash_account".into()),
                Money {
                    amount: 300.0,
                    currency: "BRL".into(),
                },
            ),
        ];

        assert_eq!(
            expr("2020-01-02 transaction Set up initial cash account balance\r\n  - 300 BRL equity:initial_balance\r\n  + 300 BRL assets:cash_account")?,
            (
                "",
                Expr::Transaction(
                    NaiveDate::from_ymd(2020, 01, 02),
                    "Set up initial cash account balance".to_string(),
                    transactions
                )
            )
            );
        Ok(())
    }
}
