use crate::data::{AccountNote, BalanceNote, FinancialInformation};
use anyhow::{bail, Result};
use chrono::NaiveDate;
use encoding_rs::ISO_8859_15;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::io::Write;

/*
   aeat 720 model specification.
   https://www.agenciatributaria.es/static_files/AEAT/Contenidos_Comunes/La_Agencia_Tributaria/Ayuda/Disenyos_de_registro/Ayudas/DR_Resto_Modelos/Ficheros/modelo_720.pdf

    Summary Register:

    registerType: NumericField,           // Pos 1   value: '1'
    model: NumericField,                  // Pos 2-4 value: '720'
    year: NumericField,                   // Pos 5-8
    nif: AlphaNumericField,               // Pos 9-17
    name: AlphaNumericField,              // Pos 18-57 Surname name
    transmission: StringField,            // Pos 58 value: 'T'
    telephone: NumericField,              // Pos 59-67
    contactName: AlphaNumericField,       // Pos 68-107
    id: NumericField,                     // Pos 108-120
    complementary: StringField,           // Pos 121 value: 'C' if complementary declaration
    replacement: StringField,             // Pos 122 value: 'S' if replacement declaration
    previousDeclarationId: NumericField,  // Pos 123-135
    totalDetailRegisters: NumericField,   // Pos 136-144
    acquisitionSummarySign: StringField,  // Pos 145 value: 'N' if negative
    acquisitionSummaryInt: NumericField,  // Pos 146-160
    acquisitionSummaryFrac: NumericField, // Pos 161-162
    valuationSummarySign: StringField,    // Pos 163 value: 'N' if negative
    valuationSummaryInt: NumericField,    // Pos 164-178
    valuationSummaryFrac: NumericField,   // Pos 179-180
    blank: StringField,                   // Pos 181-500

    Detail Register:

    registerType: NumericField,               // Pos 1 value: '2'
    model: NumericField,                      // Pos 2-4 value: '720'
    year: NumericField,                       // Pos 5-8
    nif: AlphaNumericField,                   // Pos 9-17
    nifDeclared: AlphaNumericField,           // Pos 18-26
    proxyNif: AlphaNumericField,              // Pos 27-35
    name: AlphaNumericField,                  // Pos 36-75
    declarationType: NumericField,            // Pos 76 value: '1' if owner
    ownershipType: AlphaNumericField,         // Pos 77-101
    assetType: StringField,                   // Pos 102 value: 'V' if stocks
    assetSubType: NumericField,               // Pos 103 value: '1' usually
    realStateAssetType: AlphaNumericField,    // Pos 104-128
    countryCode: StringField,                 // Pos 129-130
    stockIdType: NumericField,                // Pos 131 value: '1' if ISIN
    stockId: AlphaNumericField,               // Pos 132-143 ISIN
    accountIdType: StringField,               // Pos 144
    accountId: AlphaNumericField,             // Pos 145-155 BIC
    accountCode: AlphaNumericField,           // Pos 156-189
    entityName: AlphaNumericField,            // Pos 190-230
    entityNif: AlphaNumericField,             // Pos 231-250
    entityAddress: AlphaNumericField,         // Pos 251-414
    buyingDate: NumericField,                 // Pos 415-422 value: YYYYMMDD format
    buyingType: StringField, // Pos 423 value: 'A' initial 'M' already existed 'C' disposed
    sellingDate: NumericField, // Pos 424-431 value: YYYYMMDD format
    acquisitionSign: StringField, // Pos 432 value: 'N' if negative
    acquisitionInt: NumericField, // Pos 433-444
    acquisitionFrac: NumericField, // Pos 445-446
    valuationSign: StringField, // Pos 447 value: 'N' if negative
    valuationInt: NumericField, // Pos 448-459
    valuationFrac: NumericField, // Pos 460-461
    stockRepresentation: StringField, // Pos 462 value: 'A' usually
    stockQuantityInt: NumericField, // Pos 463-472
    stockQuantityFrac: NumericField, // Pos 473-474
    realStateRepresentation: StringField, // pos 475
    OwnPercentageInt: NumericField, // Pos 476-478
    OwnPercentageFrac: NumericField, // Pos 479-489
    blank: StringField,      // Pos 481-500
*/
const AEAT_720_REGISTER_SIZE_BYTES: usize = 500;
const AEAT_720_DOCUMENT_ID: usize = 720;
const AEAT_720_NEGATIVE_SIGN: &str = "N";

type AeatRegisterArray = [u8; AEAT_720_REGISTER_SIZE_BYTES];

#[derive(Debug, PartialEq)]
enum Aeat720Field {
    AlphaNumeric(usize, usize),
    Numeric(usize, usize),
    String(usize, usize),
}

impl Aeat720Field {
    fn write_field(fields: &mut AeatRegisterArray, field: Aeat720Field, value: &str) -> Result<()> {
        match field {
            Aeat720Field::Numeric(_, _) => {
                let num_value = value.parse::<usize>()?;
                Self::write_numeric_field(fields, field, num_value)?
            }
            Aeat720Field::AlphaNumeric(begin, end) | Aeat720Field::String(begin, end) => {
                let size = (end - begin) + 1;
                let mut slice = &mut fields[begin - 1..end];

                let result = ISO_8859_15.encode(value);
                if result.2 {
                    bail!("Unable to encode to ISO-8859-15")
                } else if result.0.len() > size {
                    slice.write_all(&result.0[0..size])?;
                } else {
                    let remainder = size - result.0.len();
                    slice.write_all(&result.0)?;

                    if remainder > 0 {
                        slice.write_all(" ".repeat(remainder).as_bytes())?;
                    }
                }
            }
        }

        Ok(())
    }

    fn write_numeric_field(
        fields: &mut AeatRegisterArray,
        field: Aeat720Field,
        value: usize,
    ) -> Result<()> {
        if let Aeat720Field::Numeric(begin, end) = field {
            let size = (end - begin) + 1;
            let mut slice = &mut fields[begin - 1..end];
            write!(slice, "{:0width$}", value, width = size)?;
        } else {
            bail!("Expected numeric field but it wasn't {:?}", field);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct SummaryRegister {
    fields: AeatRegisterArray,
}

impl SummaryRegister {
    // Field values
    const AEAT_720_SUMMARY_REGISTER_TYPE: usize = 1;
    const AEAT_720_TRANSMISSION_ASSET: &'static str = "T";

    // Field definitions
    const REGISTER_TYPE_FIELD: Aeat720Field = Aeat720Field::Numeric(1, 1);
    const DOCUMENT_ID_FIELD: Aeat720Field = Aeat720Field::Numeric(2, 4);
    const YEAR_FIELD: Aeat720Field = Aeat720Field::Numeric(5, 8);
    const NIF_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(9, 17);
    const NAME_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(18, 57);
    const TRANSMISSION_FIELD: Aeat720Field = Aeat720Field::String(58, 58);
    const TELEPHONE_FIELD: Aeat720Field = Aeat720Field::Numeric(59, 67);
    const CONTACT_NAME_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(68, 107);
    const SECOND_DOCUMENT_ID_FIELD: Aeat720Field = Aeat720Field::Numeric(108, 110);
    const ID_FIELD: Aeat720Field = Aeat720Field::Numeric(111, 120);
    const COMPLEMENTARY_FIELD: Aeat720Field = Aeat720Field::String(121, 121);
    const REPLACEMENT_FIELD: Aeat720Field = Aeat720Field::String(122, 122);
    const PREVIOUS_DECLARARION_ID_FIELD: Aeat720Field = Aeat720Field::Numeric(123, 135);
    const TOTAL_DETAIL_REGISTERS_FIELD: Aeat720Field = Aeat720Field::Numeric(136, 144);
    const ACQUISITON_SIGN_FIELD: Aeat720Field = Aeat720Field::String(145, 145); // 'N' if negative
    const ACQUISITION_INT_FIELD: Aeat720Field = Aeat720Field::Numeric(146, 160);
    const ACQUISITION_FRACTION_FIELD: Aeat720Field = Aeat720Field::Numeric(161, 162);
    const VALUATION_SIGN_FIELD: Aeat720Field = Aeat720Field::String(163, 163); // 'N' if negative
    const VALUATION_INT_FIELD: Aeat720Field = Aeat720Field::Numeric(164, 178);
    const VALUATION_FRACTION_FIELD: Aeat720Field = Aeat720Field::Numeric(179, 180);
    const REMAINDER_BLANK_FIELD: Aeat720Field = Aeat720Field::String(181, 500);
}

impl Default for SummaryRegister {
    #[allow(unused_must_use)]
    fn default() -> Self {
        let mut fields: AeatRegisterArray = [b' '; AEAT_720_REGISTER_SIZE_BYTES];

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::REGISTER_TYPE_FIELD,
            Self::AEAT_720_SUMMARY_REGISTER_TYPE,
        );

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::DOCUMENT_ID_FIELD,
            AEAT_720_DOCUMENT_ID,
        );

        Aeat720Field::write_numeric_field(&mut fields, Self::YEAR_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::NIF_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::NAME_FIELD, "");

        Aeat720Field::write_field(
            &mut fields,
            Self::TRANSMISSION_FIELD,
            Self::AEAT_720_TRANSMISSION_ASSET,
        );

        Aeat720Field::write_numeric_field(&mut fields, Self::TELEPHONE_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::CONTACT_NAME_FIELD, "");

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::SECOND_DOCUMENT_ID_FIELD,
            AEAT_720_DOCUMENT_ID,
        );

        Aeat720Field::write_numeric_field(&mut fields, Self::ID_FIELD, 1);

        Aeat720Field::write_field(&mut fields, Self::COMPLEMENTARY_FIELD, "");
        Aeat720Field::write_field(&mut fields, Self::REPLACEMENT_FIELD, "");

        Aeat720Field::write_numeric_field(&mut fields, Self::PREVIOUS_DECLARARION_ID_FIELD, 0);
        Aeat720Field::write_numeric_field(&mut fields, Self::TOTAL_DETAIL_REGISTERS_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::ACQUISITON_SIGN_FIELD, "");
        Aeat720Field::write_numeric_field(&mut fields, Self::ACQUISITION_INT_FIELD, 0);
        Aeat720Field::write_numeric_field(&mut fields, Self::ACQUISITION_FRACTION_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::VALUATION_SIGN_FIELD, "");
        Aeat720Field::write_numeric_field(&mut fields, Self::VALUATION_INT_FIELD, 0);
        Aeat720Field::write_numeric_field(&mut fields, Self::VALUATION_FRACTION_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::REMAINDER_BLANK_FIELD, "");

        Self { fields }
    }
}

impl SummaryRegister {
    fn new(notes: &[BalanceNote], year: usize, nif: &str, name: &str, phone: &str) -> Result<Self> {
        let mut fields = Self::default().fields;

        Aeat720Field::write_field(&mut fields, Self::NIF_FIELD, nif)?;

        Aeat720Field::write_numeric_field(&mut fields, Self::YEAR_FIELD, year)?;

        Aeat720Field::write_field(&mut fields, Self::NAME_FIELD, name)?;

        if !phone.is_empty() {
            Aeat720Field::write_field(&mut fields, Self::TELEPHONE_FIELD, phone)?;
        }

        Aeat720Field::write_field(&mut fields, Self::CONTACT_NAME_FIELD, name)?;

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::TOTAL_DETAIL_REGISTERS_FIELD,
            notes.len(),
        )?;

        let mut total_acquisition = Decimal::new(0, 2);

        for note in notes {
            total_acquisition += note.value_in_euro;
        }

        if total_acquisition.is_sign_negative() {
            Aeat720Field::write_field(
                &mut fields,
                Self::ACQUISITON_SIGN_FIELD,
                AEAT_720_NEGATIVE_SIGN,
            )?;
        }

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::ACQUISITION_INT_FIELD,
            total_acquisition.trunc().abs().to_usize().unwrap_or(0),
        )?;

        let mut remainder = total_acquisition.fract();
        remainder.set_scale(0)?;
        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::ACQUISITION_FRACTION_FIELD,
            remainder.to_usize().unwrap_or(0),
        )?;

        Ok(Self { fields })
    }
}

#[derive(Debug)]
struct DetailRegister {
    fields: AeatRegisterArray,
}

impl DetailRegister {
    // Field values
    const AEAT_720_DETAIL_REGISTER_TYPE: usize = 2;
    const AEAT_720_OWNER_TYPE: usize = 1;
    const AEAT_720_ASSET_TYPE: &'static str = "V";
    const AEAT_720_STOCK_ID_TYPE: usize = 1;
    const AEAT_720_ASSET_FIRST_ACQUISITION: &'static str = "A";
    // const AEAT_720_ASSET_INCREMENTAL_ACQUISITION: &'static str = "M";
    // const AEAT_720_ASSET_DISPOSAL: &'static str = "C";
    const AEAT_720_ASSET_REPRESENTATON: &'static str = "A";

    // Field definitions
    const REGISTER_TYPE_FIELD: Aeat720Field = Aeat720Field::Numeric(1, 1);
    const DOCUMENT_ID_FIELD: Aeat720Field = Aeat720Field::Numeric(2, 4);
    const YEAR_FIELD: Aeat720Field = Aeat720Field::Numeric(5, 8);
    const NIF_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(9, 17);
    const DECLARED_NIF_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(18, 26);
    const PROXY_NIF_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(27, 35);
    const NAME_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(36, 75);
    const DECLARATION_TYPE_FIELD: Aeat720Field = Aeat720Field::Numeric(76, 76);
    const OWNERSHIP_TYPE_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(77, 101);
    const ASSET_TYPE_FIELD: Aeat720Field = Aeat720Field::String(102, 102);
    const ASSET_SUBTYPE_FIELD: Aeat720Field = Aeat720Field::Numeric(103, 103);
    const REAL_STATE_ASSET_TYPE_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(104, 128);
    const COUNTRY_CODE_FIELD: Aeat720Field = Aeat720Field::String(129, 130);
    const STOCK_ID_TYPE_FIELD: Aeat720Field = Aeat720Field::Numeric(131, 131);
    const STOCK_ID_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(132, 143);
    const ACCOUNT_ID_TYPE_FIELD: Aeat720Field = Aeat720Field::String(144, 144);
    const ACCOUNT_ID_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(145, 155);
    const ACCOUNT_CODE_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(156, 189);
    const ENTITY_NAME_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(190, 230);
    const ENTITY_NIF_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(231, 250);
    const ENTITY_ADDRESS_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(251, 412);
    const ENTITY_COUNTRY_CODE_FIELD: Aeat720Field = Aeat720Field::AlphaNumeric(413, 414);
    const FIRST_ACQUISITION_DATE_FIELD: Aeat720Field = Aeat720Field::Numeric(415, 422);
    const ACQUISITION_TYPE_FIELD: Aeat720Field = Aeat720Field::String(423, 423);
    const EXTINCTION_DATE_FIELD: Aeat720Field = Aeat720Field::Numeric(424, 431);
    const ACQUISITON_SIGN_FIELD: Aeat720Field = Aeat720Field::String(432, 432); // 'N' if negative
    const ACQUISITION_INT_FIELD: Aeat720Field = Aeat720Field::Numeric(433, 444);
    const ACQUISITION_FRACTION_FIELD: Aeat720Field = Aeat720Field::Numeric(445, 446);
    const VALUATION_SIGN_FIELD: Aeat720Field = Aeat720Field::String(447, 447); // 'N' if negative
    const VALUATION_INT_FIELD: Aeat720Field = Aeat720Field::Numeric(448, 459);
    const VALUATION_FRACTION_FIELD: Aeat720Field = Aeat720Field::Numeric(460, 461);
    const STOCK_REPRESENTATION_FIELD: Aeat720Field = Aeat720Field::String(462, 462); // 'A' usually
    const STOCK_QUANTITY_INT_FIELD: Aeat720Field = Aeat720Field::Numeric(463, 472);
    const STOCK_QUANTITY_FRACTION_FIELD: Aeat720Field = Aeat720Field::Numeric(473, 474);
    const REAL_STATE_REPRESENTATION_FIELD: Aeat720Field = Aeat720Field::String(475, 475);
    const OWNED_PERCENTAGE_INT_FIELD: Aeat720Field = Aeat720Field::Numeric(476, 478);
    const OWNED_PERCENTAGE_FRACTION_FIELD: Aeat720Field = Aeat720Field::Numeric(479, 480);
    const REMAINDER_BLANK_FIELD: Aeat720Field = Aeat720Field::String(481, 500);
}

impl Default for DetailRegister {
    #[allow(unused_must_use)]
    fn default() -> Self {
        let mut fields: AeatRegisterArray = [b' '; AEAT_720_REGISTER_SIZE_BYTES];

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::REGISTER_TYPE_FIELD,
            Self::AEAT_720_DETAIL_REGISTER_TYPE,
        );

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::DOCUMENT_ID_FIELD,
            AEAT_720_DOCUMENT_ID,
        );

        Aeat720Field::write_numeric_field(&mut fields, Self::YEAR_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::NIF_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::DECLARED_NIF_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::PROXY_NIF_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::NAME_FIELD, "");

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::DECLARATION_TYPE_FIELD,
            Self::AEAT_720_OWNER_TYPE,
        );

        Aeat720Field::write_field(&mut fields, Self::OWNERSHIP_TYPE_FIELD, "");

        Aeat720Field::write_field(
            &mut fields,
            Self::ASSET_TYPE_FIELD,
            Self::AEAT_720_ASSET_TYPE,
        );

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::ASSET_SUBTYPE_FIELD,
            Self::AEAT_720_OWNER_TYPE,
        );

        Aeat720Field::write_field(&mut fields, Self::REAL_STATE_ASSET_TYPE_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::COUNTRY_CODE_FIELD, "");

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::STOCK_ID_TYPE_FIELD,
            Self::AEAT_720_STOCK_ID_TYPE,
        );

        Aeat720Field::write_field(&mut fields, Self::STOCK_ID_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ACCOUNT_ID_TYPE_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ACCOUNT_ID_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ACCOUNT_CODE_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ENTITY_NAME_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ENTITY_NIF_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ENTITY_ADDRESS_FIELD, "");

        Aeat720Field::write_field(&mut fields, Self::ENTITY_COUNTRY_CODE_FIELD, "");

        Aeat720Field::write_numeric_field(&mut fields, Self::FIRST_ACQUISITION_DATE_FIELD, 0);

        Aeat720Field::write_field(
            &mut fields,
            Self::ACQUISITION_TYPE_FIELD,
            Self::AEAT_720_ASSET_FIRST_ACQUISITION,
        );

        Aeat720Field::write_numeric_field(&mut fields, Self::EXTINCTION_DATE_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::ACQUISITON_SIGN_FIELD, "");

        Aeat720Field::write_numeric_field(&mut fields, Self::ACQUISITION_INT_FIELD, 0);

        Aeat720Field::write_numeric_field(&mut fields, Self::ACQUISITION_FRACTION_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::VALUATION_SIGN_FIELD, "");

        Aeat720Field::write_numeric_field(&mut fields, Self::VALUATION_INT_FIELD, 0);

        Aeat720Field::write_numeric_field(&mut fields, Self::VALUATION_FRACTION_FIELD, 0);

        Aeat720Field::write_field(
            &mut fields,
            Self::STOCK_REPRESENTATION_FIELD,
            Self::AEAT_720_ASSET_REPRESENTATON,
        );

        Aeat720Field::write_numeric_field(&mut fields, Self::STOCK_QUANTITY_INT_FIELD, 0);

        Aeat720Field::write_numeric_field(&mut fields, Self::STOCK_QUANTITY_FRACTION_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::REAL_STATE_REPRESENTATION_FIELD, "");

        Aeat720Field::write_numeric_field(&mut fields, Self::OWNED_PERCENTAGE_INT_FIELD, 0);

        Aeat720Field::write_numeric_field(&mut fields, Self::OWNED_PERCENTAGE_FRACTION_FIELD, 0);

        Aeat720Field::write_field(&mut fields, Self::REMAINDER_BLANK_FIELD, "");

        Self { fields }
    }
}

impl DetailRegister {
    fn new(
        note: &BalanceNote,
        transactions: &[AccountNote],
        year: usize,
        nif: &str,
        name: &str,
    ) -> Result<Self> {
        let mut fields = Self::default().fields;

        Aeat720Field::write_numeric_field(&mut fields, Self::YEAR_FIELD, year)?;
        Aeat720Field::write_field(&mut fields, Self::NIF_FIELD, nif)?;
        Aeat720Field::write_field(&mut fields, Self::DECLARED_NIF_FIELD, nif)?;
        Aeat720Field::write_field(&mut fields, Self::NAME_FIELD, name)?;
        Aeat720Field::write_field(
            &mut fields,
            Self::COUNTRY_CODE_FIELD,
            &note.broker.country_code,
        )?;
        Aeat720Field::write_field(&mut fields, Self::STOCK_ID_FIELD, &note.company.isin)?;
        Aeat720Field::write_field(
            &mut fields,
            Self::ENTITY_NAME_FIELD,
            &note.company.name.to_uppercase(),
        )?;
        Aeat720Field::write_field(
            &mut fields,
            Self::ENTITY_COUNTRY_CODE_FIELD,
            &note.company.isin[0..2],
        )?;
        let first_tx_date = {
            let company = transactions.iter().find(|&x| x.company == note.company);
            match company {
                Some(c) => c.date.format("%Y%m%d").to_string(),
                None => NaiveDate::from_ymd_opt(year as i32, 1, 1)
                    .unwrap()
                    .format("%Y%m%d")
                    .to_string(),
            }
            .parse::<usize>()
            .unwrap_or(0)
        };
        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::FIRST_ACQUISITION_DATE_FIELD,
            first_tx_date,
        )?;

        if note.value_in_euro.is_sign_negative() {
            Aeat720Field::write_field(
                &mut fields,
                Self::ACQUISITON_SIGN_FIELD,
                AEAT_720_NEGATIVE_SIGN,
            )?;
        }
        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::ACQUISITION_INT_FIELD,
            note.value_in_euro.trunc().abs().to_usize().unwrap_or(0),
        )?;
        let mut remainder = note.value_in_euro.fract();
        remainder.set_scale(0)?;
        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::ACQUISITION_FRACTION_FIELD,
            remainder.to_usize().unwrap_or(0),
        )?;

        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::STOCK_QUANTITY_INT_FIELD,
            note.quantity.trunc().abs().to_usize().unwrap_or(0),
        )?;

        let mut remainder = note.quantity.fract();
        remainder.set_scale(2)?;
        Aeat720Field::write_numeric_field(
            &mut fields,
            Self::STOCK_QUANTITY_FRACTION_FIELD,
            remainder.trunc().to_usize().unwrap_or(0),
        )?;

        Aeat720Field::write_numeric_field(&mut fields, Self::OWNED_PERCENTAGE_INT_FIELD, 100)?;
        Aeat720Field::write_numeric_field(&mut fields, Self::OWNED_PERCENTAGE_FRACTION_FIELD, 0)?;

        Ok(Self { fields })
    }
}
pub struct Aeat720Report {
    summary: SummaryRegister,
    details: Vec<DetailRegister>,
}

impl Aeat720Report {
    pub fn new(info: &FinancialInformation) -> Result<Aeat720Report> {
        let mut details = Vec::new();
        let full_name = info.full_name();

        for balance_note in &info.balance_notes {
            let detail = DetailRegister::new(
                balance_note,
                &info.account_notes,
                info.year,
                &info.nif,
                &full_name,
            )?;
            details.push(detail);
        }

        Ok(Aeat720Report {
            summary: SummaryRegister::new(
                &info.balance_notes,
                info.year,
                &info.nif,
                &full_name,
                &info.phone,
            )?,
            details,
        })
    }

    pub fn generate(self) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        result.reserve(
            AEAT_720_REGISTER_SIZE_BYTES * (self.details.len() + 1) + (self.details.len() + 1),
        );

        result.write_all(&self.summary.fields)?;
        result.write_all(b"\n")?;
        for detail in self.details {
            result.write_all(&detail.fields)?;
            result.write_all(b"\n")?;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_numeric_field() {
        let mut fields: AeatRegisterArray = [b' '; AEAT_720_REGISTER_SIZE_BYTES];

        assert!(Aeat720Field::write_numeric_field(
            &mut fields,
            DetailRegister::DOCUMENT_ID_FIELD,
            AEAT_720_DOCUMENT_ID,
        )
        .is_ok());
        assert_eq!(fields[1..4], [b'7', b'2', b'0'],);

        assert!(Aeat720Field::write_numeric_field(
            &mut fields,
            DetailRegister::REGISTER_TYPE_FIELD,
            DetailRegister::AEAT_720_DETAIL_REGISTER_TYPE,
        )
        .is_ok());
        assert_eq!(fields[0], b'2');

        assert!(
            Aeat720Field::write_numeric_field(&mut fields, DetailRegister::YEAR_FIELD, 2020)
                .is_ok()
        );
        assert_eq!(fields[4..8], [b'2', b'0', b'2', b'0']);

        assert!(
            Aeat720Field::write_numeric_field(&mut fields, DetailRegister::YEAR_FIELD, 2).is_ok()
        );
        assert_eq!(fields[4..8], [b'0', b'0', b'0', b'2']);
    }

    #[test]
    fn test_write_alphanumeric_field() {
        let mut fields: AeatRegisterArray = [b' '; AEAT_720_REGISTER_SIZE_BYTES];

        assert!(
            Aeat720Field::write_field(&mut fields, DetailRegister::NIF_FIELD, "20202020A").is_ok()
        );
        assert_eq!(
            fields[8..17],
            [b'2', b'0', b'2', b'0', b'2', b'0', b'2', b'0', b'A']
        );

        assert!(Aeat720Field::write_field(
            &mut fields,
            DetailRegister::DECLARED_NIF_FIELD,
            "20202020"
        )
        .is_ok());
        assert_eq!(
            fields[17..26],
            [b'2', b'0', b'2', b'0', b'2', b'0', b'2', b'0', b' ']
        );

        assert!(
            Aeat720Field::write_field(&mut fields, DetailRegister::PROXY_NIF_FIELD, "").is_ok()
        );
        assert_eq!(
            fields[26..35],
            [b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ']
        );
    }

    #[test]
    fn test_write_string_field() {
        let mut fields: AeatRegisterArray = [b' '; AEAT_720_REGISTER_SIZE_BYTES];

        assert!(Aeat720Field::write_field(
            &mut fields,
            DetailRegister::STOCK_REPRESENTATION_FIELD,
            DetailRegister::AEAT_720_ASSET_REPRESENTATON,
        )
        .is_ok());

        assert_eq!(fields[461..462], [b'A']);

        assert!(Aeat720Field::write_field(
            &mut fields,
            DetailRegister::REAL_STATE_REPRESENTATION_FIELD,
            "",
        )
        .is_ok());

        assert_eq!(fields[474..475], [b' ']);
    }

    #[test]
    fn test_summary_detail_register() {
        const DEFAULT_FIELDS: AeatRegisterArray = [
            b'1', // register type
            b'7', b'2', b'0', // document id
            b'0', b'0', b'0', b'0', // year
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', // nif
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // name field
            b'T', // transmission field
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', // telephone
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // contact name field
            b'7', b'2', b'0', // second document id
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'1', // id
            b' ', //  complementary field
            b' ', // replacement field
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0',
            b'0', // previuos declaration id
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', // tota registers field
            b' ', // acquisition sign
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0',
            b'0', // acquisition int field
            b'0', b'0', // acquisition fractional part field
            b' ', // valuation sign
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0',
            b'0', // valuation int field
            b'0', b'0', // valuation fractional part field
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // remainder blank
        ];

        assert_eq!(SummaryRegister::default().fields, DEFAULT_FIELDS);
    }

    #[test]
    fn test_default_detail_register() {
        const DEFAULT_FIELDS: AeatRegisterArray = [
            b'2', // register type
            b'7', b'2', b'0', // document id
            b'0', b'0', b'0', b'0', // year
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', // nif
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', // declared nif
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', // proxy  nif
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // name field
            b'1', // declaration type
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // ownership type
            b'V', // asset type
            b'1', //  asset subtype
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // real state asset type
            b' ', b' ', // country code
            b'1', // stock id type
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // stock id (ISIN)
            b' ', // account id type
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', // account id
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', // account code
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', // entity name
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', // entity nif
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', // company address
            b' ', b' ', // company country code
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', // first acquistion date
            b'A', // acquistion type
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', // extinction date
            b' ', // acquisition sign
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0',
            b'0', // acquistion int
            b'0', b'0', // acquisition fraction
            b' ', // valuation sign
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0',
            b'0', // valuation int
            b'0', b'0', // valuation fraction
            b'A', // stock represenation
            b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', b'0', // stock quantity int
            b'0', b'0', // stock quantity fraction
            b' ', // real state representation field
            b'0', b'0', b'0', // owned percentage int
            b'0', b'0', // owned percentage fraction
            b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
            b' ', b' ', b' ', b' ', b' ', b' ', // remainder blank
        ];

        assert_eq!(DetailRegister::default().fields, DEFAULT_FIELDS);
    }

    #[test]
    fn test_iso_8859_15_encoding() {
        assert_eq!(ISO_8859_15.encode("Ñ").0.to_vec(), vec![209]);
    }
}
