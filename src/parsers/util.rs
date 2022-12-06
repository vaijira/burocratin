use crate::data::BalanceNotes;
use anyhow::Result;
use rust_decimal::Decimal;

pub(crate) fn recalculate_balance_notes(
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

pub(crate) fn replace_escaped_fields(original_str: &str) -> String {
    let mut fields_str = String::new();
    let mut in_quoted_field = false;
    for char in original_str.chars() {
        if in_quoted_field {
            if char == '"' {
                in_quoted_field = false;
            } else if char != ',' {
                fields_str.push(char);
            }
        } else if char == '"' {
            in_quoted_field = true;
        } else {
            fields_str.push(char);
        }
    }

    fields_str
}
