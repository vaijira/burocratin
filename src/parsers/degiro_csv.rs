use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use std::{str::FromStr, sync::Arc};

use crate::{
    data::{BalanceNote, BalanceNotes, BrokerInformation, CompanyInfo},
    utils::decimal,
};

pub struct DegiroCSVParser {
    content: String,
    broker: Arc<BrokerInformation>,
}

impl DegiroCSVParser {
    pub fn parse_csv(&self) -> Result<BalanceNotes> {
        let mut rdr = csv::Reader::from_reader(self.content.as_bytes());
        let mut balance_notes = vec![];

        for result in rdr.records() {
            let record = result?;
            log::debug!("{:?}", record);
            if record.get(1) == Some("") {
                continue;
            }
            let currency_price = record
                .get(4)
                .ok_or_else(|| anyhow!("Unknown currency/price"))?;
            let currency = if let Some(currency_end_index) = currency_price.find(' ') {
                &currency_price[0..currency_end_index]
            } else {
                currency_price
            };
            let note = BalanceNote::new(
                CompanyInfo {
                    name: record
                        .get(0)
                        .ok_or_else(|| anyhow!("Unknown company"))?
                        .to_string(),
                    isin: record
                        .get(1)
                        .ok_or_else(|| anyhow!("Unknown ISIN"))?
                        .to_string(),
                },
                String::from(""),
                Decimal::from_str(&decimal::transform_i18n_es_str(
                    record.get(2).ok_or_else(|| anyhow!("Unknow quantity"))?,
                ))?,
                currency.to_string(),
                Decimal::from_str(&decimal::transform_i18n_es_str(
                    record
                        .get(3)
                        .ok_or_else(|| anyhow!("Unable to get price"))?,
                ))?,
                Decimal::from_str(&decimal::transform_i18n_es_str(
                    record
                        .get(5)
                        .ok_or_else(|| anyhow!("Unable to get value in euro"))?,
                ))?,
                &self.broker,
            );

            balance_notes.push(note);
        }

        Ok(balance_notes)
    }

    pub fn new(content: String, broker: &Arc<BrokerInformation>) -> Self {
        Self {
            content,
            broker: Arc::clone(broker),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare_vectors_by_item<T>(vec1: &[T], vec2: &[T])
    where
        T: std::fmt::Debug + std::cmp::PartialEq,
    {
        let mut line_number = 1;
        let mut iter1 = vec1.iter();
        let mut iter2 = vec2.iter();
        while let (Some(item1), Some(item2)) = (iter1.next(), iter2.next()) {
            assert_eq!(
                *item1, *item2,
                "comparing items in vectors, item number: {}",
                line_number
            );
            line_number += 1;
        }
    }

    #[test]
    #[allow(clippy::mistyped_literal_suffixes)]
    fn test_parse_csv() {
        let degiro_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));
        let parser = DegiroCSVParser::new(INPUT_2019.to_string(), &degiro_broker);
        let balance_notes = parser.parse_csv().unwrap();
        let bal_notes = vec![
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("ANGI HOMESERVICES INC- A"),
                    isin: String::from("US00183L1026"),
                },
                String::from(""),
                Decimal::new(300, 0),
                String::from("USD"),
                Decimal::new(8_47, 2),
                Decimal::new(2266_32, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("BURFORD CAP LD"),
                    isin: String::from("GG00B4L84979"),
                },
                String::from(""),
                Decimal::new(463, 0),
                String::from("GBX"),
                Decimal::new(712_00, 2),
                Decimal::new(3898_18, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028"),
                },
                String::from(""),
                Decimal::new(260, 0),
                String::from("USD"),
                Decimal::new(26_97, 2),
                Decimal::new(6254_18, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("GRAVITY CO. LTD. - AM"),
                    isin: String::from("US38911N2062"),
                },
                String::from(""),
                Decimal::new(102, 0),
                String::from("USD"),
                Decimal::new(37_40, 2),
                Decimal::new(3402_42, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("JD.COM INC. - AMERICA"),
                    isin: String::from("US47215P1066"),
                },
                String::from(""),
                Decimal::new(140, 0),
                String::from("USD"),
                Decimal::new(35_23, 2),
                Decimal::new(4399_03, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("JUDGES SCIENTFC"),
                    isin: String::from("GB0032398678"),
                },
                String::from(""),
                Decimal::new(145, 0),
                String::from("GBX"),
                Decimal::new(5650_00, 2),
                Decimal::new(9687_63, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("META PLATFORMS INC"),
                    isin: String::from("US30303M1027"),
                },
                String::from(""),
                Decimal::new(21, 0),
                String::from("USD"),
                Decimal::new(205_25, 2),
                Decimal::new(3844_31, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("MONDO TV"),
                    isin: String::from("IT0001447785"),
                },
                String::from(""),
                Decimal::new(1105, 0),
                String::from("EUR"),
                Decimal::new(2_39, 2),
                Decimal::new(2640_95, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("ROCKROSE ENERGY"),
                    isin: String::from("GB00BYNFCH09"),
                },
                String::from(""),
                Decimal::new(216, 0),
                String::from("GBX"),
                Decimal::new(1870_00, 2),
                Decimal::new(4776_35, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("XPO LOGISTICS INC."),
                    isin: String::from("US9837931008"),
                },
                String::from(""),
                Decimal::new(69, 0),
                String::from("USD"),
                Decimal::new(79_72, 2),
                Decimal::new(4906_06, 2),
                &degiro_broker,
            ),
        ];

        compare_vectors_by_item(&bal_notes, &balance_notes);
    }

    const INPUT_2019: &str = r#"Producto,Symbol/ISIN,Cantidad,Precio de,Valor local,Valor en EUR
CASH & CASH FUND & FTX CASH (EUR),,,,EUR 564.19,"564,19"
ANGI HOMESERVICES INC- A,US00183L1026,300,"8,47",USD 2541.00,"2266,32"
BURFORD CAP LD,GG00B4L84979,463,"712,00",GBX 329656.00,"3898,18"
EVI INDUSTRIES INC,US26929N1028,260,"26,97",USD 7012.20,"6254,18"
GRAVITY CO. LTD. - AM,US38911N2062,102,"37,40",USD 3814.80,"3402,42"
JD.COM INC. - AMERICA,US47215P1066,140,"35,23",USD 4932.20,"4399,03"
JUDGES SCIENTFC,GB0032398678,145,"5650,00",GBX 819250.00,"9687,63"
META PLATFORMS INC,US30303M1027,21,"205,25",USD 4310.25,"3844,31"
MONDO TV,IT0001447785,1105,"2,39",EUR 2640.95,"2640,95"
ROCKROSE ENERGY,GB00BYNFCH09,216,"1870,00",GBX 403920.00,"4776,35"
XPO LOGISTICS INC.,US9837931008,69,"79,72",USD 5500.68,"4906,06""#;
}
