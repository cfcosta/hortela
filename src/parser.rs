use std::{fs, path::Path};

use anyhow::Result;
use chumsky::prelude::*;

use crate::{account::*, lexer::lexer, money::*, syntax::*};

pub fn parser(
) -> impl Parser<Spanned<Token>, Vec<Spanned<Expr>>, Error = Simple<Spanned<Token>>> + Clone {
    let date_parser = filter_map(|span: Span, token| match token {
        (Token::Date(d), inner) => Ok((d, inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let keyword = |keyword: Keyword| {
        filter_map(move |span, token| match token {
            (Token::Keyword(k), inner) if k == keyword => Ok((k, inner)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    };

    let account = filter_map(|span, token| match token {
        (Token::Account(acc, parts), inner) => Ok((Account(acc, parts), inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let amount = filter_map(|span, token: (Token, Span)| match token {
        (Token::Amount(a), inner) => Ok((a, inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let negative_amount = filter_map(|span, token| match token {
        (Token::NegativeAmount(a), inner) => Ok((a, inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let currency = filter_map(|span, token| match token {
        (Token::Currency(c), inner) => Ok((c, inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let description = filter_map(|span, token| match token {
        (Token::Description(d), inner) => Ok((d, inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let movement = filter_map(|span, token| match token {
        (Token::Movement(kind), inner) => Ok((kind, inner)),
        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
    });

    let money_parser = amount
        .then(currency)
        .map(|((amount, sa), (currency, sc))| (Money(amount, currency), sa.start()..sc.end()));

    let balance_amount_parser = amount
        .map(|(x, span)| (x as i64, span))
        .or(negative_amount)
        .then(currency)
        .map(|((amount, sa), (currency, sc))| {
            (
                ((amount, sa.clone()), (currency, sc.clone())),
                sa.start()..sc.end(),
            )
        });

    let movement_expr = movement
        .then(money_parser)
        .then(account)
        .repeated()
        .at_least(1)
        .map(|movs| {
            movs.into_iter()
                .map(|(((mov, sm), (money, _)), (acc, sa))| {
                    (Movement(mov, money, acc), sm.start()..sa.end())
                })
                .collect::<Vec<_>>()
        })
        .map_with_span(|movs, span| (movs, span));

    let open_expr = date_parser
        .then_ignore(keyword(Keyword::Open))
        .then(account)
        .then(currency)
        .map_with_span(|((date, acc), cur), span| (Expr::Open(date, acc, cur), span));

    let balance_expr = date_parser
        .then_ignore(keyword(Keyword::Balance))
        .then(account)
        .then(balance_amount_parser)
        .map_with_span(|(((date, sd), acc), ((amount, cur), sc)), _| {
            (
                Expr::Balance((date, sd.clone()), acc, amount, cur),
                sd.start()..sc.end(),
            )
        });

    let transaction_expr = date_parser
        .then_ignore(keyword(Keyword::Transaction))
        .then(description)
        .then(movement_expr)
        .map_with_span(|(((date, ds), desc), (movs, ms)), _| {
            (
                Expr::Transaction((date, ds.clone()), desc, (movs, ms.clone())),
                ds.start()..ms.end(),
            )
        });

    open_expr
        .or(balance_expr)
        .or(transaction_expr)
        .repeated()
        .at_least(1)
}

pub fn parse_string(input: &str) -> Result<Vec<Spanned<Expr>>> {
    let lexer = lexer();
    let parser = parser();

    let (tokens, errs) = lexer.parse_recovery(input);

    match tokens {
        Some((l, _)) => {
            let (exprs, parse_errs) = parser.parse_recovery(l.as_slice());

            match exprs {
                Some(e) => Ok(e),
                None => panic!("{:?}", parse_errs),
            }
        }
        None => panic!("{:?}", errs),
    }
}

pub fn parse_file<'a, P: AsRef<Path>>(path: P) -> Result<Vec<Spanned<Expr>>> {
    let input = fs::read_to_string(path)?;

    parse_string(&input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use chrono::prelude::*;

    fn clean_up(input: Vec<Spanned<Expr>>) -> Vec<CleanExpr> {
        input
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<CleanExpr>>()
    }

    #[test]
    fn test_parse_open() -> Result<()> {
        assert_eq!(
            clean_up(parse_string("2020-01-01 open assets:cash_account BRL")?),
            vec![CleanExpr::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account(AccountType::Assets, vec!["cash_account".into()]),
                "BRL".into()
            ),]
        );

        Ok(())
    }

    #[test]
    fn test_parse_multiple_open() -> Result<()> {
        let transactions = Vec::from([
            CleanExpr::Open(
                NaiveDate::from_ymd(2020, 1, 1),
                Account(AccountType::Assets, vec!["cash_account".into()]),
                "BRL".into(),
            ),
            CleanExpr::Open(
                NaiveDate::from_ymd(2020, 1, 2),
                Account(AccountType::Liabilities, vec!["credit_card".into()]),
                "BRL".into(),
            ),
        ]);

        assert_eq!(
            clean_up(parse_string("2020-01-01 open assets:cash_account BRL\n 2020-01-02 open liabilities:credit_card BRL")?),
            transactions
        );

        Ok(())
    }

    #[test]
    fn test_parse_transaction() -> Result<()> {
        use AccountType::*;
        use MovementKind::*;

        let movements = vec![
            Movement(
                Debit,
                Money::from_float(400.0, "BRL"),
                Account(Assets, vec!["omg_asset".into()]),
            ),
            Movement(
                Credit,
                Money::from_float(400.0, "BRL"),
                Account(Equity, vec!["omg_equity".into()]),
            ),
        ];

        let transaction = Vec::from([CleanExpr::Transaction(
            NaiveDate::from_ymd(2020, 1, 1),
            "Hello World".into(),
            movements,
        )]);

        assert_eq!(
            clean_up(parse_string(
                "
                     2020-01-01 transaction \"Hello World\"
                     < 400 BRL assets:omg_asset
                     > 400 BRL equity:omg_equity
                "
            )?),
            transaction
        );

        Ok(())
    }
}
