use std::sync::{Arc, LazyLock};

use anyhow::{bail, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use zip::read_zip;

use crate::{
    data::{
        AccountNotes, Aeat720Record, Aeat720Records, BalanceNotes, BrokerInformation, DEFAULT_YEAR,
    },
    parsers::{
        degiro::DegiroParser, degiro_csv::DegiroCSVParser, ib::IBParser, ib_csv::IBCSVParser,
        pdf::read_pdf,
    },
};

pub mod decimal;
pub mod icons;
pub mod web;
pub mod zip;

static DEGIRO_BROKER: LazyLock<Arc<BrokerInformation>> = LazyLock::new(|| {
    Arc::new(BrokerInformation::new(
        String::from("Degiro"),
        String::from("NL"),
    ))
});

static IB_BROKER: LazyLock<Arc<BrokerInformation>> = LazyLock::new(|| {
    Arc::new(BrokerInformation::new(
        String::from("Interactive Brokers"),
        String::from("IE"),
    ))
});

pub fn usize_to_date(date_int: usize) -> Option<NaiveDate> {
    let mut date = date_int;
    let day = date % 100;
    date /= 100;
    let month = date % 100;
    date /= 100;
    NaiveDate::from_ymd_opt(date as i32, month as u32, day as u32)
}

fn read_degiro_pdf(content: Vec<u8>) -> Result<(BalanceNotes, AccountNotes)> {
    if let Ok(data) = read_pdf(&content) {
        let parser = DegiroParser::new(data, &DEGIRO_BROKER);
        parser.parse_pdf_content()
    } else {
        bail!("Error parseando el pdf de Degiro".to_string());
    }
}

fn read_ib_html(content: Vec<u8>) -> Result<(BalanceNotes, AccountNotes)> {
    if let Ok(data) = String::from_utf8(content) {
        if let Ok(parser) = IBParser::new(&data, &IB_BROKER) {
            let account_notes = parser.parse_account_notes()?;
            let balance_notes = parser.parse_balance_notes()?;
            Ok((balance_notes, account_notes))
        } else {
            bail!("Unable to parse interactive brokers html");
        }
    } else {
        bail!("Unable to get string from interactive brokers html content");
    }
}

fn read_ib_csv(content: Vec<u8>) -> Result<(BalanceNotes, AccountNotes)> {
    if let Ok(data) = String::from_utf8(content) {
        if let Ok(parser) = IBCSVParser::new(data, &IB_BROKER) {
            let account_notes = parser.parse_account_notes()?;
            let balance_notes = parser.parse_balance_notes()?;
            Ok((balance_notes, account_notes))
        } else {
            bail!("Unable to parse interactive brokers CSV");
        }
    } else {
        bail!("Unable to get string from interactive brokers csv content");
    }
}

fn read_degiro_csv(content: Vec<u8>) -> Result<(BalanceNotes, AccountNotes)> {
    if let Ok(data) = String::from_utf8(content) {
        let parser = DegiroCSVParser::new(data, &DEGIRO_BROKER);
        let balance_notes = parser.parse_csv()?;
        Ok((balance_notes, vec![]))
    } else {
        bail!("Unable to parse Degiro CSV");
    }
}

fn transform_to_aeat720_records(notes: (BalanceNotes, AccountNotes)) -> Result<Aeat720Records> {
    let mut result = vec![];

    for note in notes.0.iter() {
        let first_tx_date = {
            let company = notes.1.iter().find(|&x| x.company == note.company);
            match company {
                Some(c) => c.date.format("%Y%m%d").to_string(),
                None => NaiveDate::from_ymd_opt(DEFAULT_YEAR as i32, 1, 1)
                    .unwrap()
                    .format("%Y%m%d")
                    .to_string(),
            }
            .parse::<usize>()
            .unwrap_or(0)
        };
        result.push(Aeat720Record {
            company: note.company.clone(),
            quantity: note.quantity,
            value_in_euro: note.value_in_euro,
            first_tx_date,
            broker: note.broker.clone(),
            percentage: Decimal::new(100, 0),
        })
    }

    Ok(result)
}

pub(crate) fn file_importer(content: Vec<u8>) -> Result<Aeat720Records> {
    let file_type = infer::get(&content);

    match file_type {
        Some(infer_type) => match infer_type.extension() {
            "zip" => file_importer(read_zip(content)?),
            "html" => transform_to_aeat720_records(read_ib_html(content)?),
            "pdf" => transform_to_aeat720_records(read_degiro_pdf(content)?),
            _ => {
                bail!("{} Infer types not valid", infer_type);
            }
        },
        None => {
            if content.starts_with("Producto".as_bytes()) {
                transform_to_aeat720_records(read_degiro_csv(content)?)
            } else {
                transform_to_aeat720_records(read_ib_csv(content)?)
            }
        }
    }
}
