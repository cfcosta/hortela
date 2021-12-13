use std::{fs, path::Path};

use anyhow::Result;
use chrono::prelude::*;
use chumsky::prelude::*;
use num::{bigint::ToBigInt, BigRational, ToPrimitive};

use crate::{
    account::*,
    lexer::lexer,
    money::{Currency, Money},
    syntax::*,
};

fn sep(del: char) -> impl Parser<Spanned<Token>, Spanned<Token>, Error = Simple<Spanned<Token>>> {
    let expected = Token::Separator(del);

    filter_map(move |_: Span, (token, inner): Spanned<Token>| match token {
        Token::Separator(d) if d == del => Ok((expected.clone(), inner)),
        _ => Err(Simple::expected_input_found(
            inner.clone(),
            vec![(expected.clone(), inner)],
            None,
        )),
    })
}

fn date() -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    let bounded = |start: u64, end: u64| {
        let (start, end) = (
            BigRational::from_integer(start.to_bigint().unwrap()),
            BigRational::from_integer(end.to_bigint().unwrap()),
        );

        filter_map(move |span: Span, token| match token {
            (Token::Number(n), inner) if n > start && n < end => Ok((n, inner)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    };

    let (year, month, day) = (bounded(1000, 3000), bounded(1, 12), bounded(1, 31));
    year.then_ignore(sep('-'))
        .then(month)
        .then_ignore(sep('-'))
        .then(day)
        .try_map(|(((y, _), (m, _)), (d, _)), span| {
            match NaiveDate::from_ymd_opt(
                y.to_i32().unwrap() as i32,
                m.to_u32().unwrap() as u32,
                d.to_u32().unwrap() as u32,
            ) {
                Some(d) => Ok(Expr::Date(d)),
                None => Err(Simple::expected_input_found(span, vec![], None)),
            }
        })
        .map_with_span(|token, span| (token, span))
}

fn keyword<T: Into<String>>(
    keyword: T,
) -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    let val = keyword.into();

    filter_map(move |span: Span, token: Spanned<Token>| match token {
        (Token::Identifier(id), inner) if Keyword::from_str(&id).is_some() => {
            Ok((Expr::Keyword(Keyword::from_str(&id).unwrap()), inner))
        }
        (t, inner) => Err(Simple::expected_input_found(
            span,
            vec![(Token::Identifier(val.clone()), inner.clone())],
            Some((t, inner)),
        )),
    })
}

fn account() -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    let identifier = filter_map(|span: Span, token| match token {
        t @ (Token::Identifier(..), _) => Ok(t),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let separator = filter_map(|span: Span, token| match token {
        t @ (Token::Identifier(..), _) => Ok(t),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let account_types = |inner: Span| {
        vec![
            (Token::Identifier("assets".into()), inner.clone()),
            (Token::Identifier("liabilities".into()), inner.clone()),
            (Token::Identifier("income".into()), inner.clone()),
            (Token::Identifier("equity".into()), inner.clone()),
            (Token::Identifier("expenses".into()), inner),
        ]
    };

    let kind = filter_map(move |_, token: Spanned<Token>| match token {
        (Token::Identifier(id), inner) if &id == "assets" => Ok((AccountType::Assets, inner)),
        (Token::Identifier(id), inner) if &id == "liabilities" => {
            Ok((AccountType::Liabilities, inner))
        }
        (Token::Identifier(id), inner) if &id == "income" => Ok((AccountType::Income, inner)),
        (Token::Identifier(id), inner) if &id == "equity" => Ok((AccountType::Equity, inner)),
        (Token::Identifier(id), inner) if &id == "expenses" => Ok((AccountType::Expenses, inner)),
        (t, inner) => Err(Simple::expected_input_found(
            inner.clone(),
            account_types(inner.clone()),
            Some((t, inner)),
        )),
    });

    kind.then(identifier.padded_by(separator).repeated().collect())
        .try_map(
            |((kind, sk), parts): ((AccountType, Span), Vec<Spanned<Token>>), _: Span| {
                let end = parts.iter().map(|a| a.1.end()).min().unwrap();
                let parts: Vec<String> = parts
                    .into_iter()
                    .filter_map(|(token, _)| match token {
                        Token::Identifier(id) => Some(id),
                        _ => None,
                    })
                    .collect();
                let span = sk.start()..end;

                if parts.len() > 3 {
                    Err(Simple::custom(
                        span,
                        "Accounts can contain at most 4 segments",
                    ))
                } else {
                    Ok((Expr::Account(Account(kind, parts)), span))
                }
            },
        )
}

fn amount() -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    let number = filter_map(move |span: Span, token: Spanned<Token>| match token {
        (Token::Number(n), inner) => Ok((n, inner)),
        (t, inner) => Err(Simple::expected_input_found(span, vec![], Some((t, inner)))),
    });

    number
        .then(currency())
        .try_map(|((n, sn), (cur, sc)), _: Span| {
            let span = sn.start()..sc.end();

            match cur {
                Expr::Currency(cur) => Ok((Expr::Amount(n, cur), span)),
                _ => Err(Simple::expected_input_found(span, vec![], None)),
            }
        })
}

fn currency() -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    filter_map(move |span: Span, token: Spanned<Token>| match token {
        (Token::Currency(cur), inner) => Ok((Expr::Currency(cur), inner)),
        (t, inner) => Err(Simple::expected_input_found(span, vec![], Some((t, inner)))),
    })
}

fn unwrapped_date(
) -> impl Parser<Spanned<Token>, Spanned<NaiveDate>, Error = Simple<Spanned<Token>>> {
    date().try_map(|(expr, inner), _| match expr {
        Expr::Date(d) => Ok((d, inner)),
        _ => Err(Simple::expected_input_found(inner, vec![], None)),
    })
}

fn unwrapped_account(
) -> impl Parser<Spanned<Token>, Spanned<Account>, Error = Simple<Spanned<Token>>> {
    account().try_map(|(expr, inner), _| match expr {
        Expr::Account(acc) => Ok((acc, inner)),
        _ => Err(Simple::expected_input_found(inner, vec![], None)),
    })
}

fn unwrapped_currency(
) -> impl Parser<Spanned<Token>, Spanned<Currency>, Error = Simple<Spanned<Token>>> {
    currency().try_map(|(expr, inner), _| match expr {
        Expr::Currency(cur) => Ok((Currency(cur), inner)),
        _ => Err(Simple::expected_input_found(inner, vec![], None)),
    })
}

fn unwrapped_amount(
) -> impl Parser<Spanned<Token>, Spanned<(BigRational, String)>, Error = Simple<Spanned<Token>>> {
    amount().try_map(|(expr, inner), _| match expr {
        Expr::Amount(num, cur) => Ok(((num, cur), inner)),
        _ => Err(Simple::expected_input_found(inner, vec![], None)),
    })
}

pub fn parser() -> impl Parser<Spanned<Token>, Vec<Spanned<Op>>, Error = Simple<Spanned<Token>>> {
    let open = unwrapped_date()
        .then_ignore(keyword("open"))
        .then(unwrapped_account())
        .then(unwrapped_currency())
        .map(|(((date, sd), acc), (cur, cd))| {
            (
                Op::Open((date, sd.clone()), acc, (cur, cd.clone())),
                sd.start()..cd.end(),
            )
        });

    let balance = unwrapped_date()
        .then_ignore(keyword("balance"))
        .then(unwrapped_account())
        .then(unwrapped_amount())
        .map(|(((date, sd), acc), ((n, cur), cd))| {
            (
                Op::Balance((date, sd.clone()), acc, (Money::new(n, cur), cd.clone())),
                sd.start()..cd.end(),
            )
        });

    open.or(balance).repeated().collect()
}

pub fn parse_string(input: &str) -> Result<Vec<Spanned<Op>>> {
    let lexer = lexer();
    let parser = parser();

    let (tokens, errs) = lexer.parse_recovery(input);

    match tokens {
        Some((l, _)) => {
            let (parsed, parse_errs) = parser.parse_recovery(l.as_slice());

            match parsed {
                Some(ops) => Ok(ops),
                None => panic!("{:?}", parse_errs),
            }
        }
        None => panic!("{:?}", errs),
    }
}

pub fn parse_file<'a, P: AsRef<Path>>(path: P) -> Result<Vec<Spanned<Op>>> {
    let input = fs::read_to_string(path)?;

    parse_string(&input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    fn clean_up(input: Vec<Spanned<Op>>) -> Vec<CleanOp> {
        input
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<CleanOp>>()
    }

    #[test]
    fn test_parse_account() -> Result<()> {
        let parser = account();
        let tokens = vec![
            (Token::identifier("assets"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("cash_account"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("omg"), 0..1),
        ];

        assert_eq!(
            parser.parse(tokens.as_slice()).map(|x| x.0),
            Ok(Expr::Account(Account(
                AccountType::Assets,
                vec!["cash_account".into(), "omg".into()]
            ))),
        );

        Ok(())
    }

    #[test]
    fn test_parse_open() -> Result<()> {
        assert_eq!(
            clean_up(parse_string("2020-01-01 open assets:cash_account BRL")?),
            vec![CleanOp::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account(AccountType::Assets, vec!["cash_account".into()]),
                "BRL".into()
            ),]
        );

        Ok(())
    }
}
