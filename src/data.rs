use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{convert::From, sync::Arc};

pub type AccountNotes = Vec<AccountNote>;
pub type BalanceNotes = Vec<BalanceNote>;

pub const SPAIN_COUNTRY_CODE: &str = "ES";

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum BrokerOperation {
    Buy,
    Sell,
}

impl From<&str> for BrokerOperation {
    fn from(item: &str) -> Self {
        let c = item.chars().next().unwrap();
        match c {
            'V' | 'v' => BrokerOperation::Sell,
            'C' | 'c' => BrokerOperation::Buy,
            _ => unimplemented!("no other broker operations supported"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CompanyInfo {
    pub name: String,
    pub isin: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AccountNote {
    pub date: NaiveDate,
    pub company: CompanyInfo,
    pub operation: BrokerOperation,
    pub quantity: Decimal,
    pub price: Decimal,
    pub value: Decimal,
    pub commision: Decimal,
    pub broker: Arc<BrokerInformation>,
}

impl AccountNote {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        date: NaiveDate,
        company: CompanyInfo,
        operation: BrokerOperation,
        quantity: Decimal,
        price: Decimal,
        value: Decimal,
        commision: Decimal,
        broker: &Arc<BrokerInformation>,
    ) -> AccountNote {
        AccountNote {
            date,
            company,
            operation,
            quantity,
            price,
            value,
            commision,
            broker: Arc::clone(broker),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BalanceNote {
    pub company: CompanyInfo,
    pub market: String,
    pub quantity: Decimal,
    pub currency: String,
    pub price: Decimal,
    pub value_in_euro: Decimal,
    pub broker: Arc<BrokerInformation>,
}

impl BalanceNote {
    pub fn new(
        company: CompanyInfo,
        market: String,
        quantity: Decimal,
        currency: String,
        price: Decimal,
        value_in_euro: Decimal,
        broker: &Arc<BrokerInformation>,
    ) -> BalanceNote {
        BalanceNote {
            company,
            market,
            quantity,
            currency,
            price,
            value_in_euro,
            broker: Arc::clone(broker),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BrokerInformation {
    pub name: String,
    pub country_code: String,
}

impl BrokerInformation {
    pub fn new(name: String, cc: String) -> Self {
        Self {
            name,
            country_code: cc,
        }
    }
}
pub struct FinancialInformation {
    pub account_notes: AccountNotes,
    pub balance_notes: BalanceNotes,
    pub name: String,
    pub surname: String,
    pub nif: String,
    pub year: usize,
    pub phone: String,
}

impl FinancialInformation {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            account_notes: vec![],
            balance_notes: vec![],
            name: String::from(""),
            surname: String::from(""),
            nif: String::from(""),
            year: 0,
            phone: String::from(""),
        }
    }

    pub fn full_name(&self) -> String {
        self.surname.clone() + " " + &self.name
    }
}
