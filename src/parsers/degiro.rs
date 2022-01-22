use std::rc::Rc;

use crate::data::{
    AccountNote, AccountNotes, BalanceNote, BalanceNotes, BrokerInformation, BrokerOperation,
    CompanyInfo,
};

use crate::utils::decimal;

use anyhow::{bail, Result};
use chrono::NaiveDate;
use nom::{
    branch::alt,
    bytes::complete::{is_a, take, take_until},
    character::complete::none_of,
    combinator::{map_res, opt, recognize},
    multi::many0,
    multi::many1,
    sequence::{preceded, terminated, tuple},
};
use nom::{
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, one_of},
    multi::many_m_n,
};
use nom::{
    error::{context, VerboseError},
    IResult,
};

use rust_decimal::prelude::*;

type Res<T, U> = IResult<T, U, VerboseError<T>>;

pub struct DegiroParser {
    content: String,
    broker: Rc<BrokerInformation>,
}

const DEGIRO_BALANCE_HEADER_BEGIN: &str = "\nCASH & CASH FUND (EUR)\n";
const DEGIRO_BALANCE_HEADER_END: &str = "Amsterdam, ";

const DEGIRO_NOTES_HEADER_END: &str = "Informe anual de flatex";

const DEGIRO_NOTES_HEADER_BEGIN: &str = r#"
Fecha
Producto
Symbol/ISIN
Tipo de
orden
Cantidad
Precio
Valor local
Valor en EUR
Comisión
Tipo de
cambio
Beneficios y
pérdidas"#;

impl DegiroParser {
    fn n_to_m_digits<'b>(n: usize, m: usize) -> impl FnMut(&'b str) -> Res<&str, String> {
        move |input| {
            many_m_n(n, m, one_of("0123456789"))(input)
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
        )(input)
    }

    fn earnings_value(input: &str) -> Res<&str, Decimal> {
        context(
            "earnings value",
            map_res(
                recognize(tuple((
                    opt(one_of("+-")),
                    recognize(many1(terminated(one_of("0123456789"), many0(char('.'))))),
                    char(','),
                    recognize(many1(terminated(one_of("0123456789"), many0(char('.'))))),
                ))),
                |out: &str| Decimal::from_str(&decimal::transform_i18n_es_str(out)),
            ),
        )(input)
    }

    fn date_concept(input: &str) -> Res<&str, NaiveDate> {
        context(
            "date concept",
            tuple((
                DegiroParser::n_to_m_digits(1, 2),
                tag("/"),
                DegiroParser::n_to_m_digits(1, 2),
                tag("/"),
                DegiroParser::n_to_m_digits(1, 4),
            )),
        )(input)
        .map(|(next_input, res)| {
            let (day, _, month, _, year) = res;
            (
                next_input,
                NaiveDate::from_ymd(
                    year.parse::<i32>().unwrap(),
                    month.parse::<u32>().unwrap(),
                    day.parse::<u32>().unwrap(),
                ),
            )
        })
    }

    fn broker_operation(input: &str) -> Res<&str, BrokerOperation> {
        context(
            "broker operation",
            alt((tag_no_case("C"), tag_no_case("V"))),
        )(input)
        .map(|(next_input, res)| (next_input, res.into()))
    }

    fn isin(input: &str) -> Res<&str, String> {
        context("isin", many_m_n(12, 12, none_of("\t \n")))(input)
            .map(|(next_input, res)| (next_input, res.iter().collect()))
    }

    fn company_info(input: &str) -> Res<&str, CompanyInfo> {
        context(
            "company info",
            alt((
                tuple((
                    take_until("\n"),
                    char('\n'),
                    take(0usize),
                    take(0usize),
                    terminated(DegiroParser::isin, opt(char('\n'))),
                )),
                tuple((
                    take_until("\n"),
                    char('\n'),
                    take_until("\n"),
                    take(1usize),
                    terminated(DegiroParser::isin, opt(char('\n'))),
                )),
                tuple((
                    take_until("\n"),
                    char('\n'),
                    recognize(tuple((
                        take_until("\n"),
                        char('\n'),
                        take_until("\n"),
                        char('\n'),
                    ))),
                    take(0usize),
                    terminated(DegiroParser::isin, opt(char('\n'))),
                )),
            )),
        )(input)
        .map(|(next_input, res)| {
            let (company_name, _, extra, _, isin) = res;
            let mut name = company_name.to_string();

            if !extra.is_empty() {
                name.push(' ');
                name.push_str(&str::replace(extra, "\n", ""));
            }

            (next_input, CompanyInfo { name, isin })
        })
    }

    fn account_note<'a>(
        input: &'a str,
        broker: &Rc<BrokerInformation>,
    ) -> Res<&'a str, AccountNote> {
        context(
            "account note",
            tuple((
                DegiroParser::date_concept,
                char('\n'),
                DegiroParser::company_info,
                DegiroParser::broker_operation,
                char('\n'),
                DegiroParser::decimal_value,
                char('\n'),
                DegiroParser::decimal_value,
                char('\n'),
                DegiroParser::decimal_value,
                char('\n'),
                DegiroParser::decimal_value,
                char('\n'),
                DegiroParser::decimal_value,
                char('\n'),
                DegiroParser::decimal_value,
                opt(tuple((char('\n'), DegiroParser::earnings_value))),
            )),
        )(input)
        .map(|(next_input, res)| {
            let (
                date,
                _,
                company,
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
                earnings_opt,
            ) = res;
            let _earnings: Decimal = earnings_opt.unwrap_or_else(|| ('\n', Decimal::new(0, 2))).1;

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
        broker: &Rc<BrokerInformation>,
    ) -> Res<&'a str, BalanceNote> {
        context(
            "balance note",
            tuple((
                DegiroParser::decimal_value, // value in euro
                char('\n'),
                DegiroParser::decimal_value, // price
                char('\n'),
                take(3usize), // currency
                char('\n'),
                DegiroParser::decimal_value, // quantity
                char('\n'),
                take_until("\n"), // market
                char('\n'),
                take_until("\n"), // product type: Stock
                char('\n'),
                DegiroParser::company_info, // company info
            )),
        )(input)
        .map(|(next_input, res)| {
            let (
                value_in_euro,
                _,
                price,
                _,
                currency,
                _,
                quantity,
                _,
                market,
                _,
                _product_type,
                _,
                company,
            ) = res;

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
        broker: &Rc<BrokerInformation>,
    ) -> Res<&'a str, AccountNotes> {
        context(
            "account notes",
            many0(preceded(char('\n'), |x| {
                DegiroParser::account_note(x, broker)
            })),
        )(input)
    }

    fn balance_notes<'a>(
        input: &'a str,
        broker: &Rc<BrokerInformation>,
    ) -> Res<&'a str, BalanceNotes> {
        context(
            "balance notes",
            many0(|x| DegiroParser::balance_note(x, broker)),
        )(input)
        .map(|(next_input, res)| (next_input, res))
    }

    fn parse_account_notes(&self, notes: &str) -> Result<AccountNotes> {
        log::debug!("account notes:-{}-", notes);
        let notes = match DegiroParser::account_notes(notes, &self.broker) {
            Ok((_, notes)) => notes,
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

        let indexes: Vec<_> = self
            .content
            .match_indices(DEGIRO_NOTES_HEADER_BEGIN)
            .collect();

        for i in 0..indexes.len() {
            let header_begin = indexes.get(i).unwrap().0 + DEGIRO_NOTES_HEADER_BEGIN.len();
            let header_end = if i < indexes.len() - 1 {
                indexes.get(i + 1).unwrap().0
            } else {
                match self.content.find(DEGIRO_NOTES_HEADER_END) {
                    Some(end) => end - 1,
                    None => self.content.len(),
                }
            };
            result.extend(self.parse_account_notes(&self.content[header_begin..header_end])?);
        }

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
            result.extend(self.parse_balance_notes(&self.content[header_begin..header_end])?);
        }

        Ok(result)
    }

    pub fn new(content: String, broker: &Rc<BrokerInformation>) -> Self {
        Self {
            content,
            broker: Rc::clone(broker),
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
    use nom::{
        error::{ErrorKind, VerboseError, VerboseErrorKind},
        Err as NomErr,
    };

    #[test]
    fn broker_operation_test() {
        assert_eq!(
            DegiroParser::broker_operation("C\n"),
            Ok(("\n", BrokerOperation::Buy))
        );
        assert_eq!(
            DegiroParser::broker_operation("V\n"),
            Ok(("\n", BrokerOperation::Sell))
        );
        assert_eq!(
            DegiroParser::broker_operation("Z\n"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("Z\n", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("Z\n", VerboseErrorKind::Nom(ErrorKind::Alt)),
                    ("Z\n", VerboseErrorKind::Context("broker operation")),
                ]
            }))
        );
    }

    #[test]
    fn date_concept_test() {
        assert_eq!(
            DegiroParser::date_concept("03/11/2018\n"),
            Ok(("\n", NaiveDate::from_ymd(2018, 11, 3)))
        );
        assert_eq!(
            DegiroParser::date_concept("31/10/2018\n"),
            Ok(("\n", NaiveDate::from_ymd(2018, 10, 31)))
        );
        assert_eq!(
            DegiroParser::date_concept("32_23_2020\n"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("_23_2020\n", VerboseErrorKind::Nom(ErrorKind::Tag)),
                    ("32_23_2020\n", VerboseErrorKind::Context("date concept")),
                ]
            }))
        );
    }

    #[test]
    fn isin_test() {
        assert_eq!(
            DegiroParser::isin("GG00B4L84979\n"),
            Ok(("\n", String::from("GG00B4L84979")))
        );
        assert_eq!(
            DegiroParser::isin("IL0011320343\n"),
            Ok(("\n", String::from("IL0011320343")))
        );
        assert_eq!(
            DegiroParser::isin("US342342\n"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("\n", VerboseErrorKind::Nom(ErrorKind::NoneOf)),
                    ("\n", VerboseErrorKind::Nom(ErrorKind::ManyMN)),
                    ("US342342\n", VerboseErrorKind::Context("isin")),
                ]
            }))
        );
    }

    #[test]
    fn company_info_test() {
        let compmany_info_burford: &str = r#"BURFORD CAP LD
GG00B4L84979
"#;

        assert_eq!(
            DegiroParser::company_info(compmany_info_burford),
            Ok((
                "",
                CompanyInfo {
                    name: String::from("BURFORD CAP LD"),
                    isin: String::from("GG00B4L84979"),
                }
            ))
        );

        let compmany_info_gxo: &str = r#"GXO LOGISTICS INC.
COMMON STOCK WHEN-
ISSUED
US36262G1013
"#;

        assert_eq!(
            DegiroParser::company_info(compmany_info_gxo),
            Ok((
                "",
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
            DegiroParser::decimal_value("1.000,03\n"),
            Ok(("\n", Decimal::new(1_000_03, 2)))
        );
        assert_eq!(
            DegiroParser::decimal_value("300\n"),
            Ok(("\n", Decimal::new(300, 0)))
        );
        assert_eq!(
            DegiroParser::decimal_value("0,9030\n"),
            Ok(("\n", Decimal::new(9030, 4)))
        );
        assert_eq!(
            DegiroParser::decimal_value("a234,23\n"),
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("a234,23\n", VerboseErrorKind::Nom(ErrorKind::OneOf)),
                    ("a234,23\n", VerboseErrorKind::Nom(ErrorKind::Many1)),
                    ("a234,23\n", VerboseErrorKind::Context("decimal value")),
                ]
            }))
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
            Err(NomErr::Error(VerboseError {
                errors: vec![
                    ("\n", VerboseErrorKind::Char(',')),
                    ("1234\n", VerboseErrorKind::Context("earnings value")),
                ]
            }))
        );
    }

    #[test]
    fn balance_note_test() {
        let degiro_broker: Rc<BrokerInformation> = Rc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));

        const BURFORD_NOTE: &str = r#"3.889,94
712,0000
GBX
463
LSE
Stock
BURFORD CAP LD
GG00B4L84979"#;

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
                    Decimal::new(463, 0),
                    String::from("GBX"),
                    Decimal::new(712_0000, 4),
                    Decimal::new(3889_94, 2),
                    &degiro_broker,
                )
            ))
        );
    }

    #[test]
    fn account_note_test() {
        let degiro_broker: Rc<BrokerInformation> = Rc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));
        const BURFORD_NOTE: &str = r#"31/10/2018
BURFORD CAP LD
GG00B4L84979
C
122
1.616,0000
197.152,00
2.247,93
5,28
0,0114"#;
        assert_eq!(
            DegiroParser::account_note(BURFORD_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd(2018, 10, 31),
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

        const BURFORD_LONG_NOTE: &str = r#"31/10/2018
BURFORD
CAP LD
GG00B4L84979
C
122
1.616,0000
197.152,00
2.247,93
5,28
0,0114"#;

        assert_eq!(
            DegiroParser::account_note(BURFORD_LONG_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd(2018, 10, 31),
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

        const GXO_LONG_NOTE: &str = r#"02/08/2021
GXO LOGISTICS INC.
COMMON STOCK WHEN-
ISSUED
US36262G1013
C
69
0,0000
0,00
0,00
0,00
0,8423"#;

        assert_eq!(
            DegiroParser::account_note(GXO_LONG_NOTE, &degiro_broker),
            Ok((
                "",
                AccountNote::new(
                    NaiveDate::from_ymd(2021, 8, 2),
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
    }

    #[test]
    fn degiro_2018_parse_content_test() {
        let degiro_broker: Rc<BrokerInformation> = Rc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));
        let parser = DegiroParser::new(INPUT_2018.to_string(), &degiro_broker);
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
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("XPO LOGISTICS INC."),
                    isin: String::from("US9837931008"),
                },
                String::from("NSY"),
                Decimal::new(41, 0),
                String::from("USD"),
                Decimal::new(57_0400, 4),
                Decimal::new(2039_76, 2),
                &degiro_broker,
            ),
        ];

        assert_eq!(bal_notes, balance_notes);

        let acc_notes = vec![
            AccountNote::new(
                NaiveDate::from_ymd(2018, 10, 31),
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
                NaiveDate::from_ymd(2018, 10, 22),
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
                NaiveDate::from_ymd(2018, 10, 22),
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
                NaiveDate::from_ymd(2018, 11, 23),
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
                NaiveDate::from_ymd(2018, 11, 23),
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
                NaiveDate::from_ymd(2018, 12, 3),
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
                NaiveDate::from_ymd(2018, 12, 31),
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
        assert_eq!(acc_notes, account_notes);
    }

    const INPUT_2018: &str = r#"
Sr. John Doe
neverwhere
neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam

T
 +34 91 123 96 78
E
 
clientes@degiro.es
I
 
www.degiro.es

Estimado señor Doe,

Encuentre en el adjunto el Informe Fiscal para el año 2018, con los datos que puede utilizar para
realizar su declaración tributaria. DEGIRO está registrada en la Cámara de Comercio holandesa bajo
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

Fondos del mercado monetario
Tenga en cuenta que las tablas de este informe tienen en cuenta las ganancias o pérdidas derivadas
de los movimientos en los fondos del mercado monetario (FMM). Por ejemplo, en el caso de tener una
ganancia o pérdida en una posición en un FMM, esto se tendrá en cuenta en la tabla
“Ganancias/Comisiones” y/o en la tabla “Distribuciones Fondos del Mercado Monetario”. En caso de
que recibiera una compensación por parte de DEGIRO por una pérdida de una posición de FMM, esto
se informará en la tabla 'Compensación Fondos del Mercado Monetario (FMM)'
Las transacciones de FMM se excluyen de la tabla "Beneficios y pérdidas derivadas de la transmisión
de elementos patrimoniales". Esta información se puede encontrar en la pestaña Cuenta de su
Webtrader.

DEGIRO realiza el Informe Fiscal de la manera más rigurosa posible pero no asume responsabilidad
por cualquier posible inexactitud en el mismo. Este informe no es vinculante y de tener cualquier duda
al respecto, le rogamos se ponga en contacto con el Servicio de Atención al Cliente de DEGIRO en
clientes@degiro.es
.

Atentamente,

DEGIRO


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2018 - 
www.degiro.es
1
/ 3

Sr. John Doe
neverwhere
neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam

T
 +34 91 123 96 78
E
 
clientes@degiro.es
I
 
www.degiro.es
Valor de la Cartera
01-01-2018
31-12-2018
Fondos del Mercado Monetario
0.00 EUR
109.63 EUR
Valor en cartera
0.00 EUR
11,568.35 EUR
Valor total
0.00 EUR
11,677.98 EUR
Ganancias (1 de enero - 31 de diciembre)
Ganancia bruta por liquidación de posiciones
0.00 EUR
Pérdida bruta por liquidación de posiciones
-0.17 EUR
Total de comisiones por transacción pagadas en el año 2018
Total de comisiones por transacción de las posiciones cerradas
0.00 EUR
Comisiones
17.85 EUR
Comisión de conectividad con el mercado
10.00 EUR
Interés
Total interés pagado por venta en corto
0.00 EUR
Total interés recibido
0.00 EUR
Total interés pagado
0.00 EUR
Compensación Fondos del Mercado Monetario (FMM)
Compensación total recibida 2018
0.18
EUR


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2018 - 
www.degiro.es
2
/ 3

Sr. John Doe
neverwhere
neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam

T
 +34 91 123 96 78
E
 
clientes@degiro.es
I
 
www.degiro.es
Dividendos, Cupones y otras remuneraciones
País
Producto
Ingreso bruto
Retenciones a cuenta
Ingreso neto
GG
0.00 EUR
3.86 EUR
3.86 EUR
BURFORD CAP LD
3.86 EUR
0.00 EUR
3.86 EUR
Distribuciones Fondos del Mercado Monetario
Producto
Ingreso neto
No se han abonado distribuciones
Relación de ganancias y pérdidas por producto
Por favor, tenga en cuenta que el resultado de "Ganancias/Pérdidas" incluye las comisiones de compra/venta.
Producto
Symbol/ISIN
Ganancias / Pérdidas
Comisión pagada
Morgan Stanley EUR Liquidity Fund
LU0904783973
-0.17
0.00
EUR
EUR


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2018 - 
www.degiro.es
3
/ 3



Certificado de Beneficiario Último Económico.
Cliente:
Sr. John Doe
johndoeaaa
Nombre de usuario:
Dirección:
neverwhere, neverland
País:
España
31/12/2018
Fecha del extracto:
Producto
ISIN
Bolsa
Cantidad
Moneda
Precio
Valor (EUR)
CASH & CASH FUND (EUR)
2.247,00
1.656,0000
GBX
122
LSE
Stock
BURFORD CAP LD
GG00B4L84979
2.401,07
131,0900
USD
21
NDQ
Stock
FACEBOOK INC. - CLASS
US30303M1027
2.555,72
20,9300
USD
140
NDQ
Stock
JD.COM INC. - AMERICA
US47215P1066
1.319,37
1,1940
EUR
1105
MIL
Stock
MONDO TV
IT0001447785
1.005,43
160,0000
GBX
565
LSE
Stock
TAPTICA INT LTD
IL0011320343
2.039,76
57,0400
USD
41
NSY
Stock
XPO LOGISTICS INC.
US9837931008



Amsterdam, 28/01/2019
Este certificado está expedido en la fecha y hora exacta indicadas. Ni Degiro ni Stichting Degiro asumen
ninguna responsabilidad en la expedición o corrección del presente documento.

Beneficios y pérdidas derivadas de la transmisión de elementos patrimoniales
Por favor, tenga en cuenta que el resultado de "Beneficios y pérdidas" no incluye las comisiones de compra/venta.
Fecha
Producto
Symbol/ISIN
Tipo de
orden
Cantidad
Precio
Valor local
Valor en EUR
Comisión
Tipo de
cambio
Beneficios y
pérdidas
31/10/2018
BURFORD CAP LD
GG00B4L84979
C
122
1.616,0000
197.152,00
2.247,93
5,28
0,0114
22/10/2018
FACEBOOK INC. - CLASS
US30303M1027
C
21
154,7600
3.249,96
2.834,62
0,57
0,8722
22/10/2018
JD.COM INC. - AMERICA
US47215P1066
C
140
23,8900
3.344,60
2.917,16
0,99
0,8722
23/11/2018
MONDO TV
IT0001447785
C
877
1,9000
1.666,30
1.666,30
4,97
1,0000
23/11/2018
MONDO TV
IT0001447785
C
228
1,9000
433,20
433,20
0,25
1,0000
03/12/2018
TAPTICA INT LTD
IL0011320343
C
565
310,0000
175.150,00
1.962,91
5,15
0,0112
31/12/2018
XPO LOGISTICS INC.
US9837931008
C
41
56,6000
2.320,60
2.024,03
0,64
0,8722"#;
}
