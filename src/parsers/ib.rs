use std::{collections::HashMap, rc::Rc, str::FromStr};

use crate::{
    data::{
        AccountNote, AccountNotes, BalanceNote, BalanceNotes, BrokerInformation, BrokerOperation,
        CompanyInfo,
    },
    utils::decimal,
};
use anyhow::{anyhow, bail, Result};
use chrono::NaiveDate;
use once_cell::sync::Lazy;
use rust_decimal::Decimal;
use scraper::{node::Element, ElementRef, Html, Selector};
use selectors::attr::CaseSensitivity;

static OPEN_POSITIONS_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(r#"div[id^="tblOpenPositions_"] div table"#).unwrap());

static CONTRACT_INFO_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(r#"div[id^="tblContractInfo"] div table"#).unwrap());

static TRANSACTIONS_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(r#"div[id^="tblTransactions_"] div table"#).unwrap());

static TBODY_TR_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(r#"tbody tr"#).unwrap());
static TR_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse(r#"tr"#).unwrap());

enum NoteState {
    Invalid,
    Stocks,
    Note,
    Total,
}

pub struct IBParser {
    dom: Html,
    broker: Rc<BrokerInformation>,
    companies_info: HashMap<String, CompanyInfo>,
}

impl IBParser {
    const STOCKS_STR: &'static str = "Stocks";
    const EUR_CURRENCY_STR: &'static str = "EUR";

    pub fn new(data: &str, broker: &Rc<BrokerInformation>) -> Result<Self> {
        let dom = Html::parse_document(data);
        let companies_info = IBParser::parse_companies_info(&dom)?;

        Ok(Self {
            dom,
            broker: Rc::clone(broker),
            companies_info,
        })
    }

    fn parse_account_note(&self, row: &ElementRef<'_>) -> Result<AccountNote> {
        let field_values = row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
        log::debug!(
            "Processing field values for acount note:-{:?}-",
            field_values
        );

        let symbol = field_values
            .get(0)
            .ok_or_else(|| anyhow!("No ticker symbol"))?;
        let date = field_values
            .get(1)
            .ok_or_else(|| anyhow!("No quantity found"))?;
        let quantity_str = field_values
            .get(2)
            .ok_or_else(|| anyhow!("No mult found"))?;
        let quantity = Decimal::from_str(&decimal::normalize_str(quantity_str))?;
        let operation = if quantity.is_sign_negative() {
            BrokerOperation::Sell
        } else {
            BrokerOperation::Buy
        };
        let price = field_values
            .get(3)
            .ok_or_else(|| anyhow!("No price found"))?;
        let value = field_values
            .get(5)
            .ok_or_else(|| anyhow!("No value found"))?;
        let commision = field_values
            .get(6)
            .ok_or_else(|| anyhow!("No value found"))?;
        let _earnings = field_values
            .get(8)
            .ok_or_else(|| anyhow!("No value found"))?;
        let company_info = self
            .companies_info
            .get(*symbol)
            .ok_or_else(|| anyhow!("Not company info found"))?;

        Ok(AccountNote::new(
            NaiveDate::parse_from_str(date, "%Y-%m-%d, %H:%M:%S")?,
            company_info.clone(),
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

            for table_row in transactions.select(&TBODY_TR_SELECTOR) {
                match state {
                    NoteState::Invalid => {
                        log::debug!("Invalid state");
                        if table_row.text().next() == Some(IBParser::STOCKS_STR) {
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
                            .map(|x| has_class(x))
                            == Some(true)
                        {
                            state = NoteState::Note;
                        } else {
                            state = NoteState::Invalid;
                        }
                    }
                    NoteState::Note => {
                        log::debug!("Note state");
                        let element = table_row.value();

                        if element.has_class("row-summary", CaseSensitivity::AsciiCaseInsensitive) {
                            result.push(self.parse_account_note(&table_row)?);
                        } else if element
                            .has_class("header-asset", CaseSensitivity::AsciiCaseInsensitive)
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

        if let Some(table_contract_info) = dom.select(&CONTRACT_INFO_SELECTOR).next() {
            let mut start_parsing_symbols = false;

            for table_row in table_contract_info.select(&TR_SELECTOR) {
                log::debug!("table row: {:?}", table_row.inner_html());

                if let Some(element) = table_row.first_child().unwrap().value().as_element() {
                    if element.has_class("header-asset", CaseSensitivity::AsciiCaseInsensitive) {
                        if table_row.text().next() == Some(IBParser::STOCKS_STR) {
                            start_parsing_symbols = true;
                        } else {
                            start_parsing_symbols = false;
                        }
                        continue;
                    }
                }

                if start_parsing_symbols {
                    let field_values = table_row.text().filter(|x| *x != "\n").collect::<Vec<_>>();
                    log::debug!("field values: {:?}", field_values);
                    let ticker = field_values
                        .get(0)
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
        } else {
            bail!("Unable to parse contract info");
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
            .get(0)
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
            .ok_or_else(|| anyhow!("Not company info found"))?;

        Ok(BalanceNote::new(
            company_info.clone(),
            String::from(""),
            Decimal::from_str(&decimal::normalize_str(quantity))?
                * Decimal::from_str(&decimal::normalize_str(mult))?,
            String::from(currency.or(Some(IBParser::EUR_CURRENCY_STR)).unwrap()),
            Decimal::from_str(&decimal::normalize_str(price))?,
            Decimal::from_str(&decimal::normalize_str(value_in_euro))?,
            &self.broker,
        ))
    }

    fn recalculate_balance_notes(
        &self,
        notes: &mut BalanceNotes,
        total_in_euro: &Decimal,
    ) -> Result<()> {
        let total = notes
            .iter()
            .fold(Decimal::new(0, 2), |acc, x| acc + x.price * x.quantity);
        for note in notes {
            note.value_in_euro = ((note.value_in_euro * total_in_euro) / total).round_dp(2);
        }

        Ok(())
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
                        if table_row.text().next() == Some(IBParser::STOCKS_STR) {
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
                            .map(|x| has_class(x))
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
                        {
                            if currency == Some(IBParser::EUR_CURRENCY_STR) {
                                state = NoteState::Stocks;
                                result.append(&mut current_notes);
                            } else {
                                state = NoteState::Total;
                            }
                        } else {
                            current_notes.push(self.parse_balance_note(&table_row, currency)?);
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
                            self.recalculate_balance_notes(&mut current_notes, &total_in_euro)?;
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
        let ib_broker: Rc<BrokerInformation> = Rc::new(BrokerInformation::new(
            String::from("Interactive Brokers"),
            String::from("IE"),
        ));
        let ibparser = IBParser::new(DEFAULT_HTML_TEST, &ib_broker).unwrap();
        let notes = ibparser.parse_account_notes().unwrap();

        let acc_notes = vec![
            AccountNote::new(
                NaiveDate::from_ymd(2019, 4, 16),
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
                NaiveDate::from_ymd(2019, 9, 12),
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
                NaiveDate::from_ymd(2019, 9, 11),
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
                NaiveDate::from_ymd(2019, 2, 15),
                CompanyInfo {
                    name: String::from("TEEKAY CORP"),
                    isin: String::from("MHY8564W1030"),
                },
                BrokerOperation::Buy,
                Decimal::new(14, 0),
                Decimal::new(3_8800, 4),
                Decimal::new(54_32, 2),
                Decimal::new(0_07, 2),
                &ib_broker,
            ),
        ];

        assert_eq!(acc_notes, notes);
    }

    #[test]
    #[allow(clippy::mistyped_literal_suffixes)]
    fn ibparser_parse_balance_notes_test() {
        let ib_broker: Rc<BrokerInformation> = Rc::new(BrokerInformation::new(
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

    const DEFAULT_HTML_TEST: &str = r#"
<html>
<body>
<div class="sectionHeadingClosed" id="secTransactions_XXXXXXXHeading" onClick="javascript:showHide('tblTransactions_XXXXXXXBody', 'secTransactions_XXXXXXXHeading');"><span class="accordion-icon"></span>Trades<span class="btn-group-right"><a href="javascript:void(0);" class="btn-icon no-print" onclick="javascript:openWin('https://www.interactivebrokers.com/en/software/reportguide/reportguide/trades_default.htm', event)" ></a></span>
</div>
<div id="tblTransactions_XXXXXXXBody" class="sectionContent" style="position: absolute; display: none">
<div class="table-responsive">
<table width="100%" cellpadding="0" cellspacing="0" border="0" class="table table-bordered" id="summaryDetailTable">
<thead>
<tr>
<th align="left">Symbol</th>
<th align="left">Date/Time</th>
<th align="right">Quantity</th>
<th align="right">T. Price</th>
<th align="right">C. Price</th>
<th align="right">Proceeds</th>
<th align="right">Comm/Fee</th>
<th align="right">Basis</th>
<th align="right">Realized P/L</th>
<th align="right">Realized P/L %</th>
<th align="right">MTM P/L</th>
<th align="right">Code</th>
</tr>
</thead>
<tbody>
<tr><td class="header-asset" align="left" valign="middle" colspan="12">Stocks</td>
</tr>
</tbody>
<tbody>
<tr><td class="header-currency" align="left" valign="middle" colspan="12">EUR</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>E5T</td>
<td>2019-04-16, 04:19:18</td>
<td align="right">-1,500</td>
<td align="right">4.0550</td>
<td align="right">3.9900</td>
<td align="right">6,082.50</td>
<td align="right">-6.08</td>
<td align="right">-2,137.26</td>
<td align="right">3,939.16</td>
<td align="right">184.31</td>
<td align="right">97.50</td>
<td align="right">C;P</td>
</tr>
</tbody>
<tbody>
<tr class="subtotal">
<td class="indent" colspan="2">Total&nbsp;E5T</td>
<td align="right">-1,500</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">6,082.50</td>
<td align="right">-6.08</td>
<td align="right">-2,137.26</td>
<td align="right">3,939.16</td>
<td align="right">184.31</td>
<td align="right">97.50</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>PRX</td>
<td>2019-09-12, 07:18:31</td>
<td align="right">45</td>
<td align="right">73.5000</td>
<td align="right">72.2500</td>
<td align="right">-3,307.50</td>
<td align="right">-4.00</td>
<td align="right">3,311.50</td>
<td align="right">0.00</td>
<td align="right">0.00</td>
<td align="right">-56.25</td>
<td align="right">O;P</td>
</tr>
</tbody>
<tbody>
<tr class="subtotal">
<td class="indent" colspan="2">Total&nbsp;PRX</td>
<td align="right">45</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">-3,307.50</td>
<td align="right">-4.00</td>
<td align="right">3,311.50</td>
<td align="right">0.00</td>
<td align="right">0.00</td>
<td align="right">-56.25</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>TFF</td>
<td>2019-09-11, 05:11:01</td>
<td align="right">90</td>
<td align="right">35.4388889</td>
<td align="right">35.5000</td>
<td align="right">-3,189.50</td>
<td align="right">-4.00</td>
<td align="right">3,193.50</td>
<td align="right">0.00</td>
<td align="right">0.00</td>
<td align="right">5.50</td>
<td align="right">O;P</td>
</tr>
</tbody>
<tbody>
<tr class="subtotal">
<td class="indent" colspan="2">Total&nbsp;TFF</td>
<td align="right">90</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">-3,189.50</td>
<td align="right">-4.00</td>
<td align="right">3,193.50</td>
<td align="right">0.00</td>
<td align="right">0.00</td>
<td align="right">5.50</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="total">
<td class="indent" colspan="5">Total</td>
<td align="right">-414.50</td>
<td align="right">-14.08</td>
<td align="right">4,367.74</td>
<td align="right">3,939.16</td>
<td>&nbsp;</td>
<td align="right">46.75</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr><td class="header-currency" align="left" valign="middle" colspan="12">USD</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>TK</td>
<td>2019-02-15, 09:31:10</td>
<td align="right">14</td>
<td align="right">3.8800</td>
<td align="right">3.8100</td>
<td align="right">-54.32</td>
<td align="right">-0.07</td>
<td align="right">54.39</td>
<td align="right">0.00</td>
<td align="right">0.00</td>
<td align="right">-0.98</td>
<td align="right">O;R</td>
</tr>
</tbody>
<tbody>
<tr class="subtotal">
<td class="indent" colspan="2">Total&nbsp;TK</td>
<td align="right">14</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">-54.32</td>
<td align="right">-0.07</td>
<td align="right">54.39</td>
<td align="right">0.00</td>
<td align="right">0.00</td>
<td align="right">-0.98</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="total">
<td class="indent" colspan="5">Total</td>
<td align="right">-54.32</td>
<td align="right">-0.07</td>
<td align="right">54.39</td>
<td align="right">0.00</td>
<td>&nbsp;</td>
<td align="right">-0.98</td>
<td>&nbsp;</td>
</tr>
<tr class="total">
<td class="indent" colspan="5">Total in&nbsp;EUR</td>
<td align="right">-48.08</td>
<td align="right">-0.06</td>
<td align="right">48.15</td>
<td align="right">0.00</td>
<td>&nbsp;</td>
<td align="right">-0.87</td>
<td>&nbsp;</td>
</tr>
</tbody>
</table>
</div>
</div>
<div id="tblOpenPositions_XXXXXXXBody" class="sectionContent" style="position: absolute; display: none">
<div class="table-responsive">
<table width="100%" cellpadding="0" cellspacing="0" border="0" class="table table-bordered" id="summaryDetailTable">
<thead>
<tr>
<th align="left">Symbol</th>
<th align="right">Quantity</th>
<th align="right">Mult</th>
<th align="right">Cost Price</th>
<th align="right">Cost Basis</th>
<th align="right">Close Price</th>
<th align="right">Value</th>
<th align="right">Unrealized P/L</th>
<th align="right">Unrealized P/L %</th>
<th align="right">Code</th>
</tr>
</thead>
<tbody>
<tr><td class="header-asset" align="left" valign="middle" colspan="10">Stocks</td>
</tr>
</tbody>
<tbody>
<tr><td class="header-currency" align="left" valign="middle" colspan="10">EUR</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>ALUMS</td>
<td align="right" valign="bottom">300</td>
<td align="right">1</td>
<td align="right">8.7042</td>
<td align="right">2,611.26</td>
<td align="right" valign="bottom">5.7600</td>
<td align="right">1,728.00</td>
<td align="right">-883.26</td>
<td align="right">-33.83</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>FGA</td>
<td align="right" valign="bottom">70</td>
<td align="right">1</td>
<td align="right">17.3971429</td>
<td align="right">1,217.80</td>
<td align="right" valign="bottom">9.3000</td>
<td align="right">651.00</td>
<td align="right">-566.80</td>
<td align="right">-46.54</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>PRX</td>
<td align="right" valign="bottom">45</td>
<td align="right">1</td>
<td align="right">73.5888889</td>
<td align="right">3,311.50</td>
<td align="right" valign="bottom">66.5300</td>
<td align="right">2,993.85</td>
<td align="right">-317.65</td>
<td align="right">-9.59</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>TFF</td>
<td align="right" valign="bottom">90</td>
<td align="right">1</td>
<td align="right">35.4833333</td>
<td align="right">3,193.50</td>
<td align="right" valign="bottom">36.7000</td>
<td align="right">3,303.00</td>
<td align="right">109.50</td>
<td align="right">3.43</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="total">
<td class="indent" colspan="2">Total</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">10,334.06</td>
<td>&nbsp;</td>
<td align="right">8,675.85</td>
<td align="right">-1,658.21</td>
<td>&nbsp;</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr><td class="header-currency" align="left" valign="middle" colspan="10">USD</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>JD</td>
<td align="right" valign="bottom">200</td>
<td align="right">1</td>
<td align="right">38.4050</td>
<td align="right">7,681.00</td>
<td align="right" valign="bottom">35.2300</td>
<td align="right">7,046.00</td>
<td align="right">-635.00</td>
<td align="right">-8.27</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>LILAK</td>
<td align="right" valign="bottom">100</td>
<td align="right">1</td>
<td align="right">20.669199</td>
<td align="right">2,066.92</td>
<td align="right" valign="bottom">19.4600</td>
<td align="right">1,946.00</td>
<td align="right">-120.92</td>
<td align="right">-5.85</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="row-summary no-details">
<td>TK</td>
<td align="right" valign="bottom">1,044</td>
<td align="right">1</td>
<td align="right">6.6989311</td>
<td align="right">6,993.68</td>
<td align="right" valign="bottom">5.3200</td>
<td align="right">5,554.08</td>
<td align="right">-1,439.60</td>
<td align="right">-20.58</td>
<td align="right">&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="total">
<td class="indent" colspan="2">Total</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">16,741.60</td>
<td>&nbsp;</td>
<td align="right">14,546.08</td>
<td align="right">-2,195.52</td>
<td>&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr class="total">
<td class="indent" colspan="2">Total in&nbsp;EUR</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">14,930.83</td>
<td>&nbsp;</td>
<td align="right">12,972.78</td>
<td align="right">-1,958.06</td>
<td>&nbsp;</td>
<td>&nbsp;</td>
</tr>
</tbody>
<tbody>
<tr class="total">
<td class="indent" colspan="2">Total&nbsp;Stocks&nbsp;in&nbsp;EUR</td>
<td align="right">&nbsp;</td>
<td>&nbsp;</td>
<td align="right">25,264.89</td>
<td>&nbsp;</td>
<td align="right">21,648.63</td>
<td align="right">-3,616.27</td>
<td>&nbsp;</td>
<td>&nbsp;</td>
</tr>
</tbody>
</table>
</div>
</div>
<div class="sectionHeadingClosed" id="secContractInfoXXXXXXXHeading" onClick="javascript:showHide('tblContractInfoXXXXXXXBody', 'secContractInfoXXXXXXXHeading');"><span class="accordion-icon"></span>Financial Instrument Information<span class="btn-group-right"><a href="javascript:void(0);" class="btn-icon no-print" onclick="javascript:openWin('https://www.interactivebrokers.com/en/software/reportguide/reportguide/financialinstrumentinformation_default.htm', event)" ></a></span>
</div>
<div id="tblContractInfoXXXXXXXBody" class="sectionContent" style="position: absolute; display: none">
<div class="table-responsive">
<table width="100%" cellpadding="0" cellspacing="0" border="0" class="table table-bordered">
<thead>
<tr>
<th align="left">Symbol</th>
<th align="left">Description</th>
<th align="left">Conid</th>
<th align="left">Security ID</th>
<th align="left">Multiplier</th>
<th align="left" class="no-border-left no-border-right">&nbsp;</th>
<th align="left" class="no-border-left no-border-right">&nbsp;</th>
<th align="left">Type</th>
<th align="left" class="no-border-left no-border-right">&nbsp;</th>
<th align="left" class="no-border-left no-border-right">&nbsp;</th>
<th align="left" class="no-border-left no-border-right">&nbsp;</th>
<th align="left" class="no-border-left no-border-right">&nbsp;</th>
<th align="left">Code</th>
</tr>
</thead>
<tr><td class="header-asset" align="left" valign="middle" colspan="13">Stocks</td>
</tr>
<tr>
<td class="no-border-left">ALUMS</td>
<td>UMANIS - REG</td>
<td>282008165</td>
<td>FR0013263878</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">E5T</td>
<td>EUROTECH SPA</td>
<td>73400808</td>
<td>IT0003895668</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">FGA</td>
<td>FIGEAC-AERO</td>
<td>140392713</td>
<td>FR0011665280</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">JD</td>
<td>JD.COM INC-ADR</td>
<td>152486141</td>
<td>47215P106</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>ADR</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">LILAK</td>
<td>LIBERTY LATIN AMERIC-CL C</td>
<td>301303327</td>
<td>BMG9001E1286</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">PRX</td>
<td>PROSUS NV</td>
<td>382625193</td>
<td>NL0013654783</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">TFF</td>
<td>TFF GROUP</td>
<td>297781308</td>
<td>FR0013295789</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
<tr>
<td class="no-border-left">TK</td>
<td>TEEKAY CORP</td>
<td>2009270</td>
<td>MHY8564W1030</td>
<td>1</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>COMMON</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td class="no-border-left no-border-right">&nbsp;</td>
<td>&nbsp;</td>
</tr>
</table>
</div>
</div>
</body>
</html>"#;
}
