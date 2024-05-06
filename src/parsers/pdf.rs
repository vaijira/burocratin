use pdf_extract::OutputError;

use super::degiro::{DEGIRO_BALANCE_NOTES_HEADER, DEGIRO_NOTES_HEADER_BEGIN};

fn remove_repeated_section(mut input: String, section: &str) -> String {
    if let Some(first_pos) = input.find(section) {
        let mut pos = input.rfind(section).unwrap();

        while pos != first_pos {
            input.replace_range(pos..pos + section.len(), "");
            pos = input.rfind(section).unwrap();
        }
    }

    input
}
pub fn read_pdf(data: &[u8]) -> Result<String, OutputError> {
    let out = pdf_extract::extract_text_from_mem(data)?;
    let out = remove_repeated_section(out, DEGIRO_NOTES_HEADER_BEGIN);
    let out = remove_repeated_section(out, DEGIRO_BALANCE_NOTES_HEADER);
    Ok(out)
}

mod tests {
    #[test]
    #[ignore]
    fn read_pdf_test() {
        let bytes = std::fs::read("tests/data/degiro_2019.pdf").unwrap();
        let out = super::read_pdf(&bytes).unwrap();
        println!("-------------------------------------------------------------");
        print!("{}", out);
        println!("-------------------------------------------------------------");
    }
}
