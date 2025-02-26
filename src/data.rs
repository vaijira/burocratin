use chrono::NaiveDate;
use num_format::Locale;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    convert::From,
    sync::{Arc, LazyLock},
};

pub type AccountNotes = Vec<AccountNote>;
pub type BalanceNotes = Vec<BalanceNote>;
pub type Aeat720Records = Vec<Aeat720Record>;

pub const DEFAULT_YEAR: usize = 2024;
pub const SPAIN_COUNTRY_CODE: &str = "ES";
pub const DEFAULT_LOCALE: &Locale = &Locale::es;
pub const DEFAULT_NUMBER_OF_DECIMALS: u16 = 2;

pub static DEFAULT_BROKER: LazyLock<Arc<BrokerInformation>> = LazyLock::new(|| {
    Arc::new(BrokerInformation {
        name: "Desconocido".to_string(),
        country_code: "IE".to_string(),
    })
});

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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Aeat720Record {
    pub company: CompanyInfo,
    pub quantity: Decimal,
    pub value_in_euro: Decimal,
    pub first_tx_date: usize,
    pub broker: Arc<BrokerInformation>,
    pub percentage: Decimal,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PersonalInformation {
    pub name: String,
    pub surname: String,
    pub nif: String,
    pub year: usize,
    pub phone: String,
}

#[derive(Debug, Eq, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Aeat720Information {
    pub records: Vec<Aeat720Record>,
    pub personal_info: PersonalInformation,
}

impl Aeat720Information {
    pub fn full_name(&self) -> String {
        self.personal_info.surname.clone() + " " + &self.personal_info.name[..]
    }
}
