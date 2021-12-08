use std::{fs, path::Path};

use anyhow::Result;
use chrono::prelude::*;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while_m_n},
    character::complete::{alphanumeric1, char, digit1, line_ending, space1},
    combinator::{map_res, recognize, value},
    multi::{many0, many1, separated_list1},
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

#[derive(Clone, Copy)]
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

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Open(NaiveDate, Account, Money),
    Balance(NaiveDate, Account, Money),
    Transaction(NaiveDate, String, Vec<Movement>),
}

fn date(input: &str) -> IResult<&str, NaiveDate> {
    map_res(
        recognize(tuple((digit1, char('-'), digit1, char('-'), digit1))),
        |date| NaiveDate::parse_from_str(date, "%Y-%m-%d"),
    )(input)
}

fn amount(input: &str) -> IResult<&str, f64> {
    alt((
        float,
        map_res(digit1, |out: &str| {
            anyhow::Ok(out.to_string().parse::<f64>()?.into())
        }),
    ))(input)
}

fn float(input: &str) -> IResult<&str, f64> {
    map_res(recognize(tuple((digit1, tag("."), digit1))), |out: &str| {
        anyhow::Ok(out.to_string().parse::<f64>()?.into())
    })(input)
}

fn currency(input: &str) -> IResult<&str, &str> {
    let is_valid = |c: char| c.is_alphabetic() && c.is_uppercase();

    recognize(take_while_m_n(3, 4, is_valid))(input)
}

fn money(input: &str) -> IResult<&str, Money> {
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

fn account_type(input: &str) -> IResult<&str, AccountType> {
    alt((
        value(AccountType::Assets, tag("assets")),
        value(AccountType::Liabilities, tag("liabilities")),
        value(AccountType::Income, tag("income")),
        value(AccountType::Expenses, tag("expenses")),
        value(AccountType::Equity, tag("equity")),
    ))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(many1(alt((alphanumeric1, tag("_"), tag(":")))))(input)
}

fn account(input: &str) -> IResult<&str, Account> {
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

fn single_expr(
    keyword: &'static str,
) -> impl Fn(&str) -> IResult<&str, (NaiveDate, Account, Money)> {
    move |input: &str| {
        map_res(
            tuple((
                any_spaces0,
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

fn movement_list(input: &str) -> IResult<&str, Vec<Movement>> {
    map_res(
        tuple((any_spaces0, separated_list1(any_spaces1, movement))),
        |(_, movs)| anyhow::Ok(movs),
    )(input)
}

fn movement(input: &str) -> IResult<&str, Movement> {
    let kind = |s| {
        alt((
            value(MovementKind::Credit, tag("+")),
            value(MovementKind::Debit, tag("-")),
        ))(s)
    };

    map_res(
        tuple((kind, space1, money, space1, account)),
        |(kind, _, money, _, acc)| match kind {
            MovementKind::Credit => anyhow::Ok(Movement::Credit(acc, money)),
            MovementKind::Debit => anyhow::Ok(Movement::Debit(acc, money)),
        },
    )(input)
}

fn transaction_header(input: &str) -> IResult<&str, (NaiveDate, String)> {
    map_res(
        tuple((
            date,
            space1,
            tag("transaction"),
            space1,
            alt((is_not("\r\n"), is_not("\n"))),
        )),
        |(date, _, _, _, description)| anyhow::Ok((date, description.into())),
    )(input)
}

fn expr_transaction(input: &str) -> IResult<&str, Expr> {
    map_res(
        tuple((transaction_header, line_ending, movement_list)),
        |((date, desc), _, movements)| anyhow::Ok(Expr::Transaction(date, desc, movements)),
    )(input)
}

fn expr_open(input: &str) -> IResult<&str, Expr> {
    map_res(single_expr("open"), |(date, acc, money)| {
        anyhow::Ok(Expr::Open(date, acc, money))
    })(input)
}

fn expr_balance(input: &str) -> IResult<&str, Expr> {
    map_res(single_expr("balance"), |(date, acc, money)| {
        anyhow::Ok(Expr::Balance(date, acc, money))
    })(input)
}

fn expr(input: &str) -> IResult<&str, Expr> {
    alt((expr_open, expr_balance, expr_transaction))(input)
}

fn any_spaces0(input: &str) -> IResult<&str, ()> {
    value((), many0(alt((line_ending, space1))))(input)
}

fn any_spaces1(input: &str) -> IResult<&str, ()> {
    value((), many1(alt((line_ending, space1))))(input)
}

fn file(input: &str) -> IResult<&str, Vec<Expr>> {
    map_res(
        tuple((any_spaces0, separated_list1(any_spaces1, expr), any_spaces0)),
        |(_, expr, _)| anyhow::Ok(expr),
    )(input)
}

pub fn parse_string(input: &str) -> Result<Vec<Expr>> {
    if input.is_empty() {
        return Ok(vec![]);
    }

    let (_, expr_list) = file(input).unwrap();

    Ok(expr_list)
}

pub fn parse_file<'a, P: AsRef<Path>>(path: P) -> Result<Vec<Expr>> {
    let input = fs::read_to_string(path)?;

    parse_string(&input)
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
                    transactions.clone()
                    )
            )
            );

        assert_eq!(
            expr("2020-01-02 transaction Set up initial cash account balance\n  - 300 BRL equity:initial_balance\n  + 300 BRL assets:cash_account")?,
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

    #[test]
    fn test_parse_empty_file() -> Result<()> {
        assert_eq!(parse_string("")?, vec![]);

        Ok(())
    }

    #[test]
    fn test_parse_single_transaction_file() -> Result<()> {
        assert_eq!(
            parse_string("\n 2020-01-01 open assets:cash_account 100 BRL")?,
            vec![Expr::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account::Assets("cash_account".into()),
                Money {
                    amount: 100.0,
                    currency: "BRL".into()
                }
            )]
        );

        Ok(())
    }

    #[test]
    fn test_parse_multiple_accounts_file() -> Result<()> {
        let transactions = vec![
            Expr::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account::Assets("cash_account".into()),
                Money {
                    amount: 100.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Balance(
                NaiveDate::from_ymd(2020, 1, 1),
                Account::Assets("cash_account".into()),
                Money {
                    amount: 100.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account::Expenses("stuff".into()),
                Money {
                    amount: 0.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Balance(
                NaiveDate::from_ymd(2020, 1, 1),
                Account::Expenses("stuff".into()),
                Money {
                    amount: 0.0,
                    currency: "BRL".into(),
                },
            ),
        ];

        assert_eq!(
            parse_string(
                "\n    2020-01-01 open assets:cash_account 100 BRL\n 2020-01-01 balance assets:cash_account 100 BRL\n 2020-01-01 open expenses:stuff 0 BRL\n 2020-01-01 balance expenses:stuff 0 BRL\n\n
                 "
                 )?,
                 transactions
                 );

        Ok(())
    }

    #[test]
    fn test_parse_file() -> Result<()> {
        let transactions = vec![
            Expr::Open(
                NaiveDate::from_ymd(2020, 01, 01),
                Account::Assets("cash_account".into()),
                Money {
                    amount: 100.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Balance(
                NaiveDate::from_ymd(2020, 01, 01),
                Account::Assets("cash_account".into()),
                Money {
                    amount: 100.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Open(
                NaiveDate::from_ymd(2020, 01, 01),
                Account::Expenses("stuff".into()),
                Money {
                    amount: 0.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Balance(
                NaiveDate::from_ymd(2020, 01, 01),
                Account::Expenses("stuff".into()),
                Money {
                    amount: 0.0,
                    currency: "BRL".into(),
                },
            ),
            Expr::Transaction(
                NaiveDate::from_ymd(2020, 01, 02),
                "Buy some books".into(),
                vec![
                    Movement::Debit(
                        Account::Assets("cash_account".into()),
                        Money {
                            amount: 100.0,
                            currency: "BRL".into(),
                        },
                    ),
                    Movement::Credit(
                        Account::Expenses("stuff".into()),
                        Money {
                            amount: 100.0,
                            currency: "BRL".into(),
                        },
                    ),
                ],
            ),
        ];
        assert_eq!(
            parse_string("\n 2020-01-01 open assets:cash_account 100 BRL\n 2020-01-01 balance assets:cash_account 100 BRL\n\n 2020-01-01 open expenses:stuff 0 BRL\n 2020-01-01 balance expenses:stuff 0 BRL\n\n 2020-01-02 transaction Buy some books\n - 100 BRL assets:cash_account\n + 100 BRL expenses:stuff\n "
                        )?,
                        transactions
                  );

        Ok(())
    }
}