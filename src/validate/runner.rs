use std::path::Path;

use anyhow::{bail, Result};
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};

use crate::{
    ledger::Ledger,
    validate::{ValidationError, ALL_VALIDATORS},
};

pub struct Runner;

impl Runner {
    pub fn run_all(filename: &Path, input: &str, ledger: &Ledger) -> Result<()> {
        for (name, validator) in ALL_VALIDATORS {
            print!("Running validator: {}...", name);

            match validator(&ledger.clone()) {
                Ok(_) => {
                    println!(" OK");
                }
                Err(ValidationError::WithTrace(traces)) => {
                    println!(" ERROR");

                    traces.into_iter().for_each(|t| {
                        let span = t.span.clone().unwrap_or(0..1);
                        let report = Report::build(ReportKind::Error, (), span.start);

                        let message_parts = vec![
                            Some(t.message),
                            t.expected
                                .map(|x| format!("`{}`", x).fg(Color::Red).to_string()),
                            t.found.map(|f| format!("found {}", f.fg(Color::Blue))),
                        ];

                        let message = message_parts
                            .into_iter()
                            .filter_map(|x| x)
                            .collect::<Vec<String>>();

                        let mut report = report.with_message(message.join(", "));

                        if t.span.is_some() {
                            report = report.with_label(
                                Label::new(span)
                                    .with_message(t.details)
                                    .with_color(Color::Blue),
                            );
                        }

                        report.finish().eprint(Source::from(&input)).unwrap();
                    });

                    bail!("Running validation `{}` failed.", name.fg(Color::Green));
                }
                Err(e) => bail!(e),
            }
        }

        Ok(())
    }
}
