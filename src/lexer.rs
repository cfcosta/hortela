use chrono::prelude::*;
use chumsky::prelude::*;

use crate::{ account::*, money::*, syntax::* };

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
        .padded()
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
}
