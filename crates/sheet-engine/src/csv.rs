//! CSV in and out (RFC 4180 style: quotes, doubled quotes, embedded commas and newlines).

use std::collections::HashMap;

use crate::addr::Addr;
use crate::eval::Sheet;

/// Largest import we accept, in cells (a hostile file cannot make us allocate without bound).
const MAX_CELLS: usize = 2_000_000;

pub fn from_csv(text: &str) -> HashMap<Addr, String> {
    let mut cells = HashMap::new();
    let (mut row, mut col) = (0u32, 0u32);
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let end_field = |cells: &mut HashMap<Addr, String>, field: &mut String, col: u32, row: u32| {
        if !field.is_empty() && cells.len() < MAX_CELLS {
            cells.insert(Addr::new(col, row), std::mem::take(field));
        }
        field.clear();
    };
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                other => field.push(other),
            }
        } else {
            match c {
                '"' => quoted = true,
                ',' => {
                    end_field(&mut cells, &mut field, col, row);
                    col = col.saturating_add(1);
                }
                '\r' | '\n' => {
                    if c == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    end_field(&mut cells, &mut field, col, row);
                    row = row.saturating_add(1);
                    col = 0;
                }
                other => field.push(other),
            }
        }
    }
    end_field(&mut cells, &mut field, col, row);
    cells
}

/// Writes calculated values (not formulas) so the file opens anywhere.
pub fn to_csv(sheet: &Sheet) -> String {
    let (cols, rows) = sheet.extent();
    let mut out = String::new();
    for r in 0..rows {
        let line: Vec<String> = (0..cols)
            .map(|c| {
                let v = sheet.value(Addr::new(c, r)).to_string();
                if v.contains([',', '"', '\n', '\r']) { format!("\"{}\"", v.replace('"', "\"\"")) } else { v }
            })
            .collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::parse_addr;

    #[test]
    fn quotes_and_commas_round_trip() {
        let cells = from_csv("name,note\n\"Smith, J\",\"say \"\"hi\"\"\"\n");
        assert_eq!(cells[&parse_addr("A2").unwrap()], "Smith, J");
        assert_eq!(cells[&parse_addr("B2").unwrap()], "say \"hi\"");
        assert_eq!(to_csv(&Sheet::from_cells(cells)), "name,note\n\"Smith, J\",\"say \"\"hi\"\"\"\n");
    }

    #[test]
    fn handles_crlf_bom_and_ragged_rows() {
        let cells = from_csv("\u{feff}a,b\r\nc\r\n\r\nd,,f");
        assert_eq!(cells[&parse_addr("A1").unwrap()], "a");
        assert_eq!(cells[&parse_addr("A2").unwrap()], "c");
        assert_eq!(cells[&parse_addr("C4").unwrap()], "f");
        assert!(!cells.contains_key(&parse_addr("B4").unwrap()));
    }

    #[test]
    fn exports_values_not_formulas() {
        let mut s = Sheet::new();
        s.set(Addr::new(0, 0), "2");
        s.set(Addr::new(1, 0), "=A1*5");
        assert_eq!(to_csv(&s), "2,10\n");
    }

    #[test]
    fn unterminated_quote_does_not_panic() {
        let cells = from_csv("a,\"never closed\nstill going");
        assert!(cells.contains_key(&parse_addr("A1").unwrap()));
    }
}
