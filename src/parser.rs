use anyhow::Result;
use chrono::prelude::*;
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccountType {
    Assets,
    Liabilities,
    Income,
    Equity,
    Expenses,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Currency(String);

impl From<String> for Currency {
    fn from(val: String) -> Self {
        Self(val.to_string())
    }
}

impl From<&str> for Currency {
    fn from(val: &str) -> Self {
        Self(val.to_string())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Account(AccountType, Vec<String>);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Money(i64, Currency);

type Spanned<T> = (T, Span);

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Movement(MovementKind, Money, Account);

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Open(NaiveDate, Account, Currency),
    Balance(NaiveDate, Account, Money),
    Transaction(NaiveDate, String, Vec<Movement>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MovementKind {
    Credit,
    Debit,
}

type Span = std::ops::Range<usize>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Keyword {
    Open,
    Balance,
    Transaction,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Comment(String),
    Date(NaiveDate),
    Amount(i64),
    Description(String),
    Currency(Currency),
    Keyword(Keyword),
    Account(AccountType, Vec<String>),
    Movement(MovementKind),
}

impl Token {
    pub fn amount(&self) -> Option<i64> {
        match self {
            Self::Amount(a) => Some(*a),
            _ => None,
        }
    }

    pub fn currency(&self) -> Option<Currency> {
        match self {
            Self::Currency(c) => Some(c.clone()),
            _ => None,
        }
    }
}

fn currency() -> impl Parser<char, Token, Error = Simple<char>> {
    filter(char::is_ascii_uppercase)
        .repeated()
        .at_least(3)
        .at_most(4)
        .collect::<String>()
        .map(|x: String| x.into())
        .map(Token::Currency)
}

fn date() -> impl Parser<char, Token, Error = Simple<char>> {
    let year = text::digits(10)
        .repeated()
        .at_most(4)
        .collect::<String>()
        .try_map(|n, span| match n.parse::<i32>() {
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        });

    let month_or_day = text::digits(10)
        .repeated()
        .at_most(2)
        .at_least(1)
        .collect::<String>()
        .try_map(|n, span| match n.parse::<u32>() {
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        });

    year.then_ignore(just('-'))
        .then(month_or_day)
        .then_ignore(just('-'))
        .then(month_or_day)
        .try_map(|((year, month), day), span| {
            if year < 1900 || year > 2999 {
                return Err(Simple::custom(span, format!("Year {} is not valid", year)));
            }

            if month > 12 {
                return Err(Simple::custom(
                    span,
                    format!("Month {} is not valid", month),
                ));
            }

            if day > 31 {
                return Err(Simple::custom(span, format!("Day {} is not valid", day)));
            }

            Ok(Token::Date(NaiveDate::from_ymd(year, month, day)))
        })
}

fn account() -> impl Parser<char, Token, Error = Simple<char>> {
    let keywords = seq("assets".chars())
        .to(AccountType::Assets)
        .or(seq("liabilities".chars()).to(AccountType::Liabilities))
        .or(seq("equity".chars()).to(AccountType::Equity))
        .or(seq("income".chars()).to(AccountType::Income))
        .or(seq("expenses".chars()).to(AccountType::Expenses));

    keywords
        .then_ignore(just(':'))
        .then(
            text::ident()
                .then_ignore(just(':'))
                .repeated()
                .at_least(0)
                .at_most(2),
        )
        .then(text::ident())
        .map(|((kind, mut parts), ending)| {
            parts.push(ending);

            Token::Account(kind, parts)
        })
}

fn amount() -> impl Parser<char, Token, Error = Simple<char>> {
    text::int(10)
        .chain::<char, _, _>(just('.').chain(text::digits(10)))
        .collect::<String>()
        .try_map(|number, span| match number.parse() {
            Ok(n) => Ok(Token::Amount(n)),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        })
}

fn movement() -> impl Parser<char, Token, Error = Simple<char>> {
    just('<')
        .to(Token::Movement(MovementKind::Debit))
        .or(just('>').to(Token::Movement(MovementKind::Credit)))
}

fn keyword() -> impl Parser<char, Token, Error = Simple<char>> {
    seq("open".chars())
        .to(Token::Keyword(Keyword::Open))
        .or(seq("balance".chars()).to(Token::Keyword(Keyword::Balance)))
        .or(seq("transaction".chars()).to(Token::Keyword(Keyword::Balance)))
}

pub fn description() -> impl Parser<char, Token, Error = Simple<char>> {
    just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .map(Token::Description)
}

pub fn lexer() -> impl Parser<char, Vec<Spanned<Token>>, Error = Simple<char>> {
    let token = date()
        .or(movement())
        .or(account())
        .or(amount())
        .or(keyword())
        .or(currency())
        .or(description())
        .recover_with(skip_then_retry_until([]));

    let until_eol = take_until(just('\n'))
        .map(|(text, _)| text)
        .collect::<String>();
    let comment = seq("//".chars())
        .then(until_eol)
        .padded()
        .map(|(_, text)| Token::Comment(text));

    token
        .padded_by(comment.repeated())
        .map_with_span(|tok, span| (tok, span))
        .padded()
        .repeated()
}

pub fn parser() -> impl Parser<Token, Vec<Spanned<Expr>>, Error = Simple<Token>> + Clone {
    let date_parser = filter_map(|span, token| match token {
        Token::Date(d) => Ok(d),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let keyword = |keyword: Keyword| {
        filter_map(move |span, token| match token {
            Token::Keyword(k) if k == keyword => Ok(k),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    };

    let account = filter_map(|span, token| match token {
        Token::Account(acc, parts) => Ok(Account(acc, parts)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let amount = filter_map(|span, token| match token {
        Token::Amount(a) => Ok(a),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let currency = filter_map(|span, token| match token {
        Token::Currency(c) => Ok(c),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let description = filter_map(|span, token| match token {
        Token::Description(d) => Ok(d),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let movement = filter_map(|span, token| match token {
        Token::Movement(kind) => Ok(kind),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let money_parser = amount
        .then(currency)
        .map(|(amount, currency)| Money(amount, currency));

    let movement_expr = movement
        .then(money_parser)
        .then(account)
        .map_with_span(|((mov, money), acc), span| (Movement(mov, money, acc), span));

    let open_expr = date_parser
        .then(keyword(Keyword::Open))
        .then(account)
        .then(currency)
        .map_with_span(|(((date, _), acc), cur), span| (Expr::Open(date, acc, cur), span));

    let balance_expr = date_parser
        .then(keyword(Keyword::Balance))
        .then(account)
        .then(money_parser)
        .map_with_span(|(((date, _), acc), cur), span| (Expr::Balance(date, acc, cur), span));

    let transaction_expr = date_parser
        .then(keyword(Keyword::Transaction))
        .then(description)
        .then(movement_expr.repeated())
        .map_with_span(|(((date, _), desc), movs), span| {
            (
                Expr::Transaction(date, desc, movs.into_iter().map(|x| x.0).collect()),
                span,
            )
        });

    open_expr
        .or(balance_expr)
        .or(transaction_expr)
        .repeated()
        .then_ignore(end())
}

pub fn parse_string(input: &str) -> Result<Vec<Spanned<Expr>>> {
    let lexer = lexer();
    let parser = parser();

    let (tokens, errs) = lexer.parse_recovery(input);

    match tokens {
        Some(l) => {
            let list: Vec<Token> = l.into_iter().map(|(t, _)| t).collect();

            match parser.parse(list.as_slice()) {
                Ok(l) => Ok(l),
                Err(_) => panic!("Failed to parse"),
            }
        }
        None => panic!("none"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_parse_date() -> Result<()> {
        let parser = date();

        assert_eq!(
            parser.parse("2020-01-01").unwrap(),
            Token::Date(NaiveDate::from_ymd(2020, 1, 1))
        );
        assert_eq!(
            parser.parse("1990-12-03").unwrap(),
            Token::Date(NaiveDate::from_ymd(1990, 12, 03))
        );
        assert_eq!(
            parser.parse("2020-12-03").unwrap(),
            Token::Date(NaiveDate::from_ymd(2020, 12, 03))
        );
        assert!(parser.parse("2-1-03").is_err());
        assert!(parser.parse("3001-1-03").is_err());
        assert!(parser.parse("1990-13-03").is_err());
        assert!(parser.parse("1990-13-32").is_err());

        Ok(())
    }

    #[test]
    fn test_parse_account() -> Result<()> {
        let parser = account();

        assert_eq!(
            parser.parse("assets:cash").unwrap(),
            Token::Account(AccountType::Assets, vec!["cash".into()])
        );
        assert_eq!(
            parser.parse("assets:cash:omg").unwrap(),
            Token::Account(AccountType::Assets, vec!["cash".into(), "omg".into()])
        );
        assert_eq!(
            parser.parse("assets:cash:omg_cool").unwrap(),
            Token::Account(AccountType::Assets, vec!["cash".into(), "omg_cool".into()])
        );
        assert_eq!(
            parser.parse("liabilities:doing:something:cool").unwrap(),
            Token::Account(
                AccountType::Liabilities,
                vec!["doing".into(), "something".into(), "cool".into()]
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
            parse_string("\n 2020-01-01 open assets:cash_account BRL")?,
            vec![(
                Expr::Open(
                    NaiveDate::from_ymd(2020, 1, 1),
                    Account(AccountType::Assets, vec!["cash_account".into()]),
                    "BRL".into()
                ),
                0..4
            )]
        );

        Ok(())
    }

    #[test]
    fn test_parse_simple_transaction_file() -> Result<()> {
        let transactions = Vec::from([
            (
                Expr::Open(
                    NaiveDate::from_ymd(2020, 1, 1),
                    Account(AccountType::Assets, vec!["cash_account".into()]),
                    "BRL".into(),
                ),
                0..4,
            ),
            (
                Expr::Open(
                    NaiveDate::from_ymd(2020, 1, 2),
                    Account(AccountType::Liabilities, vec!["credit_card".into()]),
                    "BRL".into(),
                ),
                4..8,
            ),
        ]);

        assert_eq!(
            parse_string("\n 2020-01-01 open assets:cash_account BRL\n 2020-01-02 open liabilities:credit_card BRL")?,
           transactions 
        );

        Ok(())
    }
    //
    //     #[test]
    //     fn test_parse_multiple_accounts_file() -> Result<()> {
    //         let transactions = vec![
    //             Expr::Open(
    //                 NaiveDate::from_ymd(2020, 1, 1),
    //                 Account::Assets("cash_account".into()),
    //                 "BRL".into(),
    //             ),
    //             Expr::Balance(
    //                 NaiveDate::from_ymd(2020, 1, 1),
    //                 Account::Assets("cash_account".into()),
    //                 Money {
    //                     amount: 100.0,
    //                     currency: "BRL".into(),
    //                 },
    //             ),
    //             Expr::Open(
    //                 NaiveDate::from_ymd(2020, 1, 1),
    //                 Account::Expenses("stuff".into()),
    //                 "BRL".into(),
    //             ),
    //             Expr::Balance(
    //                 NaiveDate::from_ymd(2020, 1, 1),
    //                 Account::Expenses("stuff".into()),
    //                 Money {
    //                     amount: 0.0,
    //                     currency: "BRL".into(),
    //                 },
    //             ),
    //         ];
    //
    //         assert_eq!(
    //             parse_string(
    //                 "\n    2020-01-01 open assets:cash_account BRL\n 2020-01-01 balance assets:cash_account 100 BRL\n 2020-01-01 open expenses:stuff BRL\n 2020-01-01 balance expenses:stuff 0 BRL\n\n
    //                  "
    //                  )?,
    //                  transactions
    //                  );
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     fn test_parse_file() -> Result<()> {
    //         let transactions = vec![
    //             Expr::Open(
    //                 NaiveDate::from_ymd(2020, 01, 01),
    //                 Account::Assets("cash_account".into()),
    //                 "BRL".into(),
    //             ),
    //             Expr::Balance(
    //                 NaiveDate::from_ymd(2020, 01, 01),
    //                 Account::Assets("cash_account".into()),
    //                 Money {
    //                     amount: 100.0,
    //                     currency: "BRL".into(),
    //                 },
    //             ),
    //             Expr::Open(
    //                 NaiveDate::from_ymd(2020, 01, 01),
    //                 Account::Expenses("stuff".into()),
    //                 "BRL".into(),
    //             ),
    //             Expr::Balance(
    //                 NaiveDate::from_ymd(2020, 01, 01),
    //                 Account::Expenses("stuff".into()),
    //                 Money {
    //                     amount: 0.0,
    //                     currency: "BRL".into(),
    //                 },
    //             ),
    //             Expr::Transaction(
    //                 NaiveDate::from_ymd(2020, 01, 02),
    //                 "Buy some books".into(),
    //                 vec![
    //                     Movement::Debit(
    //                         Account::Assets("cash_account".into()),
    //                         Money {
    //                             amount: 100.0,
    //                             currency: "BRL".into(),
    //                         },
    //                     ),
    //                     Movement::Credit(
    //                         Account::Expenses("stuff".into()),
    //                         Money {
    //                             amount: 100.0,
    //                             currency: "BRL".into(),
    //                         },
    //                     ),
    //                 ],
    //             ),
    //         ];
    //
    //         assert_eq!(
    //             parse_string("\n 2020-01-01 open assets:cash_account BRL\n 2020-01-01 balance assets:cash_account 100 BRL\n\n 2020-01-01 open expenses:stuff BRL\n 2020-01-01 balance expenses:stuff 0 BRL\n\n 2020-01-02 transaction Buy some books\n < 100 BRL assets:cash_account\n > 100 BRL expenses:stuff\n "
    //                         )?,
    //                         transactions
    //                   );
    //
    //         Ok(())
    //     }
}
