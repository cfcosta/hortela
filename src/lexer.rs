use chrono::prelude::*;
use chumsky::prelude::*;

use crate::{account::*, money::*, syntax::*};

fn currency() -> impl Parser<char, Token, Error = Simple<char>> {
    filter(char::is_ascii_uppercase)
        .repeated()
        .at_least(3)
        .at_most(5)
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
            Ok(n) if n < 0 || n > 2999 => {
                Err(Simple::custom(span, format!("Year {} is not valid", n)))
            }
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        });

    let month = text::digits(10)
        .repeated()
        .at_most(2)
        .at_least(1)
        .collect::<String>()
        .try_map(|n, span| match n.parse::<u32>() {
            Ok(n) if n > 12 => Err(Simple::custom(span, format!("Month {} is not valid", n))),
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        });

    let day = text::digits(10)
        .repeated()
        .at_most(2)
        .at_least(1)
        .collect::<String>()
        .try_map(|input, span| match input.parse::<u32>() {
            Ok(n) if n > 31 => Err(Simple::expected_input_found(span, vec![], None)),
            Ok(n) => Ok(n),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        });

    year.then_ignore(just('-'))
        .then(month)
        .then_ignore(just('-'))
        .then(day)
        .try_map(
            |((year, month), day), span| match NaiveDate::from_ymd_opt(year, month, day) {
                Some(v) => Ok(Token::Date(v)),
                None => Err(Simple::expected_input_found(span, vec![], None)),
            },
        )
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
        .then(text::ident().then_ignore(just(':')).repeated().at_least(0))
        .then(text::ident())
        .try_map(|((kind, mut parts), ending), span| {
            parts.push(ending);

            if parts.len() > 3 {
                Err(Simple::custom(
                    span,
                    format!(
                        "Accounts can contain up to 4 parts, contained {}",
                        parts.len()
                    ),
                ))
            } else {
                Ok(Token::Account(kind, parts))
            }
        })
}

fn amount() -> impl Parser<char, Token, Error = Simple<char>> {
    let max_decimal_places: u32 = 8;

    let int = text::int(10).try_map(move |number: String, span| match number.parse::<u64>() {
        Ok(n) => Ok(Token::Amount(n * 10u64.pow(max_decimal_places))),
        Err(_) => Err(Simple::custom(span, "Not a valid number")),
    });

    let float = text::int(10)
        .chain::<char, _, _>(just('.').chain(text::digits(10)))
        .collect::<String>()
        .try_map(move |number, span| match number.parse::<f64>() {
            Ok(n) => Ok(Token::Amount(
                (n * 10f64.powi(max_decimal_places as i32)) as u64,
            )),
            Err(_) => Err(Simple::custom(span, "Not a valid number")),
        });

    float.or(int)
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
        .or(seq("transaction".chars()).to(Token::Keyword(Keyword::Transaction)))
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
        .padded_by(text::whitespace().ignored().or(just('\n').ignored()))
        .repeated()
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
        assert_eq!(
            parser.parse("2-1-3").unwrap(),
            Token::Date(NaiveDate::from_ymd(2, 1, 3))
        );
        assert!(parser.parse("3001-1-03").is_err());
        assert!(parser.parse("1990-13-03").is_err());
        assert!(parser.parse("1990-13-32").is_err());
        assert!(parser.parse("1799-13-32").is_err());

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
        assert_eq!(
            parser.parse("expenses:doing:something:cool").unwrap(),
            Token::Account(
                AccountType::Expenses,
                vec!["doing".into(), "something".into(), "cool".into()]
            )
        );
        assert_eq!(
            parser.parse("income:doing:something:cool").unwrap(),
            Token::Account(
                AccountType::Income,
                vec!["doing".into(), "something".into(), "cool".into()]
            )
        );
        assert_eq!(
            parser.parse("equity:doing:something:cool").unwrap(),
            Token::Account(
                AccountType::Equity,
                vec!["doing".into(), "something".into(), "cool".into()]
            )
        );
        assert!(parser
            .parse("liabilities:doing:something:cool:omg")
            .is_err());
        assert!(parser.parse("not_valid:doing:something:cool:omg").is_err());
        assert!(parser.parse("equity2:doing:something:cool:omg").is_err());

        Ok(())
    }

    #[test]
    fn test_parse_amount() -> Result<()> {
        let parser = amount();

        assert_eq!(parser.parse("1").unwrap(), Token::Amount(1 * 10u64.pow(8)));
        assert_eq!(parser.parse("2").unwrap(), Token::Amount(2 * 10u64.pow(8)));
        assert_eq!(
            parser.parse("10").unwrap(),
            Token::Amount(10 * 10u64.pow(8))
        );
        assert_eq!(
            parser.parse("19999").unwrap(),
            Token::Amount(19999 * 10u64.pow(8))
        );
        assert_eq!(
            parser.parse("102.00").unwrap(),
            Token::Amount(102 * 10u64.pow(8))
        );
        assert_eq!(
            parser.parse("123.456789").unwrap(),
            Token::Amount(12345678900)
        );

        Ok(())
    }

    #[test]
    fn test_lex_transaction() -> Result<()> {
        let parser = lexer();

        let tokens = vec![
            Token::Date(NaiveDate::from_ymd(2020, 1, 1)),
            Token::Keyword(Keyword::Transaction),
            Token::Description("hello World".into()),
            Token::Movement(MovementKind::Debit),
            Token::Amount(400 * 10u64.pow(8)),
            Token::Currency("BRL".into()),
            Token::Account(AccountType::Assets, vec!["omg".into()]),
            Token::Movement(MovementKind::Credit),
            Token::Amount(300 * 10u64.pow(8)),
            Token::Currency("BRL".into()),
            Token::Account(AccountType::Equity, vec!["omg".into()]),
        ];

        assert_eq!(
            parser
                .parse(
                    "2020-01-01 transaction \"hello World\"
                < 400 BRL assets:omg
                > 300 BRL equity:omg"
                )
                .unwrap()
                .into_iter()
                .map(|(t, _)| t)
                .collect::<Vec<Token>>(),
            tokens
        );

        Ok(())
    }
}
