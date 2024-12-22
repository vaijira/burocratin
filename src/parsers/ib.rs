use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, LazyLock},
};

use crate::{
    data::{
        AccountNote, AccountNotes, BalanceNote, BalanceNotes, BrokerInformation, BrokerOperation,
        CompanyInfo,
    },
    parsers::util,
    utils::decimal,
};
use anyhow::{anyhow, bail, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use scraper::{node::Element, ElementRef, Html, Selector};
use selectors::attr::CaseSensitivity;

static OPEN_POSITIONS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"div[id^="tblOpenPositions_"] div table"#).unwrap());

static CONTRACT_INFO_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"div[id^="tblContractInfo"] div table"#).unwrap());

static TRANSACTIONS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"div[id^="tblTransactions_"] div table"#).unwrap());

static THEAD_TH_TR_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"thead tr"#).unwrap());
static TBODY_TR_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"tbody tr"#).unwrap());
static TR_SELECTOR: LazyLock<Selector> = LazyLock::new(|| Selector::parse(r#"tr"#).unwrap());

enum NoteState {
    Invalid,
    Stocks,
    Note,
    Total,
}

pub struct IBParser {
    dom: Html,
    broker: Arc<BrokerInformation>,
    companies_info: HashMap<String, CompanyInfo>,
}

static STOCKS_STRS: LazyLock<HashSet<Option<&'static str>>> =
    LazyLock::new(|| HashSet::from([Some("Stocks"), Some("Acciones")]));

impl IBParser {
    const EUR_CURRENCY_STR: &'static str = "EUR";

    pub fn new(data: &str, broker: &Arc<BrokerInformation>) -> Result<Self> {
        let dom = Html::parse_document(data);
        let companies_info = IBParser::parse_companies_info(&dom)?;

        Ok(Self {
            dom,
            broker: Arc::clone(broker),
            companies_info,
        })
    }

    fn parse_account_note(
        &self,
        row: &ElementRef<'_>,
        with_account_field: bool,
    ) -> Result<AccountNote> {
        let field_values = row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
        let offset = if with_account_field { 1 } else { 0 };
        log::debug!(
            "Processing field values for account note:-{:?}-",
            field_values
        );

        let symbol = field_values
            .get(offset)
            .ok_or_else(|| anyhow!("No ticker symbol"))?;
        let date = field_values
            .get(1 + offset)
            .ok_or_else(|| anyhow!("No quantity found"))?;
        let quantity_str = field_values
            .get(2 + offset)
            .ok_or_else(|| anyhow!("No mult found"))?;
        let quantity = Decimal::from_str(&decimal::normalize_str(quantity_str))?;
        let operation = if quantity.is_sign_negative() {
            BrokerOperation::Sell
        } else {
            BrokerOperation::Buy
        };
        let price = field_values
            .get(3 + offset)
            .ok_or_else(|| anyhow!("No price found"))?;
        let value = field_values
            .get(5 + offset)
            .ok_or_else(|| anyhow!("No value found"))?;
        let commision = field_values
            .get(6 + offset)
            .ok_or_else(|| anyhow!("No value found"))?;
        let _earnings = field_values
            .get(8 + offset)
            .ok_or_else(|| anyhow!("No value found"))?;
        let company_info = if let Some(company) = self.companies_info.get(*symbol) {
            company.clone()
        } else {
            log::error!("Not company info found for {}", symbol);
            CompanyInfo {
                name: symbol.to_string(),
                isin: "".to_string(),
            }
        };

        Ok(AccountNote::new(
            NaiveDate::parse_from_str(date, "%Y-%m-%d, %H:%M:%S")?,
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
        log::debug!("parsing account notes");

        if let Some(transactions) = self.dom.select(&TRANSACTIONS_SELECTOR).next() {
            let mut state = NoteState::Invalid;
            let mut with_account_field = false;

            for table_row in transactions.select(&THEAD_TH_TR_SELECTOR) {
                let row_values = table_row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
                log::debug!("Processing header in account notes:-{:?}-", row_values);
                if row_values[0] == "Account" {
                    with_account_field = true;
                }
            }

            for table_row in transactions.select(&TBODY_TR_SELECTOR) {
                match state {
                    NoteState::Invalid => {
                        log::debug!("Invalid state");
                        if STOCKS_STRS.contains(&table_row.text().next()) {
                            state = NoteState::Stocks;
                        }
                    }
                    NoteState::Stocks => {
                        log::debug!("Stocks state");
                        let has_class = |x: &Element| {
                            x.has_class("header-currency", CaseSensitivity::AsciiCaseInsensitive)
                        };
                        if table_row
                            .first_child()
                            .map(|x| x.value())
                            .unwrap()
                            .as_element()
                            .map(has_class)
                            == Some(true)
                        {
                            state = NoteState::Note;
                        } else {
                            state = NoteState::Invalid;
                        }
                    }
                    NoteState::Note => {
                        log::debug!("Note state");
                        let has_class = |x: &Element| {
                            x.has_class("header-asset", CaseSensitivity::AsciiCaseInsensitive)
                        };
                        let element = table_row.value();

                        if element.has_class("row-summary", CaseSensitivity::AsciiCaseInsensitive) {
                            result.push(self.parse_account_note(&table_row, with_account_field)?);
                        } else if table_row
                            .first_child()
                            .map(|x| x.value())
                            .unwrap()
                            .as_element()
                            .map(has_class)
                            == Some(true)
                        {
                            state = NoteState::Invalid;
                        }
                    }
                    NoteState::Total => {
                        log::debug!("Total state");
                    }
                }
            }
        }

        Ok(result)
    }

    fn parse_companies_info(dom: &Html) -> Result<HashMap<String, CompanyInfo>> {
        log::debug!("parse companies info");
        let mut result: HashMap<String, CompanyInfo> = HashMap::new();

        for table_contract_info in dom.select(&CONTRACT_INFO_SELECTOR) {
            let mut start_parsing_symbols = false;

            for table_row in table_contract_info.select(&TR_SELECTOR) {
                log::debug!("table row: {:?}", table_row.inner_html());

                if let Some(element) = table_row.first_child().unwrap().value().as_element() {
                    if element.has_class("header-asset", CaseSensitivity::AsciiCaseInsensitive) {
                        start_parsing_symbols = STOCKS_STRS.contains(&table_row.text().next());
                        continue;
                    }
                }

                if start_parsing_symbols {
                    let field_values = table_row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
                    if field_values.is_empty() {
                        continue;
                    }
                    log::debug!("field values: {:?}", field_values);
                    let ticker = field_values
                        .first()
                        .ok_or_else(|| anyhow!("No company ticker found"))?;
                    let name = field_values
                        .get(1)
                        .ok_or_else(|| anyhow!("No company name found"))?;
                    let isin = field_values
                        .get(3)
                        .ok_or_else(|| anyhow!("No company isin found"))?;

                    result.insert(
                        String::from(*ticker),
                        CompanyInfo {
                            name: String::from(*name),
                            isin: String::from(*isin),
                        },
                    );
                }
            }
        }

        Ok(result)
    }

    fn parse_balance_note(
        &self,
        row: &ElementRef<'_>,
        currency: Option<&str>,
    ) -> Result<BalanceNote> {
        let field_values = row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
        log::debug!(
            "Processing field values for balance note:-{:?}-",
            field_values
        );

        let symbol = field_values
            .first()
            .ok_or_else(|| anyhow!("No ticker symbol"))?;
        let quantity = field_values
            .get(1)
            .ok_or_else(|| anyhow!("No quantity found"))?;
        let mult = field_values
            .get(2)
            .ok_or_else(|| anyhow!("No mult found"))?;
        let price = field_values
            .get(5)
            .ok_or_else(|| anyhow!("No price found"))?;
        let value_in_euro = field_values
            .get(6)
            .ok_or_else(|| anyhow!("No value found"))?;
        let company_info = self
            .companies_info
            .get(*symbol)
            .cloned()
            .or_else(|| {
                log::error!("Not company info found for {}", symbol);
                Some(CompanyInfo {
                    name: symbol.to_string(),
                    isin: "".to_string(),
                })
            })
            .unwrap();

        Ok(BalanceNote::new(
            company_info,
            String::from(""),
            Decimal::from_str(&decimal::normalize_str(quantity))?
                * Decimal::from_str(&decimal::normalize_str(mult))?,
            String::from(currency.unwrap_or(IBParser::EUR_CURRENCY_STR)),
            Decimal::from_str(&decimal::normalize_str(price))?,
            Decimal::from_str(&decimal::normalize_str(value_in_euro))?,
            &self.broker,
        ))
    }

    pub fn parse_balance_notes(&self) -> Result<BalanceNotes> {
        log::debug!("parsing balance notes");
        let mut result = Vec::new();

        if let Some(table_open_positions) = self.dom.select(&OPEN_POSITIONS_SELECTOR).next() {
            let mut state = NoteState::Invalid;
            let mut current_notes: BalanceNotes = Vec::new();
            let mut currency = None;

            for table_row in table_open_positions.select(&TBODY_TR_SELECTOR) {
                match state {
                    NoteState::Invalid => {
                        log::debug!("Invalid state");
                        if STOCKS_STRS.contains(&table_row.text().next()) {
                            state = NoteState::Stocks;
                        }
                    }
                    NoteState::Stocks => {
                        log::debug!("Stocks state");
                        let has_class = |x: &Element| {
                            x.has_class("header-currency", CaseSensitivity::AsciiCaseInsensitive)
                        };
                        if table_row
                            .first_child()
                            .map(|x| x.value())
                            .unwrap()
                            .as_element()
                            .map(has_class)
                            == Some(true)
                        {
                            currency = table_row.text().next();
                            state = NoteState::Note;
                        } else {
                            state = NoteState::Invalid;
                        }
                    }
                    NoteState::Note => {
                        log::debug!("Note state");
                        if table_row
                            .value()
                            .has_class("total", CaseSensitivity::AsciiCaseInsensitive)
                            || table_row
                                .value()
                                .has_class("subtotal", CaseSensitivity::AsciiCaseInsensitive)
                        {
                            if currency == Some(IBParser::EUR_CURRENCY_STR) {
                                state = NoteState::Stocks;
                                result.append(&mut current_notes);
                            } else {
                                state = NoteState::Total;
                            }
                        } else {
                            let balance_note_result = self.parse_balance_note(&table_row, currency);
                            match balance_note_result {
                                Ok(balance_note) => current_notes.push(balance_note),
                                Err(msg) => {
                                    log::error!("Error parsing balance note: {}", msg);
                                    return Err(msg);
                                }
                            }
                        }
                    }
                    NoteState::Total => {
                        log::debug!("Total state");
                        if table_row
                            .value()
                            .has_class("total", CaseSensitivity::AsciiCaseInsensitive)
                        {
                            state = NoteState::Stocks;
                            let field_values =
                                table_row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
                            let total_in_euro_str = field_values
                                .get(5)
                                .ok_or_else(|| anyhow!("Unable to get total in euro"))?;
                            let total_in_euro =
                                Decimal::from_str(&decimal::normalize_str(total_in_euro_str))?;
                            log::debug!("total in eur: {:?}", total_in_euro);
                            util::recalculate_balance_notes(&mut current_notes, &total_in_euro)?;
                        } else {
                            state = NoteState::Invalid;
                        }
                        result.append(&mut current_notes);
                    }
                }
            }
        } else {
            bail!("Unable to find div with open positions");
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ctor::ctor]
    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    #[allow(clippy::mistyped_literal_suffixes)]
    fn ibparser_parse_account_notes_test() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Interactive Brokers"),
            String::from("IE"),
        ));
        let ibparser = IBParser::new(DEFAULT_HTML_TEST, &ib_broker).unwrap();
        let notes = ibparser.parse_account_notes().unwrap();

        let acc_notes = vec![
            AccountNote::new(
                NaiveDate::from_ymd_opt(2019, 4, 16).unwrap(),
                CompanyInfo {
                    name: String::from("EUROTECH SPA"),
                    isin: String::from("IT0003895668"),
                },
                BrokerOperation::Sell,
                Decimal::new(1500, 0),
                Decimal::new(4_0550, 4),
                Decimal::new(6082_50, 2),
                Decimal::new(6_08, 2),
                &ib_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2019, 9, 12).unwrap(),
                CompanyInfo {
                    name: String::from("PROSUS NV"),
                    isin: String::from("NL0013654783"),
                },
                BrokerOperation::Buy,
                Decimal::new(45, 0),
                Decimal::new(73_5000, 4),
                Decimal::new(3307_50, 2),
                Decimal::new(4_00, 2),
                &ib_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2019, 9, 11).unwrap(),
                CompanyInfo {
                    name: String::from("TFF GROUP"),
                    isin: String::from("FR0013295789"),
                },
                BrokerOperation::Buy,
                Decimal::new(90, 0),
                Decimal::new(35_4388889, 7),
                Decimal::new(3189_50, 2),
                Decimal::new(4_00, 2),
                &ib_broker,
            ),
            AccountNote::new(
                NaiveDate::from_ymd_opt(2019, 2, 15).unwrap(),
                CompanyInfo {
                    name: String::from("TEEKAY CORP"),
                    isin: String::from("MHY8564W1030"),
                },
                BrokerOperation::Buy,
                Decimal::new(14, 0),
                Decimal::new(3_8800, 4),
                Decimal::new(54_32, 2),
                Decimal::new(7, 2),
                &ib_broker,
            ),
        ];

        assert_eq!(acc_notes, notes);
    }

    #[test]
    #[allow(clippy::mistyped_literal_suffixes)]
    fn ibparser_parse_balance_notes_test() {
        let ib_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Interactive Brokers"),
            String::from("IE"),
        ));
        let ibparser = IBParser::new(DEFAULT_HTML_TEST, &ib_broker).unwrap();
        let notes = ibparser.parse_balance_notes().unwrap();

        let bal_notes = vec![
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("UMANIS - REG"),
                    isin: String::from("FR0013263878"),
                },
                String::from(""),
                Decimal::new(300, 0),
                String::from("EUR"),
                Decimal::new(5_7600, 4),
                Decimal::new(1728_00, 2),
                &ib_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("FIGEAC-AERO"),
                    isin: String::from("FR0011665280"),
                },
                String::from(""),
                Decimal::new(70, 0),
                String::from("EUR"),
                Decimal::new(9_3000, 4),
                Decimal::new(651, 0),
                &ib_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("PROSUS NV"),
                    isin: String::from("NL0013654783"),
                },
                String::from(""),
                Decimal::new(45, 0),
                String::from("EUR"),
                Decimal::new(66_5300, 4),
                Decimal::new(2993_85, 2),
                &ib_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("TFF GROUP"),
                    isin: String::from("FR0013295789"),
                },
                String::from(""),
                Decimal::new(90, 0),
                String::from("EUR"),
                Decimal::new(36_7000, 4),
                Decimal::new(3303_00, 2),
                &ib_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("JD.COM INC-ADR"),
                    isin: String::from("47215P106"),
                },
                String::from(""),
                Decimal::new(200, 0),
                String::from("USD"),
                Decimal::new(35_2300, 4),
                Decimal::new(6283_91, 2),
                &ib_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("LIBERTY LATIN AMERIC-CL C"),
                    isin: String::from("BMG9001E1286"),
                },
                String::from(""),
                Decimal::new(100, 0),
                String::from("USD"),
                Decimal::new(19_4600, 4),
                Decimal::new(1735_52, 2),
                &ib_broker,
            ),
            BalanceNote::new(
                CompanyInfo {
                    name: String::from("TEEKAY CORP"),
                    isin: String::from("MHY8564W1030"),
                },
                String::from(""),
                Decimal::new(1044, 0),
                String::from("USD"),
                Decimal::new(5_3200, 4),
                Decimal::new(4953_35, 2),
                &ib_broker,
            ),
        ];

        assert_eq!(bal_notes, notes);
    }

    const DEFAULT_HTML_TEST: &str = include_str!("testdata/ib_test.html");
}
