use std::{fs, path::Path};

use anyhow::Result;
use chrono::prelude::*;
use chumsky::prelude::*;
use num::{BigRational, ToPrimitive};

use crate::{
    account::*,
    lexer::lexer,
    money::{Movement, MovementKind},
    syntax::*,
};

fn sep(del: char) -> impl Parser<Spanned<Token>, Spanned<Token>, Error = Simple<Spanned<Token>>> {
    let expected = Token::Separator(del);

    filter_map(move |_: Span, (token, inner): Spanned<Token>| match token {
        Token::Separator(d) if d == del => Ok((token, inner)),
        _ => Err(Simple::expected_input_found(
            inner.clone(),
            vec![(expected.clone(), inner)],
            None,
        )),
    })
}

fn movement_kind() -> impl Parser<Spanned<Token>, Spanned<Token>, Error = Simple<Spanned<Token>>> {
    filter_map(move |_: Span, (token, inner): Spanned<Token>| match token {
        Token::Movement(_) => Ok((token, inner)),
        _ => Err(Simple::expected_input_found(
            inner.clone(),
            vec![
                (Token::Movement(MovementKind::Credit), inner.clone()),
                (Token::Movement(MovementKind::Debit), inner),
            ],
            None,
        )),
    })
}

fn string() -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    filter_map(|_: Span, (token, inner): Spanned<Token>| {
        token
            .get_string()
            .map(|t| (Expr::Description(t), inner.clone()))
            .ok_or(Simple::expected_input_found(inner, vec![], None))
    })
}

fn bounded_number(
    start: u64,
    end: u64,
) -> impl Parser<Spanned<Token>, Spanned<BigRational>, Error = Simple<Spanned<Token>>> {
    let (start, end) = (
        BigRational::from_integer(start.into()),
        BigRational::from_integer(end.into()),
    );

    filter_map(
        move |_: Span, (token, inner): Spanned<Token>| match token.get_number() {
            Some(n) if n >= start && n <= end => Ok((n, inner)),
            Some(n) => Ok((n, inner)),
            None => Err(Simple::expected_input_found(
                inner.clone(),
                vec![],
                Some((token, inner)),
            )),
        },
    )
}

fn try_to_date(
    span: Span,
    y: BigRational,
    m: BigRational,
    d: BigRational,
) -> Result<NaiveDate, Simple<Spanned<Token>>> {
    NaiveDate::from_ymd_opt(
        y.to_i32()
            .ok_or(Simple::expected_input_found(span.clone(), vec![], None))?,
        m.to_u32()
            .ok_or(Simple::expected_input_found(span.clone(), vec![], None))?,
        d.to_u32()
            .ok_or(Simple::expected_input_found(span.clone(), vec![], None))?,
    )
    .ok_or(Simple::expected_input_found(span, vec![], None))
}

fn date() -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    let (year, month, day) = (
        bounded_number(1000, 3000),
        bounded_number(1, 12),
        bounded_number(1, 31),
    );

    year.then_ignore(sep('-'))
        .then(month)
        .then_ignore(sep('-'))
        .then(day)
        .try_map(|(((y, sy), (m, _)), (d, sd)), _| {
            let span = sy.start()..sd.end();
            Ok((Expr::Date(try_to_date(span.clone(), y, m, d)?), span))
        })
}

fn keyword<T: Into<String>>(
    keyword: T,
) -> impl Parser<Spanned<Token>, Spanned<Expr>, Error = Simple<Spanned<Token>>> {
    let val = keyword.into();

    filter_map(move |span: Span, token: Spanned<Token>| match token {
        (Token::Identifier(id), inner) if Keyword::from_str(&id).is_some() => Ok((
            Expr::Keyword(Keyword::from_str(&id).expect("Failed to get keyword")),
            inner,
        )),
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
        t @ (Token::Separator(..), _) => Ok(t),
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
        )
        .with_label("account type")),
    });

    kind.then(identifier.repeated().separated_by(separator).flatten())
        .try_map(
            |((kind, sk), parts): ((AccountType, Span), Vec<Spanned<Token>>), _: Span| {
                let end = parts
                    .last()
                    .map(|a| a.1.end())
                    .expect("Failed to get span for last part of account");

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

fn movement() -> impl Parser<Spanned<Token>, Spanned<Movement>, Error = Simple<Spanned<Token>>> {
    movement_kind()
        .then(amount())
        .then(account())
        .map(|(((kind, sk), (amount, _)), (acc, sa))| {
            (
                Movement(
                    kind.get_movement_kind().unwrap(),
                    amount.get_money().unwrap(),
                    acc.get_account().unwrap(),
                ),
                sk.start()..sa.end(),
            )
        })
        .labelled("movement")
}

fn movements(
) -> impl Parser<Spanned<Token>, Spanned<Vec<Spanned<Movement>>>, Error = Simple<Spanned<Token>>> {
    movement()
        .repeated()
        .collect::<Vec<_>>()
        .map(|movs| {
            let start = movs.first().cloned().map(|x| x.1.start()).unwrap();
            let end = movs.last().cloned().map(|x| x.1.end()).unwrap();

            (movs, start..end)
        })
        .labelled("movements")
}

fn open_op() -> impl Parser<Spanned<Token>, Spanned<Op>, Error = Simple<Spanned<Token>>> {
    date()
        .then_ignore(keyword("open"))
        .then(account())
        .then(currency())
        .map(|(((date, sd), (acc, sa)), (cur, sc))| {
            (
                Op::Open(
                    (date.get_date().unwrap(), sd.clone()),
                    (acc.get_account().unwrap(), sa),
                    (cur.get_currency().unwrap().into(), sc.clone()),
                ),
                sd.start()..sc.end(),
            )
        })
}

fn balance_op() -> impl Parser<Spanned<Token>, Spanned<Op>, Error = Simple<Spanned<Token>>> {
    date()
        .then_ignore(keyword("balance"))
        .then(account())
        .then(amount())
        .map(|(((date, sd), (acc, sa)), (amount, sc))| {
            (
                Op::Balance(
                    (date.get_date().unwrap(), sd.clone()),
                    (acc.get_account().unwrap(), sa),
                    (amount.get_money().unwrap(), sc.clone()),
                ),
                sd.start()..sc.end(),
            )
        })
}

fn transaction_op() -> impl Parser<Spanned<Token>, Spanned<Op>, Error = Simple<Spanned<Token>>> {
    date()
        .then_ignore(keyword("transaction"))
        .then(string())
        .then(movements())
        .map(|(((date, sd), (desc, sde)), (movs, sm))| {
            (
                Op::Transaction(
                    (date.get_date().unwrap(), sd.clone()),
                    (desc.get_description().unwrap(), sde),
                    (movs, sm.clone()),
                ),
                sd.start()..sm.end(),
            )
        })
}

pub fn parser() -> impl Parser<Spanned<Token>, Vec<Spanned<Op>>, Error = Simple<Spanned<Token>>> {
    let ops = open_op()
        .or(balance_op())
        .or(transaction_op())
        .recover_with(skip_then_retry_until([]));

    ops.repeated().collect().then_ignore(end())
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

    use crate::Money;

    fn int_rational(v: isize) -> BigRational {
        BigRational::from_integer(v.into())
    }

    #[test]
    fn test_parse_date() -> Result<()> {
        let parser = date();

        let tokens = vec![
            (Token::number(2020.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(10.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
        ];

        assert_eq!(
            parser.parse(tokens.as_slice()).unwrap().0,
            Expr::Date(NaiveDate::from_ymd(2020, 10, 1)),
        );

        Ok(())
    }

    #[test]
    fn test_parse_bounded_number() -> Result<()> {
        let parser = bounded_number(1, 10);

        assert_eq!(
            parser.parse([(Token::number(2.0), 0..1)]).unwrap().0,
            int_rational(2),
        );

        assert_eq!(
            parser.parse([(Token::number(1.0), 0..1)]).unwrap().0,
            int_rational(1),
        );

        assert_eq!(
            parser.parse([(Token::number(60.0), 0..1)]).unwrap().0,
            int_rational(60),
        );

        Ok(())
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
    fn test_parse_keyword() -> Result<()> {
        for (kw, val) in &[
            ("open", Keyword::Open),
            ("balance", Keyword::Balance),
            ("transaction", Keyword::Transaction),
        ] {
            let parser = keyword(*kw);

            let tokens = vec![(Token::identifier(kw.to_string()), 0..1)];

            assert_eq!(
                parser.parse(tokens.as_slice()).unwrap().0,
                Expr::Keyword(*val)
            );
        }

        Ok(())
    }

    #[test]
    fn test_parse_open() -> Result<()> {
        let parser = open_op();

        let tokens = vec![
            (Token::number(2020.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
            (Token::identifier("open"), 0..1),
            (Token::identifier("assets"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("cash_account"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("omg"), 0..1),
            (Token::currency("BRL"), 0..1),
        ];
        assert_eq!(
            CleanOp::from(parser.parse(tokens.as_slice()).unwrap().0),
            CleanOp::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account(
                    AccountType::Assets,
                    vec!["cash_account".into(), "omg".into()]
                ),
                "BRL".into()
            ),
        );

        Ok(())
    }

    #[test]
    fn test_parse_balance() -> Result<()> {
        let parser = balance_op();

        let tokens = vec![
            (Token::number(2020.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
            (Token::identifier("balance"), 0..1),
            (Token::identifier("assets"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("cash_account"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("omg"), 0..1),
            (Token::number(100.0), 0..1),
            (Token::currency("BRL"), 0..1),
        ];
        assert_eq!(
            CleanOp::from(parser.parse(tokens.as_slice()).unwrap().0),
            CleanOp::Balance(
                NaiveDate::from_ymd(2020, 1, 1),
                Account(
                    AccountType::Assets,
                    vec!["cash_account".into(), "omg".into()]
                ),
                Money::new(int_rational(100), "BRL")
            ),
        );

        Ok(())
    }

    #[test]
    fn test_parse_transaction() -> Result<()> {
        let parser = transaction_op();

        let tokens = vec![
            (Token::number(2020.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
            (Token::Separator('-'), 0..1),
            (Token::number(1.0), 0..1),
            (Token::identifier("transaction"), 0..1),
            (Token::String("this is so cool".into()), 0..1),
            (Token::Movement(MovementKind::Credit), 0..1),
            (Token::number(100.0), 0..1),
            (Token::currency("BRL"), 0..1),
            (Token::identifier("assets"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("cash_account"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("omg"), 0..1),
            (Token::Movement(MovementKind::Debit), 0..1),
            (Token::number(101.0), 0..1),
            (Token::currency("BRL"), 0..1),
            (Token::identifier("liabilities"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("other"), 0..1),
            (Token::Separator(':'), 0..1),
            (Token::identifier("account"), 0..1),
        ];

        let movements = vec![
            Movement(
                MovementKind::Credit,
                Money::new(int_rational(100), "BRL"),
                Account(AccountType::Assets, vec!["cash_account".into(), "omg".into()]),
            ),
            Movement(
                MovementKind::Debit,
                Money::new(int_rational(101), "BRL"),
                Account(AccountType::Liabilities, vec!["other".into(), "account".into()]),
            ),
        ];

        assert_eq!(
            CleanOp::from(parser.parse(tokens.as_slice()).unwrap().0),
            CleanOp::Transaction(
                NaiveDate::from_ymd(2020, 1, 1),
                "this is so cool".into(),
                movements
            ),
        );

        Ok(())
    }
}
