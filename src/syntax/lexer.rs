use chumsky::prelude::*;

use crate::{money::*, syntax::*};

fn separator() -> impl Parser<char, Token, Error = Simple<char>> {
    one_of(":-".chars()).map(|c| Token::Separator(c))
}

fn number() -> impl Parser<char, Token, Error = Simple<char>> {
    let num = text::digits(10)
        .chain::<char, _, _>(just('.').chain(text::digits(10)).or_not().flatten())
        .collect::<String>();

    num.try_map(move |number: String, span| match number.parse::<f64>() {
        Ok(n) => Ok(Token::number(n)),
        _ => Err(Simple::custom(span, "Not a valid number")),
    })
    .labelled("number")
}

fn movement() -> impl Parser<char, Token, Error = Simple<char>> {
    just('<')
        .to(Token::Movement(MovementKind::Debit))
        .or(just('>').to(Token::Movement(MovementKind::Credit)))
        .labelled("movement kind")
}

fn string() -> impl Parser<char, Token, Error = Simple<char>> {
    just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .map(Token::String)
}

fn identifier() -> impl Parser<char, Token, Error = Simple<char>> {
    filter(char::is_ascii_lowercase)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| char::is_ascii_lowercase(c) || *c == '_').repeated(),
        )
        .collect::<String>()
        .map(Token::identifier)
}

fn currency() -> impl Parser<char, Token, Error = Simple<char>> {
    filter(char::is_ascii_uppercase)
        .chain(
            filter(char::is_ascii_uppercase)
                .repeated()
                .at_least(2)
                .at_most(4),
        )
        .collect::<String>()
        .map(Token::currency)
}

pub fn lexer() -> impl Parser<char, Spanned<Vec<Spanned<Token>>>, Error = Simple<char>> {
    let token = currency()
        .or(movement())
        .or(string())
        .or(separator())
        .or(number())
        .or(identifier())
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
        .padded_by(text::whitespace().ignored().or(just('\n').ignored()))
        .map_with_span(|tok, span| (tok, span))
        .repeated()
        .map_with_span(|tok, span| (tok, span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    fn clean_up(input: Spanned<Vec<Spanned<Token>>>) -> Vec<Token> {
        input.0.into_iter().map(|x| x.0).collect()
    }

    #[test]
    fn test_lexer_date() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(parser.parse("2020-01-01").unwrap()),
            vec![
                Token::number(2020.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::Separator('-'),
                Token::number(1.0)
            ]
        );

        Ok(())
    }

    #[test]
    fn test_lexer_number() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(parser.parse("200").unwrap()),
            vec![Token::number(200.0),]
        );

        assert_eq!(
            clean_up(parser.parse("200.00").unwrap()),
            vec![Token::number(200.0),]
        );

        assert_eq!(
            clean_up(parser.parse("00200.00").unwrap()),
            vec![Token::number(200.0),]
        );

        Ok(())
    }

    #[test]
    fn test_lexer_account() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(parser.parse("assets:lol:wth:bbq").unwrap()),
            vec![
                Token::identifier("assets"),
                Token::Separator(':'),
                Token::identifier("lol"),
                Token::Separator(':'),
                Token::identifier("wth"),
                Token::Separator(':'),
                Token::identifier("bbq"),
            ]
        );

        Ok(())
    }

    #[test]
    fn test_lexer_amount() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(parser.parse("200 BRL").unwrap()),
            vec![Token::number(200.0), Token::currency("BRL"),]
        );

        assert_eq!(
            clean_up(parser.parse("200.0 USD").unwrap()),
            vec![Token::number(200.0), Token::currency("USD"),]
        );

        assert_eq!(
            clean_up(parser.parse("200.01 USD").unwrap()),
            vec![Token::number(200.01), Token::currency("USD"),]
        );

        Ok(())
    }

    #[test]
    fn test_lexer_currency() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(parser.parse("BRL").unwrap()),
            vec![Token::currency("BRL")],
        );

        assert_eq!(
            clean_up(parser.parse("USD").unwrap()),
            vec![Token::currency("USD")],
        );

        Ok(())
    }

    #[test]
    fn test_lexer_open_op() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(
                parser
                    .parse("2020-01-01 open assets:cash_account BRL")
                    .unwrap()
            ),
            vec![
                Token::number(2020.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::identifier("open"),
                Token::identifier("assets"),
                Token::Separator(':'),
                Token::identifier("cash_account"),
                Token::currency("BRL"),
            ]
        );

        Ok(())
    }

    #[test]
    fn test_lexer_balance_op() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(
                parser
                    .parse("2020-01-01 balance assets:cash_account 200.01 BRL")
                    .unwrap()
            ),
            vec![
                Token::number(2020.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::identifier("balance"),
                Token::identifier("assets"),
                Token::Separator(':'),
                Token::identifier("cash_account"),
                Token::number(200.01),
                Token::currency("BRL"),
            ]
        );

        Ok(())
    }

    #[test]
    fn test_lexer_multiple_ops() -> Result<()> {
        let parser = lexer();

        assert_eq!(
            clean_up(
                parser
                    .parse("2020-01-01 open assets:cash_account BRL\n2020-01-01 balance assets:cash_account 200.01 BRL")
                    .unwrap()
            ),
            vec![
                Token::number(2020.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::identifier("open"),
                Token::identifier("assets"),
                Token::Separator(':'),
                Token::identifier("cash_account"),
                Token::currency("BRL"),
                Token::number(2020.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::Separator('-'),
                Token::number(1.0),
                Token::identifier("balance"),
                Token::identifier("assets"),
                Token::Separator(':'),
                Token::identifier("cash_account"),
                Token::number(200.01),
                Token::currency("BRL"),
            ]
        );

        Ok(())
    }
}
