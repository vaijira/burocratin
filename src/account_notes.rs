use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::convert::From;

pub type AccountNotes = Vec<AccountNote>;
pub type BalanceNotes = Vec<BalanceNote>;

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CompanyInfo {
    pub name: String,
    pub isin: String,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AccountNote {
    pub date: NaiveDate,
    pub company: CompanyInfo,
    pub operation: BrokerOperation,
    pub quantity: Decimal,
    pub price: Decimal,
    pub value: Decimal,
    pub value_in_euro: Decimal,
    pub commision: Decimal,
    pub exchange_rate: Decimal,
    pub earnings: Decimal,
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
        value_in_euro: Decimal,
        commision: Decimal,
        exchange_rate: Decimal,
        earnings: Decimal,
    ) -> AccountNote {
        AccountNote {
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
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BalanceNote {
    pub company: CompanyInfo,
    pub market: String,
    pub quantity: Decimal,
    pub currency: String,
    pub price: Decimal,
    pub value_in_euro: Decimal,
}

impl BalanceNote {
    pub fn new(
        company: CompanyInfo,
        market: String,
        quantity: Decimal,
        currency: String,
        price: Decimal,
        value_in_euro: Decimal,
    ) -> BalanceNote {
        BalanceNote {
            company,
            market,
            quantity,
            currency,
            price,
            value_in_euro,
        }
    }
}
