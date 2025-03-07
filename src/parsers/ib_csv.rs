use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{
    data::{
        AccountNote, AccountNotes, BalanceNote, BalanceNotes, BrokerInformation, BrokerOperation,
        CompanyInfo,
    },
    parsers::util,
    utils::decimal,
};

enum NoteState {
    Invalid,
    Stocks,
    Total,
}
pub struct IBCSVParser {
    content: String,
    locale: &'static [&'static str],
    broker: Arc<BrokerInformation>,
    companies_info: HashMap<String, CompanyInfo>,
}

impl IBCSVParser {
    const EUR_CURRENCY_STR: &'static str = "EUR";

    const STOCK_COMPANY_INFO_SECTOR_START_STR: usize = 0;
    const STOCK_COMPANY_INFO_SECTOR_END_STR: usize = 1;
    const OPEN_POSITIONS_BEGIN_STR: usize = 2;
    const OPEN_POSITIONS_END_STR: usize = 3;
    const OPEN_POSITIONS_STOCK_STR: usize = 4;
    const OPEN_POSITIONS_TOTAL_STR: usize = 5;
    const TRADE_BEGIN_STR: usize = 6;
    const TRADE_BEGIN__NO_ACCOUNT_STR: usize = 7;
    const TRADE_END_STR: usize = 8;
    const TRADE_STOCK_STR: usize = 9;
    const STOCK_COMPANY_INFO_SECTOR_START_OLD_STR: usize = 10;

    const ES_HEADER_CONTENT: &str = "Statement,Header,Nombre del campo,Valor del campo";

    const EN_MSGS: &'static [&'static str] = &[
        "Financial Instrument Information,Header,Asset Category,Symbol,Description,Conid,Security ID,Underlying,Listing Exch,Multiplier,Type,Code", // STOCK_COMPANY_INFO_SECTOR_START_STR
        "Financial Instrument Information,Data,Stocks,", // STOCK_COMPANY_INFO_SECTOR_END_STR
        "Open Positions,Header,DataDiscriminator,Asset Category,Currency,Symbol,Quantity,Mult,Cost Price,Cost Basis,Close Price,Value,Unrealized P/L,Code", // OPEN_POSITIONS_BEGIN_STR
        "Open Positions,Total,,Stocks,EUR,", // OPEN_POSITIONS_END_STR
        "Open Positions,Data,Summary,Stocks,", // OPEN_POSITIONS_STOCK_STR
        "Open Positions,Total,,Stocks,",     // OPEN_POSITIONS_TOTAL_STR
        "Trades,Header,DataDiscriminator,Asset Category,Currency,Account,Symbol,Date/Time,Quantity,T. Price,C. Price,Proceeds,Comm/Fee,Basis,Realized P/L,MTM P/L,Code", // TRADE_BEGIN_STR
        "Trades,Header,DataDiscriminator,Asset Category,Currency,Symbol,Date/Time,Quantity,T. Price,C. Price,Proceeds,Comm/Fee,Basis,Realized P/L,MTM P/L,Code", // TRADE_BEGIN_NO_ACCOUNT_STR
        "Trades,Total,",             // TRADE_END_STR
        "Trades,Data,Order,Stocks,", // TRADE_STOCK_STR
        "Financial Instrument Information,Header,Asset Category,Symbol,Description,Conid,Security ID,Listing Exch,Multiplier,Type,Code", // STOCK_COMPANY_INFO_SECTOR_START_OLD_STR
    ];

    const ES_MSGS: &'static [&'static str] = &[
        "Información de instrumento financiero,Header,Categoría de activo,Símbolo,Descripción,Conid,Id. de seguridad,Underlying,Merc. de cotización,Multiplicador,Tipo,Código", // STOCK_COMPANY_INFO_SECTOR_START_STR
        "Información de instrumento financiero,Data,Acciones,", // STOCK_COMPANY_INFO_SECTOR_END_STR
        "Posiciones abiertas,Header,DataDiscriminator,Categoría de activo,Divisa,Símbolo,Cantidad,Mult.,Precio de coste,Base de coste,Precio de cierre,Valor,PyG no realizadas,Código", // OPEN_POSITIONS_BEGIN_STR
        "Posiciones abiertas,Total,,Acciones,EUR,", // OPEN_POSITIONS_END_STR
        "Posiciones abiertas,Data,Summary,Acciones,", // OPEN_POSITIONS_STOCK_STR
        "Posiciones abiertas,Total,,Acciones,",     // OPEN_POSITIONS_TOTAL_STR
        "Operaciones,Header,DataDiscriminator,Categoría de activo,Divisa,Cuenta,Símbolo,Fecha/Hora,Cantidad,Precio trans.,Precio de cier.,Productos,Tarifa/com.,Básico,PyG realizadas,MTM P/G,Código", // TRADE_BEGIN_STR
        "Operaciones,Header,DataDiscriminator,Categoría de activo,Divisa,Símbolo,Fecha/Hora,Cantidad,Precio trans.,Precio de cier.,Productos,Tarifa/com.,Básico,PyG realizadas,MTM P/G,Código", // TRADE_BEGIN_NO_ACCOUNT_STR
        "Operaciones,Total,",               // TRADE_END_STR
        "Operaciones,Data,Order,Acciones,", // TRADE_STOCK_STR
        "Información de instrumento financiero,Header,Categoría de activo,Símbolo,Descripción,Conid,Id. de seguridad,Merc. de cotización,Multiplicador,Tipo,Código", // STOCK_COMPANY_INFO_SECTOR_START_OLD_STR
    ];

    fn parse_companies_info(
        content: &str,
        locale: &[&'static str],
    ) -> Result<HashMap<String, CompanyInfo>> {
        log::debug!("parse companies info");
        let mut result: HashMap<String, CompanyInfo> = HashMap::new();

        let start = content
            .find(locale[IBCSVParser::STOCK_COMPANY_INFO_SECTOR_START_STR])
            .or_else(|| content.find(locale[IBCSVParser::STOCK_COMPANY_INFO_SECTOR_START_OLD_STR]))
            .ok_or_else(|| anyhow!("Not found beginning of companies info section"))?;

        let end_left = content
            .rfind(locale[IBCSVParser::STOCK_COMPANY_INFO_SECTOR_END_STR])
            .ok_or_else(|| anyhow!("Not found end of companies info section"))?;

        let end = content[end_left..]
            .find('\n')
            .ok_or_else(|| anyhow!("Not found end of companies info section"))?;

        let mut rdr = csv::Reader::from_reader(&content.as_bytes()[start..end_left + end]);

        for record_result in rdr.records() {
            let record = record_result?;
            result.insert(
                String::from(record.get(3).ok_or_else(|| anyhow!("Unknown ticker"))?),
                CompanyInfo {
                    name: String::from(
                        record
                            .get(4)
                            .ok_or_else(|| anyhow!("Unknown company name"))?,
                    ),
                    isin: String::from(record.get(6).ok_or_else(|| anyhow!("Unknown isin"))?),
                },
            );
        }

        Ok(result)
    }

    fn parse_account_note(&self, fields: &[&str]) -> Result<AccountNote> {
        log::debug!("account note fields {:?}", fields);
        let offset = if fields.len() == 16 { 0 } else { 1 };
        let symbol = fields[5 + offset];
        let date = fields[6 + offset];
        let quantity_str = fields[7 + offset];
        let quantity = Decimal::from_str(&decimal::normalize_str(quantity_str))?;
        let operation = if quantity.is_sign_negative() {
            BrokerOperation::Sell
        } else {
            BrokerOperation::Buy
        };
        let price = fields[8 + offset];
        let value = fields[10 + offset];
        let commision = fields[11 + offset];
        let _earnings = fields[13 + offset];
        let company_info = if let Some(company) = self.companies_info.get(symbol) {
            company.clone()
        } else {
            log::error!("Not company info found for {}", symbol);
            CompanyInfo {
                name: symbol.to_string(),
                isin: "".to_string(),
            }
        };

        Ok(AccountNote::new(
            NaiveDate::parse_from_str(date, "%Y-%m-%d %H:%M:%S")?,
            company_info,
            operation,
            quantity.abs(),
            Decimal::from_str(&decimal::normalize_str(price))?,
            Decimal::from_str(&decimal::normalize_str(value))?.abs(),
            Decimal::from_str(&decimal::normalize_str(commision))?.abs(),
            &self.broker,
        ))
    }

    pub fn parse_account_notes(&self) -> Result<AccountNotes> {
        let mut result = Vec::new();
        let start = self
            .content
            .find(self.locale[IBCSVParser::TRADE_BEGIN_STR])
            .or_else(|| {
                self.content
                    .find(self.locale[IBCSVParser::TRADE_BEGIN__NO_ACCOUNT_STR])
            })
            .ok_or_else(|| anyhow!("Not found beginning of trades section"))?;

        let end = self
            .content
            .rfind(self.locale[IBCSVParser::TRADE_END_STR])
            .ok_or_else(|| anyhow!("Not found end of trades section"))?;

        let lines: Vec<&str> = (self.content[start..end - 1]).split('\n').collect();

        for line in lines.iter() {
            if line.starts_with(self.locale[IBCSVParser::TRADE_STOCK_STR]) {
                let replaced_line = util::replace_escaped_fields(line);
                let fields: Vec<&str> = replaced_line.split(',').collect();
                let account_note = self.parse_account_note(&fields)?;
                result.push(account_note);
            }
        }

        Ok(result)
    }

    fn parse_balance_note(&self, fields: &[&str], currency: &Option<&str>) -> Result<BalanceNote> {
        let symbol = fields[5];
        let quantity = fields[6];
        let mult = fields[7];
        let price = fields[10];
        let value_in_euro = fields[11];
        let company_info = self
            .companies_info
            .get(symbol)
            .cloned()
            .or_else({
                || {
                    log::error!("Not company info found for {}", symbol);
                    Some(CompanyInfo {
                        name: symbol.to_string(),
                        isin: "".to_string(),
                    })
                }
            })
            .unwrap();

        Ok(BalanceNote::new(
            company_info,
            String::from(""),
            Decimal::from_str(&decimal::normalize_str(quantity))?
                * Decimal::from_str(&decimal::normalize_str(mult))?,
            String::from(currency.unwrap_or(IBCSVParser::EUR_CURRENCY_STR)),
            Decimal::from_str(&decimal::normalize_str(price))?,
            Decimal::from_str(&decimal::normalize_str(value_in_euro))?,
            &self.broker,
        ))
    }

    pub fn parse_balance_notes(&self) -> Result<BalanceNotes> {
        let mut balance_notes = vec![];

        let start = self
            .content
            .find(self.locale[IBCSVParser::OPEN_POSITIONS_BEGIN_STR])
            .ok_or_else(|| anyhow!("Not found beginning of open position section"))?;

        let end = self
            .content
            .rfind(self.locale[IBCSVParser::OPEN_POSITIONS_END_STR])
            .ok_or_else(|| anyhow!("Not found end of open position section"))?;

        let lines: Vec<&str> = (self.content[start..end - 1]).split('\n').collect();

        let mut state = NoteState::Invalid;
        let mut current_notes: BalanceNotes = Vec::new();
        let mut currency = None;

        for line in lines.iter() {
            match state {
                NoteState::Invalid => {
                    log::debug!("Invalid state");
                    if line.starts_with(self.locale[IBCSVParser::OPEN_POSITIONS_STOCK_STR]) {
                        state = NoteState::Stocks;
                        let fields: Vec<&str> = line.split(',').collect();
                        currency = Some(fields[4]);
                        let balance_note = self.parse_balance_note(&fields, &currency)?;
                        current_notes.push(balance_note);
                    }
                }
                NoteState::Stocks => {
                    log::debug!("Stocks state");
                    if line.starts_with(self.locale[IBCSVParser::OPEN_POSITIONS_STOCK_STR]) {
                        let fields: Vec<&str> = line.split(',').collect();
                        currency = Some(fields[4]);
                        let balance_note = self.parse_balance_note(&fields, &currency)?;
                        current_notes.push(balance_note);
                    } else if line.starts_with(self.locale[IBCSVParser::OPEN_POSITIONS_TOTAL_STR]) {
                        state = NoteState::Total;
                        if currency == Some(IBCSVParser::EUR_CURRENCY_STR) {
                            state = NoteState::Stocks;
                            balance_notes.append(&mut current_notes);
                        }
                    }
                }
                NoteState::Total => {
                    log::debug!("Total state");

                    state = NoteState::Stocks;
                    let fields: Vec<&str> = line.split(',').collect();
                    let total_in_euro_str = fields[11];
                    let total_in_euro =
                        Decimal::from_str(&decimal::normalize_str(total_in_euro_str))?;
                    log::debug!("total in eur: {:?}", total_in_euro);
                    util::recalculate_balance_notes(&mut current_notes, &total_in_euro)?;
                    balance_notes.append(&mut current_notes);
                }
            }
        }

        Ok(balance_notes)
    }

    pub fn new(content: String, broker: &Arc<BrokerInformation>) -> Result<Self> {
        let locale = if content.contains(IBCSVParser::ES_HEADER_CONTENT) {
            IBCSVParser::ES_MSGS
        } else {
            IBCSVParser::EN_MSGS
        };
        let companies_info = IBCSVParser::parse_companies_info(&content, locale)?;

        Ok(Self {
            content,
            locale,
            broker: Arc::clone(broker),
            companies_info,
        })
    }
}

#[cfg(test)]
#[allow(clippy::zero_prefixed_literal)]
mod tests {
    use super::*;

    #[ctor::ctor]
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

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
    fn test_parse_companies_info() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("IB"),
            String::from("IE"),
        ));

        let parser = IBCSVParser::new(INPUT_2021.to_string(), &ib_broker).unwrap();

        let companies_info: HashMap<String, CompanyInfo> = companies_info();

        assert_eq!(companies_info, parser.companies_info);
    }

    #[test]
    fn test_parse_companies_info_es() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("IB"),
            String::from("IE"),
        ));

        let parser = IBCSVParser::new(INPUT_2021_ES.to_string(), &ib_broker).unwrap();

        let companies_info: HashMap<String, CompanyInfo> = companies_info();

        assert_eq!(companies_info, parser.companies_info);
    }

    #[test]
    fn test_parse_balance_notes() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("IB"),
            String::from("IE"),
        ));

        let parser = IBCSVParser::new(INPUT_2021.to_string(), &ib_broker).unwrap();
        let balance_notes = parser.parse_balance_notes().unwrap();
        let bal_notes = get_test_balance_notes(&ib_broker);

        assert_eq!(&bal_notes, &balance_notes);
    }

    #[test]
    fn test_parse_balance_notes_es() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("IB"),
            String::from("IE"),
        ));

        let parser = IBCSVParser::new(INPUT_2021_ES.to_string(), &ib_broker).unwrap();
        let balance_notes = parser.parse_balance_notes().unwrap();
        let bal_notes = get_test_balance_notes(&ib_broker);

        assert_eq!(&bal_notes, &balance_notes);
    }

    #[test]
    fn test_parse_account_notes() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Interactive Brokers"),
            String::from("IE"),
        ));
        let ibparser = IBCSVParser::new(INPUT_2021.to_string(), &ib_broker).unwrap();
        let notes = ibparser.parse_account_notes().unwrap();

        let acc_notes = get_test_account_notes(&ib_broker);

        compare_vectors_by_item(&acc_notes, &notes);
        assert_eq!(acc_notes, notes);
    }

    #[test]
    fn test_parse_account_notes_es() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Interactive Brokers"),
            String::from("IE"),
        ));
        let ibparser = IBCSVParser::new(INPUT_2021_ES.to_string(), &ib_broker).unwrap();
        let notes = ibparser.parse_account_notes().unwrap();

        let acc_notes = get_test_account_notes(&ib_broker);

        compare_vectors_by_item(&acc_notes, &notes);
        assert_eq!(acc_notes, notes);
    }

    const INPUT_2021: &str = include_str!("testdata/ib_test.csv");
    const INPUT_2021_ES: &str = include_str!("testdata/ib_test_es.csv");

    #[allow(clippy::mistyped_literal_suffixes)]
    fn get_test_balance_notes(broker: &Arc<BrokerInformation>) -> Vec<BalanceNote> {
        vec![
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                String::from(""),
                Decimal::new(10330, 0),
                String::from("AUD"),
                Decimal::new(33, 1),
                Decimal::new(21778_78, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("CETTIRE LTD"),
                    isin: String::from("AU0000122210"),
                },
                String::from(""),
                Decimal::new(2500, 0),
                String::from("AUD"),
                Decimal::new(3_56, 2),
                Decimal::new(5686_03, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("REDBUBBLE LTD"),
                    isin: String::from("AU000000RBL2"),
                },
                String::from(""),
                Decimal::new(1800, 0),
                String::from("AUD"),
                Decimal::new(3_27, 2),
                Decimal::new(3760_45, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("VOXTUR ANALYTICS CORP"),
                    isin: String::from("CA9290821052"),
                },
                String::from(""),
                Decimal::new(5700, 0),
                String::from("CAD"),
                Decimal::new(1_19, 2),
                Decimal::new(4719_00, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("ADYEN NV"),
                    isin: String::from("NL0012969182"),
                },
                String::from(""),
                Decimal::new(1, 0),
                String::from("EUR"),
                Decimal::new(2311_5, 1),
                Decimal::new(2311_5, 1),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("PROSUS NV"),
                    isin: String::from("NL0013654783"),
                },
                String::from(""),
                Decimal::new(45, 0),
                String::from("EUR"),
                Decimal::new(73_53, 2),
                Decimal::new(3308_85, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("JD.COM INC - CL A"),
                    isin: String::from("KYG8208B1014"),
                },
                String::from(""),
                Decimal::new(400, 0),
                String::from("HKD"),
                Decimal::new(274, 0),
                Decimal::new(12361_78, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("BEENOS INC"),
                    isin: String::from("JP3758110005"),
                },
                String::from(""),
                Decimal::new(100, 0),
                String::from("JPY"),
                Decimal::new(2500, 0),
                Decimal::new(1909_9, 1),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("EVOLUTION AB"),
                    isin: String::from("SE0012673267"),
                },
                String::from(""),
                Decimal::new(20, 0),
                String::from("SEK"),
                Decimal::new(1286_2, 1),
                Decimal::new(2499_19, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("AMAZON.COM INC"),
                    isin: String::from("US0231351067"),
                },
                String::from(""),
                Decimal::new(2, 0),
                String::from("USD"),
                Decimal::new(3334_34, 2),
                Decimal::new(5863_97, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("GOLAR LNG LTD"),
                    isin: String::from("BMG9456A1009"),
                },
                String::from(""),
                Decimal::new(250, 0),
                String::from("USD"),
                Decimal::new(12_39, 2),
                Decimal::new(2723_72, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("ILLUMINA INC"),
                    isin: String::from("US4523271090"),
                },
                String::from(""),
                Decimal::new(8, 0),
                String::from("USD"),
                Decimal::new(380_44, 2),
                Decimal::new(2676_26, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("OPEN LENDING CORP - CL A"),
                    isin: String::from("US68373J1043"),
                },
                String::from(""),
                Decimal::new(250, 0),
                String::from("USD"),
                Decimal::new(22_48, 2),
                Decimal::new(4941_83, 2),
                broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("MIND TECHNOLOGY INC"),
                    isin: String::from("US6025661017"),
                },
                String::from(""),
                Decimal::new(2350, 0),
                String::from("USD"),
                Decimal::new(1_6884, 4),
                Decimal::new(3488_95, 2),
                broker,
            ),
        ]
    }

    #[allow(clippy::mistyped_literal_suffixes)]
    fn get_test_account_notes(broker: &Arc<BrokerInformation>) -> Vec<AccountNote> {
        vec![
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 01, 13).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(950, 0),
                Decimal::new(4_33, 2),
                Decimal::new(4113_5, 1),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 01, 18).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(1424, 0),
                Decimal::new(3_91, 2),
                Decimal::new(5567_84, 2),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 01, 28).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(26, 0),
                Decimal::new(3_91, 2),
                Decimal::new(101_66, 2),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 02, 25).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(1200, 0),
                Decimal::new(4_09, 2),
                Decimal::new(4908, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 03, 17).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(900, 0),
                Decimal::new(4_4, 1),
                Decimal::new(3960, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 06, 24).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(1410, 0),
                Decimal::new(3_7, 1),
                Decimal::new(5217, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 06, 30).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(180, 0),
                Decimal::new(3_75, 2),
                Decimal::new(675, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 07, 07).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(1850, 0),
                Decimal::new(3_7, 1),
                Decimal::new(6845, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 08, 02).unwrap(),
                CompanyInfo {
                    name: String::from("ADVANCE ZINCTEK LTD"),
                    isin: String::from("AU000000ANO7"),
                },
                BrokerOperation::Buy,
                Decimal::new(2300, 0),
                Decimal::new(3_6, 1),
                Decimal::new(8280, 0),
                Decimal::new(6_624, 3),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 03, 17).unwrap(),
                CompanyInfo {
                    name: String::from("CETTIRE LTD"),
                    isin: String::from("AU0000122210"),
                },
                BrokerOperation::Buy,
                Decimal::new(5000, 0),
                Decimal::new(1_28, 2),
                Decimal::new(6400, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 06, 08).unwrap(),
                CompanyInfo {
                    name: String::from("CETTIRE LTD"),
                    isin: String::from("AU0000122210"),
                },
                BrokerOperation::Sell,
                Decimal::new(2500, 0),
                Decimal::new(2_77, 2),
                Decimal::new(6925, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 06, 24).unwrap(),
                CompanyInfo {
                    name: String::from("REDBUBBLE LTD"),
                    isin: String::from("AU000000RBL2"),
                },
                BrokerOperation::Buy,
                Decimal::new(1000, 0),
                Decimal::new(3_32, 2),
                Decimal::new(3320, 0),
                Decimal::new(6, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 01, 28).unwrap(),
                CompanyInfo {
                    name: String::from("ILA"),
                    isin: String::from(""),
                },
                BrokerOperation::Buy,
                Decimal::new(5700, 0),
                Decimal::new(0_55, 2),
                Decimal::new(3135, 0),
                Decimal::new(17_895, 3),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 01, 14).unwrap(),
                CompanyInfo {
                    name: String::from("UMANIS - REG"),
                    isin: String::from("FR0013263878"),
                },
                BrokerOperation::Sell,
                Decimal::new(300, 0),
                Decimal::new(9_044, 3),
                Decimal::new(2713_2, 1),
                Decimal::new(4, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 03, 17).unwrap(),
                CompanyInfo {
                    name: String::from("TFF GROUP"),
                    isin: String::from("FR0013295789"),
                },
                BrokerOperation::Sell,
                Decimal::new(90, 0),
                Decimal::new(28_3, 1),
                Decimal::new(2547, 0),
                Decimal::new(4, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 12, 07).unwrap(),
                CompanyInfo {
                    name: String::from("EVOLUTION AB"),
                    isin: String::from("SE0012673267"),
                },
                BrokerOperation::Buy,
                Decimal::new(20, 0),
                Decimal::new(987_1, 1),
                Decimal::new(19742, 0),
                Decimal::new(49, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 05, 12).unwrap(),
                CompanyInfo {
                    name: String::from("AMAZON.COM INC"),
                    isin: String::from("US0231351067"),
                },
                BrokerOperation::Buy,
                Decimal::new(2, 0),
                Decimal::new(3139_64, 2),
                Decimal::new(6279_28, 2),
                Decimal::new(1, 0),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 05, 11).unwrap(),
                CompanyInfo {
                    name: String::from("CANNAE HOLDINGS INC"),
                    isin: String::from("US13765N1072"),
                },
                BrokerOperation::Sell,
                Decimal::new(66_1549, 4),
                Decimal::new(36_16, 2),
                Decimal::new(2392_161184, 6),
                Decimal::new(1_020072455, 9),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 03, 30).unwrap(),
                CompanyInfo {
                    name: String::from("INTELLICHECK INC"),
                    isin: String::from("US45817G2012"),
                },
                BrokerOperation::Buy,
                Decimal::new(430, 0),
                Decimal::new(7_958104651, 9),
                Decimal::new(3421_985, 3),
                Decimal::new(2_15, 2),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 05, 12).unwrap(),
                CompanyInfo {
                    name: String::from("INTELLICHECK INC"),
                    isin: String::from("US45817G2012"),
                },
                BrokerOperation::Sell,
                Decimal::new(430, 0),
                Decimal::new(7_329302326, 9),
                Decimal::new(3151_6, 1),
                Decimal::new(2_21724316, 8),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 01, 14).unwrap(),
                CompanyInfo {
                    name: String::from("LIBERTY LATIN AMERIC-CL C"),
                    isin: String::from("BMG9001E1286"),
                },
                BrokerOperation::Sell,
                Decimal::new(100, 0),
                Decimal::new(11_32, 2),
                Decimal::new(1132, 0),
                Decimal::new(1_0369172, 7),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 05, 12).unwrap(),
                CompanyInfo {
                    name: String::from("MIND TECHNOLOGY INC"),
                    isin: String::from("US6025661017"),
                },
                BrokerOperation::Buy,
                Decimal::new(350, 0),
                Decimal::new(2_248657143, 9),
                Decimal::new(787_03, 2),
                Decimal::new(1_75, 2),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 05, 12).unwrap(),
                CompanyInfo {
                    name: String::from("TELADOC HEALTH INC"),
                    isin: String::from("US87918A1051"),
                },
                BrokerOperation::Sell,
                Decimal::new(10_656, 3),
                Decimal::new(140_739384384, 9),
                Decimal::new(1499_71888, 5),
                Decimal::new(1_00891663, 8),
                broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2021, 03, 17).unwrap(),
                CompanyInfo {
                    name: String::from("TEEKAY CORP"),
                    isin: String::from("MHY8564W1030"),
                },
                BrokerOperation::Sell,
                Decimal::new(1744, 0),
                Decimal::new(3_556880734, 9),
                Decimal::new(6203_2, 1),
                Decimal::new(8_95917232, 8),
                broker,
            ),
        ]
    }

    fn companies_info() -> HashMap<String, CompanyInfo> {
        HashMap::from([
            (
                "3328.T".to_string(),
                CompanyInfo {
                    name: "BEENOS INC".to_string(),
                    isin: "JP3758110005".to_string(),
                },
            ),
            (
                "9618".to_string(),
                CompanyInfo {
                    name: "JD.COM INC - CL A".to_string(),
                    isin: "KYG8208B1014".to_string(),
                },
            ),
            (
                "ADYEN".to_string(),
                CompanyInfo {
                    name: "ADYEN NV".to_string(),
                    isin: "NL0012969182".to_string(),
                },
            ),
            (
                "ALUMS".to_string(),
                CompanyInfo {
                    name: "UMANIS - REG".to_string(),
                    isin: "FR0013263878".to_string(),
                },
            ),
            (
                "AMZN".to_string(),
                CompanyInfo {
                    name: "AMAZON.COM INC".to_string(),
                    isin: "US0231351067".to_string(),
                },
            ),
            (
                "ANO".to_string(),
                CompanyInfo {
                    name: "ADVANCE ZINCTEK LTD".to_string(),
                    isin: "AU000000ANO7".to_string(),
                },
            ),
            (
                "ANO.RTS".to_string(),
                CompanyInfo {
                    name: "ADVANCE NANOTEK LTD - RIGHTS".to_string(),
                    isin: "AU0000151565".to_string(),
                },
            ),
            (
                "ANO.SUB6".to_string(),
                CompanyInfo {
                    name: "ADVANCE NANOTEK LTD - RIGHTS SUBSCRIPTION".to_string(),
                    isin: "AU00ANO7SUB6".to_string(),
                },
            ),
            (
                "CNNE".to_string(),
                CompanyInfo {
                    name: "CANNAE HOLDINGS INC".to_string(),
                    isin: "US13765N1072".to_string(),
                },
            ),
            (
                "CTT".to_string(),
                CompanyInfo {
                    name: "CETTIRE LTD".to_string(),
                    isin: "AU0000122210".to_string(),
                },
            ),
            (
                "EVOs".to_string(),
                CompanyInfo {
                    name: "EVOLUTION AB".to_string(),
                    isin: "SE0012673267".to_string(),
                },
            ),
            (
                "GLNG".to_string(),
                CompanyInfo {
                    name: "GOLAR LNG LTD".to_string(),
                    isin: "BMG9456A1009".to_string(),
                },
            ),
            (
                "IDN".to_string(),
                CompanyInfo {
                    name: "INTELLICHECK INC".to_string(),
                    isin: "US45817G2012".to_string(),
                },
            ),
            (
                "ILA.OLD, ILA".to_string(),
                CompanyInfo {
                    name: "ILOOKABOUT CORP".to_string(),
                    isin: "CA45236R1010".to_string(),
                },
            ),
            (
                "ILMN".to_string(),
                CompanyInfo {
                    name: "ILLUMINA INC".to_string(),
                    isin: "US4523271090".to_string(),
                },
            ),
            (
                "JD".to_string(),
                CompanyInfo {
                    name: "JD.COM INC-ADR".to_string(),
                    isin: "US47215P1066".to_string(),
                },
            ),
            (
                "JD.CNV".to_string(),
                CompanyInfo {
                    name: "JD.COM INC-ADR - TENDER".to_string(),
                    isin: "US47215PCNV0".to_string(),
                },
            ),
            (
                "LILAK".to_string(),
                CompanyInfo {
                    name: "LIBERTY LATIN AMERIC-CL C".to_string(),
                    isin: "BMG9001E1286".to_string(),
                },
            ),
            (
                "LPRO".to_string(),
                CompanyInfo {
                    name: "OPEN LENDING CORP - CL A".to_string(),
                    isin: "US68373J1043".to_string(),
                },
            ),
            (
                "MIND".to_string(),
                CompanyInfo {
                    name: "MIND TECHNOLOGY INC".to_string(),
                    isin: "US6025661017".to_string(),
                },
            ),
            (
                "PRX".to_string(),
                CompanyInfo {
                    name: "PROSUS NV".to_string(),
                    isin: "NL0013654783".to_string(),
                },
            ),
            (
                "PRX.RTS".to_string(),
                CompanyInfo {
                    name: "PROSUS NV - RIGHTS".to_string(),
                    isin: "NL0015000LD0".to_string(),
                },
            ),
            (
                "RBL".to_string(),
                CompanyInfo {
                    name: "REDBUBBLE LTD".to_string(),
                    isin: "AU000000RBL2".to_string(),
                },
            ),
            (
                "TDOC".to_string(),
                CompanyInfo {
                    name: "TELADOC HEALTH INC".to_string(),
                    isin: "US87918A1051".to_string(),
                },
            ),
            (
                "TFF".to_string(),
                CompanyInfo {
                    name: "TFF GROUP".to_string(),
                    isin: "FR0013295789".to_string(),
                },
            ),
            (
                "TK".to_string(),
                CompanyInfo {
                    name: "TEEKAY CORP".to_string(),
                    isin: "MHY8564W1030".to_string(),
                },
            ),
            (
                "VXTR".to_string(),
                CompanyInfo {
                    name: "VOXTUR ANALYTICS CORP".to_string(),
                    isin: "CA9290821052".to_string(),
                },
            ),
        ])
    }
}
