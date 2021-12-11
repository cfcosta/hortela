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
        .then(movement_expr.repeated().collect::<Vec<_>>())
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

            dbg!(&list, errs);
            let (exprs, parse_errs) = parser.parse_recovery(list.as_slice());

            dbg!(exprs, parse_errs);

            Ok(vec![])
        }
        None => panic!("none"),
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

    #[test]
    fn test_parse_transaction() -> Result<()> {
        let transactions = Vec::from([
            (
                Expr::Transaction(
                    NaiveDate::from_ymd(2020, 1, 1),
                    "Hello World".into(),
                    vec![]
                ),
                0..4,
            ),
        ]);

        assert_eq!(
            parse_string("\n 2020-01-01 transaction \"hello World\"\n < 400 BRL assets:omg")?,
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
