use crate::account_notes::{AccountNote, AccountNotes, BrokerOperation, CompanyInfo};

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
}

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
                |out: &str| {
                    
                    Decimal::from_str(&str::replace(&str::replace(&out, ".", ""),",", "."))
            },
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
                |out: &str| Decimal::from_str(&str::replace(&str::replace(&out, ".", ""),",", ".")),
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
        context(
            "isin",
            many_m_n(12, 12, none_of("\n")),
        )(input)
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
                    terminated(DegiroParser::isin, char('\n')),
                )),
                tuple((
                    take_until("\n"),
                    char('\n'),
                    take_until("\n"),
                    take(1usize),
                    terminated(DegiroParser::isin, char('\n')),
                )),
            )),
        )(input)
        .map(|(next_input, res)| {
            let (company_name, _, extra, _, isin) = res;
            let mut name = company_name.to_string();

            if !extra.is_empty() {
                name.push(' ');
                name.push_str(extra);
            }

            (next_input, CompanyInfo { name, isin })
        })
    }

    fn account_note(input: &str) -> Res<&str, AccountNote> {
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
                value_in_euro,
                _,
                commision,
                _,
                exchange_rate,
                earnings_opt,
            ) = res;
            let earnings: Decimal = earnings_opt.unwrap_or_else(|| ('\n', Decimal::new(0, 2))).1;

            (
                next_input,
                AccountNote::new(
                    date,
                    company,
                    operation,
                    quantity,
                    price,
                    value,
                    value_in_euro,
                    commision,
                    exchange_rate,
                    earnings,
                ),
            )
        })
    }

    fn account_notes(input: &str) -> Res<&str, AccountNotes> {
        context(
            "account notes",
            many0(preceded(char('\n'), DegiroParser::account_note)),
        )(input)
    }

    fn parse_account_notes(&self, notes: &str) -> Result<AccountNotes> {
        log::debug!("-{}-", notes);
        let notes = match DegiroParser::account_notes(notes) {
            Ok((_, notes)) => notes,
            Err(err) => {
                bail!("Unable to parse account notes: {}", err);
            }
        };

        Ok(notes)
    }

    pub fn new(content: String) -> DegiroParser {
        DegiroParser { content }
    }

    pub fn parse_pdf_content(&self) -> Result<AccountNotes> {
        let mut result = vec![];

        let indexes: Vec<_> = self.content.match_indices(DEGIRO_NOTES_HEADER_BEGIN).collect();

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
}

#[cfg(test)]
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
            Ok(("\n", Decimal::new(09030, 4)))
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
            Ok(("\n", Decimal::new(09030, 4)))
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
    fn account_note_test() {
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
            DegiroParser::account_note(BURFORD_NOTE),
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
                    Decimal::new(2_247_93, 2),
                    Decimal::new(5_28, 2),
                    Decimal::new(0_0114, 4),
                    Decimal::new(0, 2),
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
            DegiroParser::account_note(BURFORD_LONG_NOTE),
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
                    Decimal::new(2_247_93, 2),
                    Decimal::new(5_28, 2),
                    Decimal::new(0_0114, 4),
                    Decimal::new(0, 2),
                )
            ))
        );
    }

    #[test]
    fn degiro_2018_parse_content_test() {
        let parser = DegiroParser::new(INPUT_2018.to_string());
        let account_notes = parser.parse_pdf_content().unwrap();
        let notes = vec![
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
                Decimal::new(2_247_93, 2),
                Decimal::new(5_28, 2),
                Decimal::new(0_0114, 4),
                Decimal::new(0, 2),
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
                Decimal::new(2_834_62, 2),
                Decimal::new(0_57, 2),
                Decimal::new(0_8722, 4),
                Decimal::new(0, 2),
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
                Decimal::new(2_917_16, 2),
                Decimal::new(0_99, 2),
                Decimal::new(0_8722, 4),
                Decimal::new(0, 2),
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
                Decimal::new(1_666_30, 2),
                Decimal::new(4_97, 2),
                Decimal::new(1_0000, 4),
                Decimal::new(0, 2),
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
                Decimal::new(433_20, 2),
                Decimal::new(0_25, 2),
                Decimal::new(1_0000, 4),
                Decimal::new(0, 2),
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
                Decimal::new(1_962_91, 2),
                Decimal::new(5_15, 2),
                Decimal::new(0_0112, 4),
                Decimal::new(0, 2),
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
                Decimal::new(2_024_03, 2),
                Decimal::new(0_64, 2),
                Decimal::new(0_8722, 4),
                Decimal::new(0, 2),
            ),
        ];
        assert_eq!(notes, account_notes);
    }

    #[test]
    fn degiro_2020_parse_content_test() {
        let parser = DegiroParser::new(INPUT_2020.to_string());
        let account_notes = parser.parse_pdf_content().unwrap();
        let notes = vec![
            AccountNote::new(
                NaiveDate::from_ymd(2020,09,15),
                CompanyInfo {
                    name: String::from("CTT SYSTEMS"),
                    isin: String::from("SE0000418923"),
                },
                BrokerOperation::Buy,
                Decimal::new(50, 0),
                Decimal::new(128_8000, 4),
                Decimal::new(6440_00, 2),
                Decimal::new(618_24, 2),
                Decimal::new(4_31, 2),
                Decimal::new(0_0960, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020,09,15),
                CompanyInfo {
                    name: String::from("CTT SYSTEMS"),
                    isin: String::from("SE0000418923"),
                },
                BrokerOperation::Buy,
                Decimal::new(13, 0),
                Decimal::new(129_0000, 4),
                Decimal::new(1677_00, 2),
                Decimal::new(160_99, 2),
                Decimal::new(0_08, 2),
                Decimal::new(0_0960, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 09, 24),
                CompanyInfo {
                    name: String::from("CTT SYSTEMS"),
                    isin: String::from("SE0000418923"),
                },
                BrokerOperation::Buy,
                Decimal::new(142, 0),
                Decimal::new(118_8000, 4),
                Decimal::new(16869_60, 2),
                Decimal::new(1589_12, 2),
                Decimal::new(4_79, 2),
                Decimal::new(0_0942, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 06, 11),
                CompanyInfo {
                    name: String::from("CVD EQUIPMENT CORPORAT"),
                    isin: String::from("US1266011030"),
                },
                BrokerOperation::Buy,
                Decimal::new(700, 0),
                Decimal::new(3_4800, 4),
                Decimal::new(2436_00, 2),
                Decimal::new(2156_10, 2),
                Decimal::new(2_97, 2),
                Decimal::new(0_8851, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 06, 11),
                CompanyInfo {
                    name: String::from("CVD EQUIPMENT CORPORAT"),
                    isin: String::from("US1266011030"),
                },
                BrokerOperation::Buy,
                Decimal::new(300, 0),
                Decimal::new(3_4800, 4),
                Decimal::new(1044_00, 2),
                Decimal::new(924_04, 2),
                Decimal::new(1_06, 2),
                Decimal::new(0_8851, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new (
                NaiveDate::from_ymd(2020,01,13),
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028"),
                },
                BrokerOperation::Buy,
                Decimal::new(100, 0),
                Decimal::new(25_3300, 4),
                Decimal::new(2533_00, 2),
                Decimal::new(2274_89, 2),
                Decimal::new(0_86, 2),
                Decimal::new(0_8981, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020,01,13),
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028")
                },
                BrokerOperation::Buy,
                Decimal::new(40, 0),
                Decimal::new(25_3700, 4),
                Decimal::new(1014_80, 2),
                Decimal::new(911_39, 2),
                Decimal::new(0_14, 2),
                Decimal::new(0_8981, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 02, 12),
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028"),
                },
                BrokerOperation::Buy,
                Decimal::new(100, 0),
                Decimal::new(24_3550, 4),
                Decimal::new(2435_50, 2),
                Decimal::new(2239_69, 2),
                Decimal::new(0_87, 2),
                Decimal::new(0_9196, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 02, 12),
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028"),
                },
                BrokerOperation::Buy,
                Decimal::new(38, 0),
                Decimal::new(24_4150, 4),
                Decimal::new(927_77, 2),
                Decimal::new(853_18, 2),
                Decimal::new(0_14, 2),
                Decimal::new(0_9196, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 03, 05),
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028"),
                },
                BrokerOperation::Buy,
                Decimal::new(40, 0),
                Decimal::new(21_0000, 4),
                Decimal::new(840_00, 2),
                Decimal::new(747_60, 2),
                Decimal::new(0_64, 2),
                Decimal::new(0_8900, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 03, 05),
                CompanyInfo {
                    name: String::from("EVI INDUSTRIES INC"),
                    isin: String::from("US26929N1028"),
                },
                BrokerOperation::Buy,
                Decimal::new(40, 0),
                Decimal::new(21_0000, 4),
                Decimal::new(840_00, 2),
                Decimal::new(747_60, 2),
                Decimal::new(0_14, 2),
                Decimal::new(0_8900, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 08, 11),
                CompanyInfo {
                    name: String::from("FINANCIERE ODET"),
                    isin: String::from("FR0000062234"),
                },
                BrokerOperation::Buy,
                Decimal::new(3, 0),
                Decimal::new(680_0000, 4),
                Decimal::new(2040_00, 2),
                Decimal::new(2040_00, 2),
                Decimal::new(5_02, 2),
                Decimal::new(1_0000, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 03, 06),
                CompanyInfo {
                    name: String::from("FLOWTRADERS"),
                    isin: String::from("NL0011279492"),
                },
                BrokerOperation::Buy,
                Decimal::new(70, 0),
                Decimal::new(21_4400, 4),
                Decimal::new(1500_80, 2),
                Decimal::new(1500_80, 2),
                Decimal::new(4_75, 2),
                Decimal::new(1_0000, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 06, 11),
                CompanyInfo {
                    name: String::from("FLOWTRADERS"),
                    isin: String::from("NL0011279492"),
                },
                BrokerOperation::Sell,
                Decimal::new(70, 0),
                Decimal::new(30_9400, 4),
                Decimal::new(2165_80, 2),
                Decimal::new(2165_80, 2),
                Decimal::new(5_08, 2),
                Decimal::new(1_0000, 4),
                Decimal::new(665_0000, 4)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 07, 07),
                CompanyInfo {
                    name: String::from("GENUS"),
                    isin: String::from("GB0002074580"),
                },
                BrokerOperation::Buy,
                Decimal::new(50, 0),
                Decimal::new(3520_0000, 4),
                Decimal::new(176000_00, 2),
                Decimal::new(1958_00, 2),
                Decimal::new(4_97, 2),
                Decimal::new(0_0111, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 05, 20),
                CompanyInfo {
                    name: String::from("GEORGIA CAPITAL"),
                    isin: String::from("GB00BF4HYV08"),
                },
                BrokerOperation::Buy,
                Decimal::new(355, 0),
                Decimal::new(447_5000, 4),
                Decimal::new(158862_50, 2),
                Decimal::new(1771_63, 2),
                Decimal::new(4_89, 2),
                Decimal::new(0_0112, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 05, 20),
                CompanyInfo {
                    name: String::from("GEORGIA CAPITAL"),
                    isin: String::from("GB00BF4HYV08"),
                },
                BrokerOperation::Buy,
                Decimal::new(434, 0),
                Decimal::new(447_5000, 4),
                Decimal::new(194215_00, 2),
                Decimal::new(2165_89, 2),
                Decimal::new(1_09, 2),
                Decimal::new(0_0112, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 05, 20),
                CompanyInfo {
                    name: String::from("GEORGIA CAPITAL"),
                    isin: String::from("GB00BF4HYV08")
                },
                BrokerOperation::Buy,
                Decimal::new(10, 0),
                Decimal::new(447_5000, 4),
                Decimal::new(4475_00, 2),
                Decimal::new(49_91, 2),
                Decimal::new(0_03, 2),
                Decimal::new(0_0112, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 05, 20),
                CompanyInfo {
                    name: String::from("GEORGIA CAPITAL"),
                    isin: String::from("GB00BF4HYV08")
                },
                BrokerOperation::Buy,
                Decimal::new(1, 0),
                Decimal::new(447_5000, 4),
                Decimal::new(447_50, 2),
                Decimal::new(4_99, 2),
                Decimal::new(0_00, 2),
                Decimal::new(0_0112, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 05, 19),
                CompanyInfo {
                    name: String::from("GRAVITY CO. LTD. - AM"),
                    isin: String::from("US38911N2062")
                },
                BrokerOperation::Sell,
                Decimal::new(100, 0),
                Decimal::new(42_0400, 4),
                Decimal::new(4204_00, 2),
                Decimal::new(3848_76, 2),
                Decimal::new(0_87, 2),
                Decimal::new(0_9155, 4),
                Decimal::new(832_6120, 4)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 05, 19), 
                CompanyInfo {
                    name: String::from("GRAVITY CO. LTD. - AM"),
                    isin: String::from("US38911N2062")
                },
                BrokerOperation::Sell,
                Decimal::new(2, 0),
                Decimal::new(42_0100, 4),
                Decimal::new(84_02, 2),
                Decimal::new(76_92, 2),
                Decimal::new(0_01, 2),
                Decimal::new(0_9155, 4),
                Decimal::new(16_5973, 4)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 02, 18),
                CompanyInfo {
                    name: String::from("INTER RAO LIETUVA AB"),
                    isin: String::from("LT0000128621")
                },
                BrokerOperation::Buy,
                Decimal::new(695, 0),
                Decimal::new(20_6000, 4),
                Decimal::new(14317_00, 2),
                Decimal::new(3354_47, 2),
                Decimal::new(10_37, 2),
                Decimal::new(0_2343, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 02, 18),
                CompanyInfo {
                    name: String::from("INTER RAO LIETUVA AB"),
                    isin: String::from("LT0000128621")
                },
                BrokerOperation::Buy,
                Decimal::new(49, 0),
                Decimal::new(20_7000, 4),
                Decimal::new(1014_30, 2),
                Decimal::new(237_65, 2),
                Decimal::new(0_38, 2),
                Decimal::new(0_2343, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 02, 18),
                CompanyInfo {
                    name: String::from("INTER RAO LIETUVA AB"),
                    isin: String::from("LT0000128621")
                },
                BrokerOperation::Buy,
                Decimal::new(256, 0),
                Decimal::new(20_7000, 4),
                Decimal::new(5299_20, 2),
                Decimal::new(1241_60, 2),
                Decimal::new(1_99, 2),
                Decimal::new(0_2343, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 01, 17),
                CompanyInfo {
                    name: String::from("KEYWORDS STUDIO"),
                    isin: String::from("GB00BBQ38507")
                },
                BrokerOperation::Buy,
                Decimal::new(25, 0),
                Decimal::new(1544_0000, 4),
                Decimal::new(38600_00, 2),
                Decimal::new(452_86, 2),
                Decimal::new(4_26, 2),
                Decimal::new(0_0117, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 01, 17),
                CompanyInfo {
                    name: String::from("KEYWORDS STUDIO"),
                    isin: String::from("GB00BBQ38507")
                },
                BrokerOperation::Buy,
                Decimal::new(105, 0),
                Decimal::new(1544_0000, 4),
                Decimal::new(162120_00, 2),
                Decimal::new(1901_99, 2),
                Decimal::new(1_11, 2),
                Decimal::new(0_0117, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 01, 17),
                CompanyInfo {
                    name: String::from("MONDO TV"),
                    isin: String::from("IT0001447785")
                },
                BrokerOperation::Sell,
                Decimal::new(1105, 0),
                Decimal::new(2_2560, 4),
                Decimal::new(2492_88, 2),
                Decimal::new(2492_88, 2),
                Decimal::new(5_45, 2),
                Decimal::new(1_0000, 4),
                Decimal::new(393_3800, 4)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 04, 01),
                CompanyInfo {
                    name: String::from("Okeanis Eco Tankers Corp"),
                    isin: String::from("MHY641771016"),
                },
                BrokerOperation::Buy,
                Decimal::new(59, 0),
                Decimal::new(74_0000, 4),
                Decimal::new(4366_00, 2),
                Decimal::new(380_72, 2),
                Decimal::new(4_19, 2),
                Decimal::new(0_0872, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 04, 01),
                CompanyInfo {
                    name: String::from("Okeanis Eco Tankers Corp"),
                    isin: String::from("MHY641771016")
                },
                BrokerOperation::Buy,
                Decimal::new(25, 0),
                Decimal::new(74_0000, 4),
                Decimal::new(1850_00, 2),
                Decimal::new(161_32, 2),
                Decimal::new(0_08, 2),
                Decimal::new(0_0872, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 04, 01),
                CompanyInfo {
                    name: String::from("Okeanis Eco Tankers Corp"),
                    isin: String::from("MHY641771016")
                },
                BrokerOperation::Buy,
                Decimal::new(346, 0),
                Decimal::new(74_0000, 4),
                Decimal::new(25604_00, 2),
                Decimal::new(2232_67, 2),
                Decimal::new(1_11, 2),
                Decimal::new(0_0872, 4),
                Decimal::new(0_00, 2)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 07, 06),
                CompanyInfo {
                    name: String::from("ROCKROSE ENERGY"),
                    isin: String::from("GB00BYNFCH09")
                },
                BrokerOperation::Sell,
                Decimal::new(216, 0),
                Decimal::new(1830_0000, 4),
                Decimal::new(395280_00, 2),
                Decimal::new(4366_26, 2),
                Decimal::new(6_19, 2),
                Decimal::new(0_0110, 4),
                Decimal::new(-80_3072, 4)),
            AccountNote::new(
                NaiveDate::from_ymd(2020, 01, 13),
                CompanyInfo {
                    name: String::from("SHAKE SHACK INC. CLAS"),
                    isin: String::from("US8190471016")
                },
                BrokerOperation::Buy,
                Decimal::new(34, 0),
                Decimal::new(60_6300, 4),
                Decimal::new(2061_42, 2),
                Decimal::new(1851_36, 2),
                Decimal::new(0_62, 2),
                Decimal::new(0_8981, 4),
                Decimal::new(0_00, 2)),
        ];
        assert_eq!(notes, account_notes);
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
109.63
BURFORD CAP LD
GG00B4L84979
LSE
122
1,656.0000
2,247.00
GBX
FACEBOOK INC. - CLASS
US30303M1027
NDQ
21
131.0900
2,401.07
USD
JD.COM INC. - AMERICA
US47215P1066
NDQ
140
20.9300
2,555.72
USD
MONDO TV
IT0001447785
MIL
1105
1.1940
1,319.37
EUR
TAPTICA INT LTD
IL0011320343
LSE
565
160.0000
1,005.43
GBX
XPO LOGISTICS INC.
US9837931008
NSY
41
57.0400
2,039.76
USD
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

    const INPUT_2020: &str = r#" 
Sr. John Doe
neverwhere 8
8888 neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam


E
 
clientes@degiro.es
I
 
www.degiro.es

Estimado señor Doe,

Encuentre en el adjunto el Informe Fiscal para el año 2020, con los datos que puede utilizar para realizar su declaración
tributaria. DEGIRO está registrada en la Cámara de Comercio holandesa bajo el número 34342820 y el número de
identificación fiscal (TIN) es 820866933. DEGIRO B.V tiene domicilio social en Rembrandt Tower - 9th floor, Amstelplein 1, 1096
HA Amsterdam.

Le recomendamos contactar con la Agencia Tributaria o con su asesor fiscal si necesita ayuda o asistencia a la hora de rellenar
su declaración. El Informe Fiscal se compone de:

Valor de la Cartera
En el apartado “Valor de la Cartera” podrá encontrar el valor de su cartera a fecha 1 de enero de 2020 y a 31 de diciembre de
2020. Con los datos a 1 de enero puede determinar los rendimientos sobre el capital durante el año 2020.

Dividendos y otras remuneraciones al accionista
Dividendos y otras remuneraciones recibidas junto a su correspondiente retención.

Relación de ganancias y pérdidas por producto
Contiene la relación de ganancias y pérdidas netas obtenidas de las posiciones cerradas durante el año 2020. El cambio de
divisa utilizado para los cálculos es el tipo de cambio a final del día de la fecha de cada transacción.

Certificado de Beneficiario Último Económico
Extracto de posiciones a fecha: 31/12/2020.

Beneficios y pérdidas derivados de la transmisión de elementos patrimoniales
Contiene la relación de transacciones realizadas durante el año 2020, así como las comisiones asociadas a dichas operaciones
y las ganancias o pérdidas derivadas de las posiciones cerradas durante el año.

Tenga en cuenta que el cambio de divisa utilizado en este informe es el cambio a final del día, por lo que difiere con el utilizado
en su cuenta de DEGIRO.

Informe anual de su Cuenta de Efectivo en flatex
En este apartado encontrará toda la información relativa a su Cuenta de Efectivo de flatex Bank.

DEGIRO realiza el Informe Fiscal de la manera más rigurosa posible pero no asume responsabilidad por cualquier posible
inexactitud en el mismo. Este informe no es vinculante y de tener cualquier duda al respecto, le rogamos se ponga en contacto
con el Servicio de Atención al Cliente de DEGIRO en 
clientes@degiro.es
.

Atentamente,

DEGIRO


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2020 - 
www.degiro.es
1
/ 4

Sr. John Doe
neverwhere 8
8888 neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam


E
 
clientes@degiro.es
I
 
www.degiro.es
Valor de la Cartera
01-01-2020
31-12-2020
Fondos del Mercado Monetario
564,19 EUR
0,00 EUR
Valor en cartera
46.075,43 EUR
82.624,36 EUR
Valor total
46.639,63 EUR
82.624,36 EUR
Total de comisiones por transacción pagadas en el año 2020 *
Total de comisiones por transacción de las posiciones cerradas *
39,04 EUR
Comisiones
78,46 EUR
Comisión de conectividad con el mercado
25,00 EUR
EUR
83,38
Pérdidas por la venta de ETFs
Ganancias patrimoniales totales
0,00
1.907,59
Pérdidas totales
Ganancia en venta de derivados
0,00
Ganancias por la venta de bonos
Otras ganancias
Pérdidas por la venta de otros productos
EUR
EUR
EUR
EUR
Ganancias / Pérdidas Realizadas
EUR
1.907,59
Pérdidas por la venta de acciones
3,07
0,00
EUR
Ganancias por la venta de acciones
EUR
EUR
0,00
Ganancias por la venta de ETFs
0,00
EUR
3,07
EUR
Otras pérdidas
Ganancias por la venta de otros productos
EUR
0,00
Pérdida en venta de derivados
0,00
Pérdidas por la venta de bonos
0,00
80,31
EUR
EUR
* Incluye la comisión fija de cambio de divisa cuando se haya realizado de forma manual


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2020 - 
www.degiro.es
2
/ 4

Sr. John Doe
neverwhere 8
8888 neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam


E
 
clientes@degiro.es
I
 
www.degiro.es
Interés
Total interés recibido
0,00 EUR
Total interés pagado
0,00 EUR
Total interés pagado por venta en corto
0,00 EUR


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2020 - 
www.degiro.es
3
/ 4

Sr. John Doe
neverwhere 8
8888 neverland


Nombre de usuario: 
******aaa
DEGIRO B.V.
Rembrandt Tower - 9th floor
Amstelplein 1
1096 HA Amsterdam


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
GB
0,00 EUR
10,75 EUR
10,75 EUR
GENUS
GB
0,00 EUR
56,34 EUR
56,34 EUR
JUDGES SCIENTFC
GB
0,00 EUR
26,49 EUR
26,49 EUR
JUDGES SCIENTFC
GB
0,00 EUR
61,62 EUR
61,62 EUR
ROCKROSE ENERGY
LT
-105,00 EUR
700,00 EUR
595,00 EUR
INTER RAO LIETUVA AB
MH
0,00 EUR
198,08 EUR
198,08 EUR
Okeanis Eco Tankers Corp
MH
0,00 EUR
270,42 EUR
270,42 EUR
Okeanis Eco Tankers Corp
MH
0,00 EUR
35,45 EUR
35,45 EUR
Okeanis Eco Tankers Corp
NL
-5,78 EUR
38,50 EUR
32,72 EUR
FLOWTRADERS
1.286,87 EUR
-110,78 EUR
1.397,65 EUR
Distribuciones Fondos del Mercado Monetario
Producto
Ingreso neto
No se han abonado distribuciones
Relación de ganancias y pérdidas por producto
Por favor, tenga en cuenta que el resultado de "Ganancias/Pérdidas" no incluye las comisiones de compra/venta. Las
Ganancias y Pérdidas (G/P) se calculan usando el cambio de divisa al final del día como se expone en la primera página de
este informe.
Producto
Symbol/ISIN
Ganancias / Pérdidas
Comisión pagada
FLOWTRADERS
NL0011279492
665,00
9,83
EUR
EUR
GRAVITY CO. LTD. - AM
US38911N2062
849,21
1,75
EUR
EUR
MONDO TV
IT0001447785
393,38
10,67
EUR
EUR
Morgan Stanley EUR Liquidity Fund
LU1959429272
-3,07
0,00
EUR
EUR
ROCKROSE ENERGY
GB00BYNFCH09
-80,31
16,79
EUR
EUR


DEGIRO B.V. es una empresa de servicios de inversión regulada por la Autoridad Financiera de los Mercados
Holandeses.
Informe Anual 2020 - 
www.degiro.es
4
/ 4



Certificado de Beneficiario Último Económico.
Cliente:
Sr. John Doe
******aaa (56234543)
Nombre de usuario:
Dirección:
neverwhere
País:
España
31/12/2020
Fecha del extracto:
Producto
ISIN
Bolsa
Cantidad
Moneda
Precio
Valor (EUR)
CASH & CASH FUND (EUR)
114,63
ANGI HOMESERVICES INC- A
US00183L1026
NDQ
300
13,1950
3.240,43
USD
BURFORD CAP LD
GG00BMGYLN96
LSE
463
711,0000
3.686,96
GBX
CTT SYSTEMS
SE0000418923
OMX
205
152,2000
3.104,50
SEK
CVD EQUIPMENT CORPORAT
US1266011030
NDQ
1000
3,6300
2.971,52
USD
EVI INDUSTRIES INC
US26929N1028
ASE
618
30,3600
15.358,97
USD
FACEBOOK INC. - CLASS
US30303M1027
NDQ
21
273,1600
4.695,78
USD
FINANCIERE ODET
FR0000062234
EPA
3
786,0000
2.358,00
EUR
GENUS
GB0002074580
LSE
50
4.196,0000
2.349,76
GBX
GEORGIA CAPITAL
GB00BF4HYV08
LSE
800
540,0000
4.838,40
GBX
INTER RAO LIETUVA AB
LT0000128621
WSE
1000
18,8000
4.122,84
PLN
JD.COM INC. - AMERICA
US47215P1066
NDQ
140
87,9000
10.073,69
USD
JUDGES SCIENTFC
GB0032398678
LSE
145
6.380,0000
10.361,12
GBX
KEYWORDS STUDIO
GB00BBQ38507
LSE
130
2.860,0000
4.164,16
GBX
Okeanis Eco Tankers Corp
MHY641771016
OSL
430
54,6000
2.239,80
NOK
SHAKE SHACK INC. CLAS
US8190471016
NSY
34
84,7900
2.359,91
USD
XPO LOGISTICS INC.
US9837931008
NSY
69
119,1950
6.732,54
USD
Amsterdam, 18/01/2021

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
15/09/2020
CTT SYSTEMS
SE0000418923
C
50
128,8000
6.440,00
618,24
4,31
0,0960
15/09/2020
CTT SYSTEMS
SE0000418923
C
13
129,0000
1.677,00
160,99
0,08
0,0960
24/09/2020
CTT SYSTEMS
SE0000418923
C
142
118,8000
16.869,60
1.589,12
4,79
0,0942
11/06/2020
CVD EQUIPMENT
CORPORAT
US1266011030
C
700
3,4800
2.436,00
2.156,10
2,97
0,8851
11/06/2020
CVD EQUIPMENT
CORPORAT
US1266011030
C
300
3,4800
1.044,00
924,04
1,06
0,8851
13/01/2020
EVI INDUSTRIES INC
US26929N1028
C
100
25,3300
2.533,00
2.274,89
0,86
0,8981
13/01/2020
EVI INDUSTRIES INC
US26929N1028
C
40
25,3700
1.014,80
911,39
0,14
0,8981
12/02/2020
EVI INDUSTRIES INC
US26929N1028
C
100
24,3550
2.435,50
2.239,69
0,87
0,9196
12/02/2020
EVI INDUSTRIES INC
US26929N1028
C
38
24,4150
927,77
853,18
0,14
0,9196
05/03/2020
EVI INDUSTRIES INC
US26929N1028
C
40
21,0000
840,00
747,60
0,64
0,8900
05/03/2020
EVI INDUSTRIES INC
US26929N1028
C
40
21,0000
840,00
747,60
0,14
0,8900
11/08/2020
FINANCIERE ODET
FR0000062234
C
3
680,0000
2.040,00
2.040,00
5,02
1,0000
06/03/2020
FLOWTRADERS
NL0011279492
C
70
21,4400
1.500,80
1.500,80
4,75
1,0000
11/06/2020
FLOWTRADERS
NL0011279492
V
70
30,9400
2.165,80
2.165,80
5,08
1,0000
665,0000
07/07/2020
GENUS
GB0002074580
C
50
3.520,0000
176.000,00
1.958,00
4,97
0,0111
20/05/2020
GEORGIA CAPITAL
GB00BF4HYV08
C
355
447,5000
158.862,50
1.771,63
4,89
0,0112
20/05/2020
GEORGIA CAPITAL
GB00BF4HYV08
C
434
447,5000
194.215,00
2.165,89
1,09
0,0112
20/05/2020
GEORGIA CAPITAL
GB00BF4HYV08
C
10
447,5000
4.475,00
49,91
0,03
0,0112
20/05/2020
GEORGIA CAPITAL
GB00BF4HYV08
C
1
447,5000
447,50
4,99
0,00
0,0112
19/05/2020
GRAVITY CO. LTD. - AM
US38911N2062
V
100
42,0400
4.204,00
3.848,76
0,87
0,9155
832,6120
19/05/2020
GRAVITY CO. LTD. - AM
US38911N2062
V
2
42,0100
84,02
76,92
0,01
0,9155
16,5973
18/02/2020
INTER RAO LIETUVA AB
LT0000128621
C
695
20,6000
14.317,00
3.354,47
10,37
0,2343
18/02/2020
INTER RAO LIETUVA AB
LT0000128621
C
49
20,7000
1.014,30
237,65
0,38
0,2343
18/02/2020
INTER RAO LIETUVA AB
LT0000128621
C
256
20,7000
5.299,20
1.241,60
1,99
0,2343
17/01/2020
KEYWORDS STUDIO
GB00BBQ38507
C
25
1.544,0000
38.600,00
452,86
4,26
0,0117
17/01/2020
KEYWORDS STUDIO
GB00BBQ38507
C
105
1.544,0000
162.120,00
1.901,99
1,11
0,0117
17/01/2020
MONDO TV
IT0001447785
V
1105
2,2560
2.492,88
2.492,88
5,45
1,0000
393,3800
01/04/2020
Okeanis Eco Tankers Corp
MHY641771016
C
59
74,0000
4.366,00
380,72
4,19
0,0872

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
01/04/2020
Okeanis Eco Tankers Corp
MHY641771016
C
25
74,0000
1.850,00
161,32
0,08
0,0872
01/04/2020
Okeanis Eco Tankers Corp
MHY641771016
C
346
74,0000
25.604,00
2.232,67
1,11
0,0872
06/07/2020
ROCKROSE ENERGY
GB00BYNFCH09
V
216
1.830,0000
395.280,00
4.366,26
6,19
0,0110
-80,3072
13/01/2020
SHAKE SHACK INC. CLAS
US8190471016
C
34
60,6300
2.061,42
1.851,36
0,62
0,8981

Informe anual de flatex
Para ayudarle a realizar su declaración de la renta le proveemos con este informe anual ya que dispone de una Cuenta de
Efectivo en flatex asociada a su cuenta de DEGIRO.

Tenga en cuenta que es su responsabilidad el notificar a las autoridades locales sobre su Cuenta de Efectivo en flatex, lo
cual incluye declararlo en su declaración de la renta. Le informamos de que su cuenta en DEGIRO es independiente de su
Cuenta de Efectivo, ya que, la primera está mantenida por DEGIRO B.V. en Holanda y la segunda por flatex Bank AG en
Alemania.

Este reporte refleja la siguiente información:


    •  El balance total de su Cuenta de Efectivo en flatex durante el periodo reportado.

    •  Los depósitos y retiradas a su cuenta de flatex durante el mismo periodo. Se incluye, en dicha información, el valor total
de los depósitos que haya realizado desde su cuenta de contrapartida a su cuenta de flatex, las retiradas realizadas desde
su Cuenta de Efectivo a su cuenta de contrapartida y la suma de los depósitos y retiradas entre su Cuenta de Efectivo y su
cuenta de valores de DEGIRO.

    •  El interés pagado y recibido en su Cuenta de Efectivo durante el periodo señalado.
Cuenta de Efectivo en flatex
December 31, 2019
December 31, 2020
Balance total
0,00 EUR
114,63 EUR
Depósitos y retiradas
1.600,00 EUR
Valor total de los depósitos *
0,00 EUR
Valor total de las retiradas realizadas *
0,00 EUR
Intereses totales recibidos
Intereses flatex
Intereses totales pagados
0,10 EUR
* Esta sección refleja los depósitos realizados desde su cuenta de contrapartida a su Cuenta de Efectivo en flatex y las retiradas realizadas
desde su Cuenta de Efectivo a su cuenta vinculada
-1.485,27 EUR
Valor total de los depósitos y retiradas desde y a DEGIRO"#;
}
