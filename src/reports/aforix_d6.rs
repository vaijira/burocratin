use crate::data::{BalanceNote, FinancialInformation, SPAIN_COUNTRY_CODE};

use rust_decimal::Decimal;
use std::io::Write;
use std::str;
use xml::writer::{EmitterConfig, EventWriter, Result, XmlEvent};

const AFORIX_D6_FORM_TYPE: &str = "D-6";

const RECORDS_FIRST_PAGE: usize = 3;
const RECORDS_PER_PAGE: usize = 6;

const GBX_CURRENCY: &str = "GBX";
const GBP_CURRENCY: &str = "GBP";

struct D6Context {
    page_id: u32,
    field_id: u32,
    notes_index: usize,
}

impl D6Context {
    fn new() -> D6Context {
        D6Context {
            page_id: 1,
            field_id: 0x2DB,
            notes_index: 0,
        }
    }
}

fn write_d6_header<W: Write>(writer: &mut EventWriter<W>) -> Result<()> {
    writer.write(XmlEvent::start_element("Formulario"))?;

    writer.write(XmlEvent::start_element("Tipo"))?;
    writer.write(XmlEvent::characters(AFORIX_D6_FORM_TYPE))?;
    writer.write(XmlEvent::end_element())?; // Tipo

    writer.write(XmlEvent::start_element("Version"))?;
    writer.write(XmlEvent::characters("R10"))?;
    writer.write(XmlEvent::end_element())?; // Version

    Ok(())
}

fn write_d6_footer<W: Write>(writer: &mut EventWriter<W>) -> Result<()> {
    writer.write(XmlEvent::end_element())?; // Formulario

    Ok(())
}

fn write_field<W: Write>(
    writer: &mut EventWriter<W>,
    context: &mut D6Context,
    data: &str,
) -> Result<()> {
    writer.write(XmlEvent::start_element("Campo"))?;

    writer.write(XmlEvent::start_element("Codigo"))?;
    writer.write(XmlEvent::characters(&format!("{:X}", context.field_id)))?;
    writer.write(XmlEvent::end_element())?; // Codigo
    context.field_id += 1;

    writer.write(XmlEvent::start_element("Datos"))?;
    writer.write(XmlEvent::characters(data))?;
    writer.write(XmlEvent::end_element())?; // Datos

    writer.write(XmlEvent::end_element())?; // Campo

    Ok(())
}

fn write_page_header<W: Write>(
    writer: &mut EventWriter<W>,
    context: &mut D6Context,
    info: &FinancialInformation,
) -> Result<()> {
    writer.write(XmlEvent::start_element("Pagina"))?;

    writer.write(XmlEvent::start_element("Tipo"))?;
    if context.page_id == 1 {
        writer.write(XmlEvent::characters("D61"))?;
    } else {
        writer.write(XmlEvent::characters("D62"))?;
    }
    writer.write(XmlEvent::end_element())?; // Tipo

    writer.write(XmlEvent::start_element("Campos"))?;

    write_field(writer, context, "D")?;

    write_field(writer, context, &info.year.to_string())?;

    if context.page_id == 1 {
        context.field_id += 2;
        write_field(writer, context, &info.full_name())?;
        write_field(writer, context, &info.nif)?;
    } else {
        write_field(writer, context, &info.full_name())?;
        write_field(writer, context, &info.nif)?;
        context.field_id += 2;
    }

    Ok(())
}

fn write_page_footer<W: Write>(writer: &mut EventWriter<W>, context: &mut D6Context) -> Result<()> {
    writer.write(XmlEvent::end_element())?; // Campos
    writer.write(XmlEvent::end_element())?; // Pagina
    context.page_id += 1;

    Ok(())
}

pub fn format_valuation(valuation: &Decimal) -> String {
    valuation.to_string().replace('.', ",")
}

fn write_company_note<W: Write>(
    writer: &mut EventWriter<W>,
    context: &mut D6Context,
    note: &BalanceNote,
) -> Result<()> {
    write_field(writer, context, "N")?;
    write_field(writer, context, &note.company.isin)?;
    write_field(writer, context, &note.company.name)?;
    if &note.company.name[0..2] == SPAIN_COUNTRY_CODE {
        write_field(writer, context, "800")?;
    } else {
        write_field(writer, context, "400")?;
    }
    write_field(writer, context, "01")?;
    write_field(writer, context, &note.broker.country_code)?;
    if note.currency == GBX_CURRENCY {
        write_field(writer, context, GBP_CURRENCY)?;
    } else {
        write_field(writer, context, &note.currency)?;
    }
    write_field(writer, context, &format_valuation(&note.quantity))?;
    context.field_id += 1; // for empty field
    let price = if note.currency == GBX_CURRENCY {
        ((note.quantity * note.price) / Decimal::new(100, 0)).round_dp(2)
    } else {
        (note.quantity * note.price).round_dp(2)
    };
    write_field(writer, context, &format_valuation(&price))?;
    context.field_id += 2;
    context.notes_index += 1;

    Ok(())
}

fn write_first_page<W: Write>(
    writer: &mut EventWriter<W>,
    context: &mut D6Context,
    info: &FinancialInformation,
) -> Result<()> {
    write_page_header(writer, context, info)?;
    context.field_id += 7;

    while context.notes_index < info.balance_notes.len() && context.notes_index < RECORDS_FIRST_PAGE
    {
        write_company_note(
            writer,
            context,
            info.balance_notes.get(context.notes_index).unwrap(),
        )?;
    }

    write_page_footer(writer, context)?;

    Ok(())
}

fn write_page<W: Write>(
    writer: &mut EventWriter<W>,
    context: &mut D6Context,
    info: &FinancialInformation,
) -> Result<()> {
    context.field_id = 0x320;

    write_page_header(writer, context, info)?;

    let initial_index = context.notes_index;

    while context.notes_index < info.balance_notes.len()
        && context.notes_index < initial_index + RECORDS_PER_PAGE
    {
        write_company_note(
            writer,
            context,
            info.balance_notes.get(context.notes_index).unwrap(),
        )?;
    }

    write_page_footer(writer, context)?;

    Ok(())
}

pub fn create_d6_form(info: &FinancialInformation) -> Result<Vec<u8>> {
    let mut target: Vec<u8> = Vec::new();
    let mut context = D6Context::new();

    let mut writer = EmitterConfig::new()
        .line_separator("\r\n")
        .perform_indent(true)
        .normalize_empty_elements(false)
        .write_document_declaration(true)
        .create_writer(&mut target);

    write_d6_header(&mut writer)?;

    while context.notes_index < info.balance_notes.len() {
        if context.notes_index == 0 {
            write_first_page(&mut writer, &mut context, info)?;
        } else {
            write_page(&mut writer, &mut context, info)?;
        }
    }
    write_d6_footer(&mut writer)?;

    Ok(target)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::data::{BalanceNotes, BrokerInformation, CompanyInfo};
    use std::sync::Arc;

    fn compare_strs_by_line(file1: &str, file2: &str) {
        let mut line_number = 1;
        let mut iter1 = file1.lines();
        let mut iter2 = file2.lines();
        while let (Some(line1), Some(line2)) = (iter1.next(), iter2.next()) {
            assert_eq!(
                line1, line2,
                "comparing lines in files, line number: {}",
                line_number
            );
            line_number += 1;
        }
    }

    #[test]
    fn create_d6_form_test() {
        let degiro_broker: Arc<BrokerInformation> = Arc::new(BrokerInformation::new(
            String::from("Degiro"),
            String::from("NL"),
        ));
        let balance_notes: BalanceNotes = vec![
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
        let mut info = FinancialInformation::new();
        info.balance_notes = balance_notes;
        info.name = String::from("NILES");
        info.surname = String::from("SMITH DONCIC");
        info.year = 2019;
        info.nif = String::from("123456789A");

        let d6_form = create_d6_form(&info).unwrap();
        compare_strs_by_line(
            &D6_FORM_XML_RESULT.replace("\n", "\r\n"),
            str::from_utf8(&d6_form[..]).unwrap(),
        );
    }

    const D6_FORM_XML_RESULT: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Formulario>
  <Tipo>D-6</Tipo>
  <Version>R10</Version>
  <Pagina>
    <Tipo>D61</Tipo>
    <Campos>
      <Campo>
        <Codigo>2DB</Codigo>
        <Datos>D</Datos>
      </Campo>
      <Campo>
        <Codigo>2DC</Codigo>
        <Datos>2019</Datos>
      </Campo>
      <Campo>
        <Codigo>2DF</Codigo>
        <Datos>SMITH DONCIC NILES</Datos>
      </Campo>
      <Campo>
        <Codigo>2E0</Codigo>
        <Datos>123456789A</Datos>
      </Campo>
      <Campo>
        <Codigo>2E8</Codigo>
        <Datos>N</Datos>
      </Campo>
      <Campo>
        <Codigo>2E9</Codigo>
        <Datos>GG00B4L84979</Datos>
      </Campo>
      <Campo>
        <Codigo>2EA</Codigo>
        <Datos>BURFORD CAP LD</Datos>
      </Campo>
      <Campo>
        <Codigo>2EB</Codigo>
        <Datos>400</Datos>
      </Campo>
      <Campo>
        <Codigo>2EC</Codigo>
        <Datos>01</Datos>
      </Campo>
      <Campo>
        <Codigo>2ED</Codigo>
        <Datos>NL</Datos>
      </Campo>
      <Campo>
        <Codigo>2EE</Codigo>
        <Datos>GBP</Datos>
      </Campo>
      <Campo>
        <Codigo>2EF</Codigo>
        <Datos>122</Datos>
      </Campo>
      <Campo>
        <Codigo>2F1</Codigo>
        <Datos>2020,32</Datos>
      </Campo>
      <Campo>
        <Codigo>2F4</Codigo>
        <Datos>N</Datos>
      </Campo>
      <Campo>
        <Codigo>2F5</Codigo>
        <Datos>US30303M1027</Datos>
      </Campo>
      <Campo>
        <Codigo>2F6</Codigo>
        <Datos>FACEBOOK INC. - CLASS</Datos>
      </Campo>
      <Campo>
        <Codigo>2F7</Codigo>
        <Datos>400</Datos>
      </Campo>
      <Campo>
        <Codigo>2F8</Codigo>
        <Datos>01</Datos>
      </Campo>
      <Campo>
        <Codigo>2F9</Codigo>
        <Datos>NL</Datos>
      </Campo>
      <Campo>
        <Codigo>2FA</Codigo>
        <Datos>USD</Datos>
      </Campo>
      <Campo>
        <Codigo>2FB</Codigo>
        <Datos>21</Datos>
      </Campo>
      <Campo>
        <Codigo>2FD</Codigo>
        <Datos>2752,89</Datos>
      </Campo>
      <Campo>
        <Codigo>300</Codigo>
        <Datos>N</Datos>
      </Campo>
      <Campo>
        <Codigo>301</Codigo>
        <Datos>US47215P1066</Datos>
      </Campo>
      <Campo>
        <Codigo>302</Codigo>
        <Datos>JD.COM INC. - AMERICA</Datos>
      </Campo>
      <Campo>
        <Codigo>303</Codigo>
        <Datos>400</Datos>
      </Campo>
      <Campo>
        <Codigo>304</Codigo>
        <Datos>01</Datos>
      </Campo>
      <Campo>
        <Codigo>305</Codigo>
        <Datos>NL</Datos>
      </Campo>
      <Campo>
        <Codigo>306</Codigo>
        <Datos>USD</Datos>
      </Campo>
      <Campo>
        <Codigo>307</Codigo>
        <Datos>140</Datos>
      </Campo>
      <Campo>
        <Codigo>309</Codigo>
        <Datos>2930,20</Datos>
      </Campo>
    </Campos>
  </Pagina>
  <Pagina>
    <Tipo>D62</Tipo>
    <Campos>
      <Campo>
        <Codigo>320</Codigo>
        <Datos>D</Datos>
      </Campo>
      <Campo>
        <Codigo>321</Codigo>
        <Datos>2019</Datos>
      </Campo>
      <Campo>
        <Codigo>322</Codigo>
        <Datos>SMITH DONCIC NILES</Datos>
      </Campo>
      <Campo>
        <Codigo>323</Codigo>
        <Datos>123456789A</Datos>
      </Campo>
      <Campo>
        <Codigo>326</Codigo>
        <Datos>N</Datos>
      </Campo>
      <Campo>
        <Codigo>327</Codigo>
        <Datos>IT0001447785</Datos>
      </Campo>
      <Campo>
        <Codigo>328</Codigo>
        <Datos>MONDO TV</Datos>
      </Campo>
      <Campo>
        <Codigo>329</Codigo>
        <Datos>400</Datos>
      </Campo>
      <Campo>
        <Codigo>32A</Codigo>
        <Datos>01</Datos>
      </Campo>
      <Campo>
        <Codigo>32B</Codigo>
        <Datos>NL</Datos>
      </Campo>
      <Campo>
        <Codigo>32C</Codigo>
        <Datos>EUR</Datos>
      </Campo>
      <Campo>
        <Codigo>32D</Codigo>
        <Datos>1105</Datos>
      </Campo>
      <Campo>
        <Codigo>32F</Codigo>
        <Datos>1319,37</Datos>
      </Campo>
      <Campo>
        <Codigo>332</Codigo>
        <Datos>N</Datos>
      </Campo>
      <Campo>
        <Codigo>333</Codigo>
        <Datos>IL0011320343</Datos>
      </Campo>
      <Campo>
        <Codigo>334</Codigo>
        <Datos>TAPTICA INT LTD</Datos>
      </Campo>
      <Campo>
        <Codigo>335</Codigo>
        <Datos>400</Datos>
      </Campo>
      <Campo>
        <Codigo>336</Codigo>
        <Datos>01</Datos>
      </Campo>
      <Campo>
        <Codigo>337</Codigo>
        <Datos>NL</Datos>
      </Campo>
      <Campo>
        <Codigo>338</Codigo>
        <Datos>GBP</Datos>
      </Campo>
      <Campo>
        <Codigo>339</Codigo>
        <Datos>565</Datos>
      </Campo>
      <Campo>
        <Codigo>33B</Codigo>
        <Datos>904,00</Datos>
      </Campo>
      <Campo>
        <Codigo>33E</Codigo>
        <Datos>N</Datos>
      </Campo>
      <Campo>
        <Codigo>33F</Codigo>
        <Datos>US9837931008</Datos>
      </Campo>
      <Campo>
        <Codigo>340</Codigo>
        <Datos>XPO LOGISTICS INC.</Datos>
      </Campo>
      <Campo>
        <Codigo>341</Codigo>
        <Datos>400</Datos>
      </Campo>
      <Campo>
        <Codigo>342</Codigo>
        <Datos>01</Datos>
      </Campo>
      <Campo>
        <Codigo>343</Codigo>
        <Datos>NL</Datos>
      </Campo>
      <Campo>
        <Codigo>344</Codigo>
        <Datos>USD</Datos>
      </Campo>
      <Campo>
        <Codigo>345</Codigo>
        <Datos>41</Datos>
      </Campo>
      <Campo>
        <Codigo>347</Codigo>
        <Datos>2338,64</Datos>
      </Campo>
    </Campos>
  </Pagina>
</Formulario>"#;
}
