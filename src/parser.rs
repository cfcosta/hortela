use std::{ path::Path, fs };

use anyhow::Result;
use chumsky::prelude::*;

use crate::{ account::*, money::*, lexer::lexer, syntax::* };

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
        .repeated()
        .at_least(1)
        .map(|movs| movs.into_iter().map(|((mov, money), acc)| Movement(mov, money, acc)).collect())
        .map_with_span(|movs, span| (movs, span));

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
        .then(movement_expr)
        .map_with_span(|(((date, _), desc), (movs, _)), span| {
            (
                Expr::Transaction(date, desc, movs),
                span,
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
        Some(l) => {
            let list: Vec<Token> = l.into_iter().map(|(t, _)| t).collect();
            let (exprs, parse_errs) = parser.parse_recovery(list.as_slice());

            match exprs {
                Some(e) => Ok(e),
                None => panic!("{:?}", parse_errs)
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

    #[test]
    fn test_parse_open() -> Result<()> {
        assert_eq!(
            parse_string("2020-01-01 open assets:cash_account BRL")?,
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
    fn test_parse_multiple_open() -> Result<()> {
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
            parse_string("2020-01-01 open assets:cash_account BRL\n 2020-01-02 open liabilities:credit_card BRL")?,
           transactions 
        );

        Ok(())
    }

    #[test]
    fn test_parse_transaction() -> Result<()> {
        use MovementKind::*;
        use AccountType::*;

        let movements = vec![
            Movement(Debit, Money::from_float(400.0, "BRL"), Account(Assets, vec!["omg_asset".into()])),
            Movement(Credit, Money::from_float(400.0, "BRL"), Account(Equity, vec!["omg_equity".into()])),
        ];

        let transaction = Vec::from([
            (
                Expr::Transaction(
                    NaiveDate::from_ymd(2020, 1, 1),
                    "Hello World".into(),
                    movements
                ),
                0..11,
            ),
        ]);

        assert_eq!(
           parse_string("
                2020-01-01 transaction \"Hello World\"
                < 400 BRL assets:omg_asset
                > 400 BRL equity:omg_equity
           ")?,
           transaction 
        );

        Ok(())
    }
}
