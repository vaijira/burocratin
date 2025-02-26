use std::sync::Arc;

use crate::data::{
    AccountNote, AccountNotes, BalanceNote, BalanceNotes, BrokerInformation, BrokerOperation,
    CompanyInfo,
};

use crate::utils::decimal;

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use nom::character::complete::anychar;
use nom::error::ErrorKind;
use nom::multi::many_till;
use nom::sequence::{preceded, separated_pair};
use nom::{IResult, Parser, error::context};
use nom::{
    branch::alt,
    bytes::complete::{is_a, take},
    character::complete::none_of,
    combinator::{map_res, opt, recognize},
    multi::many0,
    multi::many1,
    sequence::terminated,
};
use nom::{
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, one_of},
    multi::many_m_n,
};

use rust_decimal::prelude::*;

type Res<T, U> = IResult<T, U, (T, ErrorKind)>;

pub struct DegiroParser {
    content: String,
    broker: Arc<BrokerInformation>,
}

pub(crate) const DEGIRO_BALANCE_NOTES_HEADER: &str = r#"Producto ISIN Bolsa Cantidad Moneda Precio Valor (EUR)
Tipo de
producto
"#;

const DEGIRO_BALANCE_HEADER_BEGIN: &str = "CurrencyCASH & CASH FUND (EUR)";
const DEGIRO_BALANCE_HEADER_END: &str = "Amsterdam, ";

const DEGIRO_NOTES_HEADER_END: &str = "EURTotal\n\nInforme anual de flatex";

pub(crate) const DEGIRO_NOTES_HEADER_BEGIN: &str = r#"
Fecha Producto Symbol/ISIN Tipo de
orden Cantidad Precio Valor local Valor en EUR Comisión Tipo de
cambio Beneficios y
pérdidas
"#;

impl DegiroParser {
    fn n_to_m_digits<'b>(n: usize, m: usize) -> impl FnMut(&'b str) -> Res<&'b str, String> {
        move |input| {
            many_m_n(n, m, one_of("0123456789"))
                .parse(input)
                .map(|(next_input, result)| (next_input, result.into_iter().collect()))
        }
    }

    fn decimal_value(input: &str) -> Res<&str, Decimal> {
        context(
            "decimal value",
            map_res(
                recognize(many1(terminated(one_of("0123456789"), many0(is_a(",."))))),
                |out: &str| Decimal::from_str(&decimal::transform_i18n_es_str(out)),
            ),
        )
        .parse(input)
    }

    fn number_no_decimal_digits(input: &str) -> Res<&str, Decimal> {
        context(
            "number no decimal digits",
            map_res(recognize(many1(one_of("0123456789"))), |out: &str| {
                Decimal::from_str(out)
            }),
        )
        .parse(input)
    }

    fn number_decimal_digits(input: &str, count: usize) -> Res<&str, Decimal> {
        context(
            "number n decimal digits",
            map_res(
                recognize(separated_pair(
                    many1(one_of("0123456789.")),
                    tag(","),
                    take(count),
                )),
                |out: &str| Decimal::from_str(&decimal::transform_i18n_es_str(out)),
            ),
        )
        .parse(input)
    }

    fn earnings_value(input: &str) -> Res<&str, Decimal> {
        context(
            "earnings value",
            map_res(
                recognize((
                    opt(one_of("+-")),
                    recognize(many1(terminated(one_of("0123456789"), many0(char('.'))))),
                    char(','),
                    recognize(many1(terminated(one_of("0123456789"), many0(char('.'))))),
                )),
                |out: &str| Decimal::from_str(&decimal::transform_i18n_es_str(out)),
            ),
        )
        .parse(input)
    }

    fn date_concept(input: &str) -> Res<&str, NaiveDate> {
        context(
            "date concept",
            (
                DegiroParser::n_to_m_digits(1, 2),
                tag("/"),
                DegiroParser::n_to_m_digits(1, 2),
                tag("/"),
                DegiroParser::n_to_m_digits(1, 4),
            ),
        )
        .parse(input)
        .map(|(next_input, res)| {
            let (day, _, month, _, year) = res;
            (
                next_input,
                NaiveDate::from_ymd_opt(
                    year.parse::<i32>().unwrap(),
                    month.parse::<u32>().unwrap(),
                    day.parse::<u32>().unwrap(),
                )
                .unwrap(),
            )
        })
    }

    fn broker_operation(input: &str) -> Res<&str, BrokerOperation> {
        context(
            "broker operation",
            alt((tag_no_case("C"), tag_no_case("V"))),
        )
        .parse(input)
        .map(|(next_input, res)| (next_input, res.into()))
    }

    fn isin(input: &str) -> Res<&str, String> {
        context(
            "isin",
            (
                many_m_n(2, 2, none_of("\t \n0123456789")),
                many_m_n(9, 9, none_of("\t \n")),
                many1(one_of("0123456789")),
            ),
        )
        .parse(input)
        .map(|(next_input, res)| {
            let (prefix, main, control) = res;
            let mut result: String = prefix.iter().collect();
            result.push_str(&main.iter().collect::<String>());
            result.push_str(&control.iter().collect::<String>());
            (next_input, result)
        })
    }

    fn company_info(input: &str) -> Res<&str, CompanyInfo> {
        context("company info", many_till(anychar, DegiroParser::isin))
            .parse(input)
            .map(|(next_input, res)| {
                let (company_name, isin) = res;
                let company_name: String = company_name.into_iter().collect();
                let company_name = company_name.replace('\n', " ").trim_end().to_string();

                (
                    next_input,
                    CompanyInfo {
                        name: company_name,
                        isin,
                    },
                )
            })
    }

    fn account_note<'a>(
        input: &'a str,
        broker: &Arc<BrokerInformation>,
    ) -> Res<&'a str, AccountNote> {
        context(
            "account note",
            (
                DegiroParser::date_concept,
                tag(" "),
                DegiroParser::company_info,
                tag(" "),
                DegiroParser::broker_operation,
                tag(" "),
                DegiroParser::decimal_value,
                tag(" "),
                DegiroParser::decimal_value,
                tag(" "),
                DegiroParser::decimal_value,
                tag(" "),
                DegiroParser::decimal_value,
                tag(" "),
                DegiroParser::decimal_value,
                tag(" "),
                DegiroParser::decimal_value,
                opt((char(' '), DegiroParser::earnings_value)),
                tag("\n"),
            ),
        )
        .parse(input)
        .map(|(next_input, res)| {
            let (
                date,
                _,
                company,
                _,
                operation,
                _,
                quantity,
                _,
                price,
                _,
                value,
                _,
                _value_in_euro,
                _,
                commision,
                _,
                _exchange_rate,
                _earnings_value,
                _,
            ) = res;

            (
                next_input,
                AccountNote::new(
                    date, company, operation, quantity, price, value, commision, broker,
                ),
            )
        })
    }

    fn balance_note<'a>(
        input: &'a str,
        broker: &Arc<BrokerInformation>,
    ) -> Res<&'a str, BalanceNote> {
        log::trace!("balance note: -{}-", input);
        context(
            "balance note",
            (
                tag("\n "),
                |input| DegiroParser::number_decimal_digits(input, 2), // value in euro
                |input| DegiroParser::number_decimal_digits(input, 4), // price
                take(3usize),                                          // currency
                DegiroParser::number_no_decimal_digits,                // quantity
                take(3usize),                                          // market
                alt((tag("Stock"), tag("ETF"))),                       // product type: Stock | ETF
                DegiroParser::company_info,                            // company info
            ),
        )
        .parse(input)
        .map(|(next_input, res)| {
            let (_, value_in_euro, price, currency, quantity, market, _product_type, company) = res;

            (
                next_input,
                BalanceNote::new(
                    company,
                    market.to_string(),
                    quantity,
                    currency.to_string(),
                    price,
                    value_in_euro,
                    broker,
                ),
            )
        })
    }

    fn account_notes<'a>(
        input: &'a str,
        broker: &Arc<BrokerInformation>,
    ) -> Res<&'a str, AccountNotes> {
        context(
            "account notes",
            many0(preceded(char('\n'), |x| {
                DegiroParser::account_note(x, broker)
            })),
        )
        .parse(input)
    }

    fn balance_notes<'a>(
        input: &'a str,
        broker: &Arc<BrokerInformation>,
    ) -> Res<&'a str, BalanceNotes> {
        context(
            "balance notes",
            many0(|x| DegiroParser::balance_note(x, broker)),
        )
        .parse(input)
    }

    fn parse_account_notes(&self, notes: &str) -> Result<AccountNotes> {
        log::debug!("account notes:-{}-", notes);
        let notes = match DegiroParser::account_notes(notes, &self.broker) {
            Ok((_, notes)) => {
                log::debug!("Ok parsing {} account notes", notes.len());
                notes
            }
            Err(err) => {
                bail!("Unable to parse account notes: {}", err);
            }
        };

        Ok(notes)
    }

    fn parse_balance_notes(&self, notes: &str) -> Result<BalanceNotes> {
        log::debug!("balance notes:-{}-", notes);
        let notes = match DegiroParser::balance_notes(notes, &self.broker) {
            Ok((_, notes)) => {
                log::debug!("Ok parsing {} balance notes", notes.len());
                notes
            }
            Err(err) => {
                log::debug!("Unable to parse balance notes:-{}-", err);
                bail!("Unable to parse balance notes: {}", err);
            }
        };

        Ok(notes)
    }

    fn parse_pdf_account_notes(&self) -> Result<AccountNotes> {
        let mut result = vec![];

        let header_begin = self
            .content
            .find(DEGIRO_NOTES_HEADER_BEGIN)
            .context("No account notes section found")?;

        let header_end = match self.content.rfind(DEGIRO_NOTES_HEADER_END) {
            Some(end) => end - 1,
            None => self.content.len(),
        };

        let header_end = if let Some(pos) = self.content[..header_end].rfind('\n') {
            pos
        } else {
            header_end
        };

        result.extend(self.parse_account_notes(
            &self.content[header_begin + DEGIRO_NOTES_HEADER_BEGIN.len()..header_end],
        )?);

        Ok(result)
    }

    fn parse_pdf_balance_notes(&self) -> Result<BalanceNotes> {
        let mut result = vec![];

        let indexes: Vec<_> = self
            .content
            .match_indices(DEGIRO_BALANCE_HEADER_BEGIN)
            .collect();

        for i in 0..indexes.len() {
            let header_begin = indexes.get(i).unwrap().0 + DEGIRO_BALANCE_HEADER_BEGIN.len();
            let header_end = if i < indexes.len() - 1 {
                indexes.get(i + 1).unwrap().0
            } else {
                match self.content.find(DEGIRO_BALANCE_HEADER_END) {
                    Some(end) => end - 1,
                    None => self.content.len(),
                }
            };
            result.extend(self.parse_balance_notes(&self.content[header_begin..header_end - 1])?);
        }

        Ok(result)
    }

    pub fn new(content: String, broker: &Arc<BrokerInformation>) -> Self {
        Self {
            content,
            broker: Arc::clone(broker),
        }
    }

    pub fn parse_pdf_content(&self) -> Result<(BalanceNotes, AccountNotes)> {
        let account_notes = self.parse_pdf_account_notes()?;
        let balance_notes = self.parse_pdf_balance_notes()?;

        Ok((balance_notes, account_notes))
    }
}

#[cfg(test)]
#[allow(clippy::mistyped_literal_suffixes)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn broker_operation_test() {
        assert_eq!(
            DegiroParser::broker_operation("C "),
            Ok((" ", BrokerOperation::Buy))
        );
        assert_eq!(
            DegiroParser::broker_operation("V "),
            Ok((" ", BrokerOperation::Sell))
        );
        assert_eq!(
            DegiroParser::broker_operation("Z "),
            Err(nom::Err::Error(("Z ", ErrorKind::Tag)))
        );
    }

    #[test]
    fn date_concept_test() {
        assert_eq!(
            DegiroParser::date_concept("03/11/2018 "),
            Ok((" ", NaiveDate::from_ymd_opt(2018, 11, 3).unwrap()))
        );
        assert_eq!(
            DegiroParser::date_concept("31/10/2018 "),
            Ok((" ", NaiveDate::from_ymd_opt(2018, 10, 31).unwrap()))
        );
        assert_eq!(
            DegiroParser::date_concept("32_23_2020 "),
            Err(nom::Err::Error(("_23_2020 ", ErrorKind::Tag)))
        );
    }

    #[test]
    fn isin_test() {
        assert_eq!(
            DegiroParser::isin("GG00B4L84979 "),
            Ok((" ", String::from("GG00B4L84979")))
        );
        assert_eq!(
            DegiroParser::isin("IL0011320343 "),
            Ok((" ", String::from("IL0011320343")))
        );
        assert_eq!(
            DegiroParser::isin("US342342 "),
            Err(nom::Err::Error((" ", ErrorKind::NoneOf)))
        );
    }

    #[test]
    fn company_info_test() {
        let company_info_burford: &str = r#"BURFORD CAP LD GG00B4L84979 "#;

        assert_eq!(
            DegiroParser::company_info(company_info_burford),
            Ok((
                " ",
                CompanyInfo {
                    name: String::from("BURFORD CAP LD"),
                    isin: String::from("GG00B4L84979"),
                }
            ))
        );

        let company_info_gxo: &str = r#"GXO LOGISTICS INC. COMMON
STOCK WHEN-ISSUED US36262G1013 "#;

        assert_eq!(
            DegiroParser::company_info(company_info_gxo),
            Ok((
                " ",
                CompanyInfo {
                    name: String::from("GXO LOGISTICS INC. COMMON STOCK WHEN-ISSUED"),
                    isin: String::from("US36262G1013"),
                }
            ))
        );
    }

    #[test]
    fn decimal_value_test() {
        assert_eq!(
            DegiroParser::decimal_value("1.000,03 "),
            Ok((" ", Decimal::new(1_000_03, 2)))
        );
        assert_eq!(
            DegiroParser::decimal_value("300 "),
            Ok((" ", Decimal::new(300, 0)))
        );
        assert_eq!(
            DegiroParser::decimal_value("0,9030 "),
            Ok((" ", Decimal::new(9030, 4)))
        );
        assert_eq!(
            DegiroParser::decimal_value("a234,23 "),
            Err(nom::Err::Error(("a234,23 ", ErrorKind::OneOf)))
        );
    }

    #[test]
    fn number_decimal_digits_test() {
        assert_eq!(
            DegiroParser::number_decimal_digits("1.000,03 ", 2),
            Ok((" ", Decimal::new(1_000_03, 2)))
        );
        assert_eq!(
            DegiroParser::number_decimal_digits("300,00 ", 2),
            Ok((" ", Decimal::new(300, 0)))
        );
        assert_eq!(
            DegiroParser::number_decimal_digits("0,90 ", 2),
            Ok((" ", Decimal::new(90, 2)))
        );
        assert_eq!(
            DegiroParser::number_decimal_digits("a234,23 ", 2),
            Err(nom::Err::Error(("a234,23 ", ErrorKind::OneOf)))
        );
    }

    #[test]
    fn earnings_value_test() {
        assert_eq!(
            DegiroParser::earnings_value("-500,03\n"),
            Ok(("\n", Decimal::new(-500_03, 2)))
        );
        assert_eq!(
            DegiroParser::earnings_value("300,00\n"),
            Ok(("\n", Decimal::new(300_00, 2)))
        );
        assert_eq!(
            DegiroParser::earnings_value("0,9030\n"),
            Ok(("\n", Decimal::new(9030, 4)))
        );
        assert_eq!(
            DegiroParser::earnings_value("1234\n"),
            Err(nom::Err::Error(("\n", ErrorKind::Char)))
        );
    }

    #[test]
    fn balance_note_test() {
        let degiro_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));

        const BURFORD_NOTE: &str = r#"
 2.247,001.656,0000GBX122LSEStockBURFORD CAP LD GG00B4L84979"#;

        assert_eq!(
            DegiroParser::balance_note(BURFORD_NOTE, &degiro_broker),
            Ok((
                "",
                BalanceNote::new(
                    CompanyInfo {
                        name: String::from("BURFORD CAP LD"),
                        isin: String::from("GG00B4L84979")
                    },
                    String::from("LSE"),
                    Decimal::new(122, 0),
                    String::from("GBX"),
                    Decimal::new(1_6560000, 4),
                    Decimal::new(2247_00, 2),
                    &degiro_broker,
                )
            ))
        );
    }

    #[test]
    fn account_note_test() {
        let degiro_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));
        const BURFORD_NOTE: &str = r#"31/10/2018 BURFORD CAP LD GG00B4L84979 C 122 1.616,0000 197.152,00 2.247,93 5,28 0,0114
"#;

        assert_eq!(
            DegiroParser::account_note(BURFORD_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd_opt(2018, 10, 31).unwrap(),
                    CompanyInfo {
                        name: String::from("BURFORD CAP LD"),
                        isin: String::from("GG00B4L84979")
                    },
                    BrokerOperation::Buy,
                    Decimal::new(122, 0),
                    Decimal::new(1_616_0000, 4),
                    Decimal::new(197_152_00, 2),
                    Decimal::new(5_28, 2),
                    &degiro_broker,
                )
            ))
        );

        const BURFORD_LONG_NOTE: &str = r#"31/10/2018 BURFORD CAP LD GG00B4L84979 C 122 1.616,0000 197.152,00 2.247,93 5,28 0,0114
"#;

        assert_eq!(
            DegiroParser::account_note(BURFORD_LONG_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd_opt(2018, 10, 31).unwrap(),
                    CompanyInfo {
                        name: String::from("BURFORD CAP LD"),
                        isin: String::from("GG00B4L84979")
                    },
                    BrokerOperation::Buy,
                    Decimal::new(122, 0),
                    Decimal::new(1_616_0000, 4),
                    Decimal::new(197_152_00, 2),
                    Decimal::new(5_28, 2),
                    &degiro_broker,
                )
            ))
        );

        const GXO_LONG_NOTE: &str = r#"02/08/2021 GXO LOGISTICS INC. COMMON
STOCK WHEN-ISSUED US36262G1013 C 69 0,0000 0,00 0,00 0,00 0,8423
"#;

        assert_eq!(
            DegiroParser::account_note(GXO_LONG_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd_opt(2021, 8, 2).unwrap(),
                    CompanyInfo {
                        name: String::from("GXO LOGISTICS INC. COMMON STOCK WHEN-ISSUED"),
                        isin: String::from("US36262G1013")
                    },
                    BrokerOperation::Buy,
                    Decimal::new(69, 0),
                    Decimal::new(0, 4),
                    Decimal::new(0, 2),
                    Decimal::new(0, 2),
                    &degiro_broker,
                )
            ))
        );

        const WATER_NOTE: &str = r#"07/02/2023 WATER INTELLIGENCE PLC GB00BZ973D04 C 880 600,0000 528.000,00 5.928,91 4,90 0,0112
"#;
        assert_eq!(
            DegiroParser::account_note(WATER_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd_opt(2023, 2, 7).unwrap(),
                    CompanyInfo {
                        name: String::from("WATER INTELLIGENCE PLC"),
                        isin: String::from("GB00BZ973D04")
                    },
                    BrokerOperation::Buy,
                    Decimal::new(880, 0),
                    Decimal::new(6000000, 4),
                    Decimal::new(52800000, 2),
                    Decimal::new(490, 2),
                    &degiro_broker,
                )
            ))
        );
    }

    #[test]
    fn degiro_2023_parse_content_test() {
        let degiro_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));
        let parser = DegiroParser::new(INPUT_2023.to_string(), &degiro_broker);
        let (balance_notes, account_notes) = parser.parse_pdf_content().unwrap();

        let bal_notes = vec![
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("BURFORD CAP LD"),
                    isin: String::from("GG00B4L84979"),
                },
                String::from("LSE"),
                Decimal::new(122, 0),
                String::from("GBX"),
                Decimal::new(1_656_0000, 4),
                Decimal::new(2_247_00, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("FACEBOOK INC. - CLASS"),
                    isin: String::from("US30303M1027"),
                },
                String::from("NDQ"),
                Decimal::new(21, 0),
                String::from("USD"),
                Decimal::new(131_0900, 4),
                Decimal::new(2_401_07, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("JD.COM INC. - AMERICA"),
                    isin: String::from("US47215P1066"),
                },
                String::from("NDQ"),
                Decimal::new(140, 0),
                String::from("USD"),
                Decimal::new(20_9300, 4),
                Decimal::new(2555_72, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("GXO LOGISTICS INC. COMMON STOCK"),
                    isin: String::from("US9837931008"),
                },
                String::from("NSY"),
                Decimal::new(41, 0),
                String::from("USD"),
                Decimal::new(57_0400, 4),
                Decimal::new(2039_76, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("MONDO TV"),
                    isin: String::from("IT0001447785"),
                },
                String::from("MIL"),
                Decimal::new(1105, 0),
                String::from("EUR"),
                Decimal::new(1_1940, 4),
                Decimal::new(1319_37, 2),
                &degiro_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("TAPTICA INT LTD"),
                    isin: String::from("IL0011320343"),
                },
                String::from("LSE"),
                Decimal::new(565, 0),
                String::from("GBX"),
                Decimal::new(160_0000, 4),
                Decimal::new(1005_43, 2),
                &degiro_broker,
            ),
        ];

        for (i, item) in bal_notes.iter().enumerate() {
            assert_eq!(*item, balance_notes[i]);
        }
        assert_eq!(bal_notes, balance_notes);

        let acc_notes = vec![
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 10, 31).unwrap(),
                CompanyInfo {
                    name: String::from("BURFORD CAP LD"),
                    isin: String::from("GG00B4L84979"),
                },
                BrokerOperation::Buy,
                Decimal::new(122, 0),
                Decimal::new(1_616_0000, 4),
                Decimal::new(197_152_00, 2),
                Decimal::new(5_28, 2),
                &degiro_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 10, 22).unwrap(),
                CompanyInfo {
                    name: String::from("FACEBOOK INC. - CLASS"),
                    isin: String::from("US30303M1027"),
                },
                BrokerOperation::Buy,
                Decimal::new(21, 0),
                Decimal::new(154_7600, 4),
                Decimal::new(3_249_96, 2),
                Decimal::new(57, 2),
                &degiro_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 10, 22).unwrap(),
                CompanyInfo {
                    name: String::from("JD.COM INC. - AMERICA"),
                    isin: String::from("US47215P1066"),
                },
                BrokerOperation::Buy,
                Decimal::new(140, 0),
                Decimal::new(23_8900, 4),
                Decimal::new(3_344_60, 2),
                Decimal::new(99, 2),
                &degiro_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 11, 23).unwrap(),
                CompanyInfo {
                    name: String::from("MONDO TV"),
                    isin: String::from("IT0001447785"),
                },
                BrokerOperation::Buy,
                Decimal::new(877, 0),
                Decimal::new(1_9000, 4),
                Decimal::new(1_666_30, 2),
                Decimal::new(4_97, 2),
                &degiro_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 11, 23).unwrap(),
                CompanyInfo {
                    name: String::from("MONDO TV"),
                    isin: String::from("IT0001447785"),
                },
                BrokerOperation::Buy,
                Decimal::new(228, 0),
                Decimal::new(1_9000, 4),
                Decimal::new(433_20, 2),
                Decimal::new(25, 2),
                &degiro_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 12, 3).unwrap(),
                CompanyInfo {
                    name: String::from("TAPTICA INT LTD"),
                    isin: String::from("IL0011320343"),
                },
                BrokerOperation::Buy,
                Decimal::new(565, 0),
                Decimal::new(310_0000, 4),
                Decimal::new(175_150_00, 2),
                Decimal::new(5_15, 2),
                &degiro_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2018, 12, 31).unwrap(),
                CompanyInfo {
                    name: String::from("XPO LOGISTICS INC."),
                    isin: String::from("US9837931008"),
                },
                BrokerOperation::Buy,
                Decimal::new(41, 0),
                Decimal::new(56_6000, 4),
                Decimal::new(2_320_60, 2),
                Decimal::new(64, 2),
                &degiro_broker,
            ),
        ];
        for (i, item) in acc_notes.iter().enumerate() {
            assert_eq!(*item, account_notes[i]);
        }

        assert_eq!(acc_notes, account_notes);
    }

    const INPUT_2023: &str = r#"
Sr. John Doe
neverwhere
neverland


Nombre de usuario: ******aaa
 DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam

T +34 91 123 96 78
E clientes@degiro.es
I www.degiro.es

Estimado señor Doe,

Encuentre en el adjunto el Informe Fiscal para el año 2018, con los datos que puede utilizar para
realizar su declaración tributaria. DEGIRO está registrada en la Cámara de ComeArcio holandesa bajo
el número 34342820 y el número de identificación fiscal (TIN) es 820866933. DEGIRO B.V tiene
domicilio social en Rembrandt Tower - 9th floor, Amstelplein 1, 1096 HA Amsterdam.

Le recomendamos contactar con la Agencia Tributaria o con su asesor fiscal si necesita ayuda o
asistencia a la hora de rellenar su declaración. El Informe Fiscal se compone de:

Valor de la Cartera
En el apartado “Valor de la Cartera” podrá encontrar el valor de su cartera a fecha 1 de enero de 2018
y a 31 de diciembre de 2018. Con los datos a 1 de enero puede determinar los rendimientos sobre el
capital durante el año 2018.

Dividendos y otras remuneraciones al accionista
Dividendos y otras remuneraciones recibidas junto a su correspondiente retención.

Relación de ganancias y pérdidas por producto
Contiene la relación de ganancias y pérdidas netas obtenidas de las posiciones cerradas durante el
año 2018.

Certificado de Beneficiario Último Económico
Extracto de posiciones a fecha: 31/12/2018.

Beneficios y pérdidas derivados de la transmisión de elementos patrimoniales
Contiene la relación de transacciones realizadas durante el año 2018, así como las comisiones
asociadas a dichas operaciones y las ganancias o pérdidas derivadas de las posiciones cerradas
durante el año.

Fondos del meArcado monetario
Tenga en cuenta que las tablas de este informe tienen en cuenta las ganancias o pérdidas derivadas
de los movimientos en los fondos del meArcado monetario (FMM). Por ejemplo, en el caso de tener una
ganancia o pérdida en una posición en un FMM, esto se tendrá en cuenta en la tabla
“Ganancias/Comisiones” y/o en la tabla “Distribuciones Fondos del MeArcado Monetario”. En caso de
que recibiera una compensación por parte de DEGIRO por una pérdida de una posición de FMM, esto
se informará en la tabla 'Compensación Fondos del MeArcado Monetario (FMM)'
Las transacciones de FMM se excluyen de la tabla "Beneficios y pérdidas derivadas de la transmisión
de elementos patrimoniales". Esta información se puede encontrar en la pestaña Cuenta de su
Webtrader.

DEGIRO realiza el Informe Fiscal de la manera más rigurosa posible pero no asume responsabilidad
por cualquier posible inexactitud en el mismo. Este informe no es vinculante y de tener cualquier duda
al respecto, le rogamos se ponga en contacto con el Servicio de Atención al Cliente de DEGIRO en
clientes@degiro.es.

Atentamente,

DEGIRO

DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los MeArcados
Holandeses.
Informe Anual 2018 - www.degiro.es 1 / 3

Sr. John Doe
neverwhere
neverland

Nombre de usuario: ******aaa
 DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam

T +34 91 123 96 78
E clientes@degiro.es
I www.degiro.es

Valor de la Cartera 01-01-2018 31-12-2018

Fondos del Mercado Monetario 0.00 EUR 109.63 EUR

Valor en cartera 0.00 EUR 11,568.35 EUR

Valor total 0.00 EUR 11,677.98 EUR

Ganancias (1 de enero - 31 de diciembre)

Ganancia bruta por liquidación de posiciones 0.00 EUR

Pérdida bruta por liquidación de posiciones -0.17 EUR

Total de comisiones por transacción pagadas en el año 2018

Total de comisiones por transacción de las posiciones cerradas 0.00 EUR

Comisiones
 17.85 EUR

Comisión de conectividad con el mercado 10.00 EUR

Interés

Total interés pagado por venta en corto 0.00 EUR

Total interés recibido 0.00 EUR

Total interés pagado 0.00 EUR

Compensación Fondos del Mercado Monetario (FMM)

Compensación total recibida 2018 0.18 EUR

DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los MeArcados
Holandeses.

Informe Anual 2018 - www.degiro.es
2 / 3

Sr. John Doe
neverwhere
neverland


Nombre de usuario: ******aaa
 DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam

T +34 91 123 96 78
E clientes@degiro.es
I www.degiro.es

Dividendos, Cupones y otras remuneraciones
País Producto Ingreso bruto Retenciones a cuenta Ingreso neto

GG 0.00 EUR3.86 EUR 3.86 EURBURFORD CAP LD
 3.86 EUR0.00 EUR3.86 EUR

Distribuciones Fondos del Mercado Monetario

Producto Ingreso neto

No se han abonado distribuciones

Relación de ganancias y pérdidas por producto

Por favor, tenga en cuenta que el resultado de "Ganancias/Pérdidas" incluye las comisiones de compra/venta.

Producto Symbol/ISIN Ganancias / Pérdidas Comisión pagada

Morgan Stanley EUR Liquidity Fund LU0904783973 -0.17 0.00EUR EUR

DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los MeArcados
Holandeses.

Informe Anual 2018 - www.degiro.es
3 / 3


Certificado de Beneficiario Último Económico.

Cliente: Sr. John Doe

johndoeaaaNombre de usuario:

Dirección: neverwhere, neverland

País: España
 31/12/2018Fecha del extracto:

Producto ISIN Bolsa Cantidad Moneda Precio Valor (EUR)

 2.247,00CurrencyCASH & CASH FUND (EUR)
 2.247,001.656,0000GBX122LSEStockBURFORD CAP LD GG00B4L84979
 2.401,07131,0900USD21NDQStockFACEBOOK INC. - CLASS US30303M1027
 2.555,7220,9300USD140NDQStockJD.COM INC. - AMERICA US47215P1066
 2.039,7657,0400USD41NSYStockGXO LOGISTICS INC. COMMON
STOCK US9837931008
 1.319,371,1940EUR1105MILStockMONDO TV IT0001447785
 1.005,43160,0000GBX565LSEStockTAPTICA INT LTD IL0011320343

Amsterdam, 28/01/2019

Este certificado está expedido en la fecha y hora exacta indicadas. Ni Degiro ni Stichting Degiro asumen
ninguna responsabilidad en la expedición o corrección del presente documento.

Beneficios y pérdidas derivadas de la transmisión de elementos patrimoniales

Por favor, tenga en cuenta que el resultado de "Beneficios y pérdidas" no incluye las comisiones de compra/venta.

Fecha Producto Symbol/ISIN Tipo de
orden Cantidad Precio Valor local Valor en EUR Comisión Tipo de
cambio Beneficios y
pérdidas

31/10/2018 BURFORD CAP LD GG00B4L84979 C 122 1.616,0000 197.152,00 2.247,93 5,28 0,0114

22/10/2018 FACEBOOK INC. - CLASS US30303M1027 C 21 154,7600 3.249,96 2.834,62 0,57 0,8722

22/10/2018 JD.COM INC. - AMERICA US47215P1066 C 140 23,8900 3.344,60 2.917,16 0,99 0,8722

23/11/2018 MONDO TV IT0001447785 C 877 1,9000 1.666,30 1.666,30 4,97 1,0000

23/11/2018 MONDO TV IT0001447785 C 228 1,9000 433,20 433,20 0,25 1,0000

03/12/2018 TAPTICA INT LTD IL0011320343 C 565 310,0000 175.150,00 1.962,91 5,15 0,0112

31/12/2018 XPO LOGISTICS INC.  US9837931008 C 41 56,6000 2.320,60 2.024,03 0,64 0,8722

67,00 EURTotal

Informe anual de flatex

Para ayudarle a realizar su declaración de la renta le proveemos con este informe anual ya que dispone de una Cuenta de
Efectivo en flatex asociada a su cuenta de DEGIRO.
"#;
}
