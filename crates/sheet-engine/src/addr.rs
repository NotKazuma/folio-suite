//! Cell addresses: `(col, row)` zero-based, written `A1`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Addr {
    pub col: u32,
    pub row: u32,
}

impl Addr {
    pub fn new(col: u32, row: u32) -> Addr {
        Addr { col, row }
    }

    /// `A1`, `B12`, `AA3`.
    pub fn label(self) -> String {
        format!("{}{}", col_name(self.col), self.row + 1)
    }
}

/// 0 -> "A", 25 -> "Z", 26 -> "AA".
pub fn col_name(col: u32) -> String {
    let mut n = u64::from(col) + 1;
    let mut out = Vec::new();
    while n > 0 {
        let rem = ((n - 1) % 26) as u8;
        out.push(b'A' + rem);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// Parses `A1` / `$B$7` (case-insensitive). `None` for anything else, including row 0.
pub fn parse_addr(text: &str) -> Option<Addr> {
    let t: String = text.chars().filter(|c| *c != '$').collect();
    let letters = t.chars().take_while(char::is_ascii_alphabetic).count();
    if letters == 0 || letters > 3 || letters == t.len() {
        return None;
    }
    let (l, d) = t.split_at(letters);
    if !d.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let col = l.to_ascii_uppercase().bytes().fold(0u32, |n, b| n * 26 + u32::from(b - b'A' + 1)) - 1;
    let row: u32 = d.parse().ok()?;
    if row == 0 || row > 1_048_576 {
        return None;
    }
    Some(Addr { col, row: row - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for (c, n) in [(0, "A"), (25, "Z"), (26, "AA"), (27, "AB"), (701, "ZZ"), (702, "AAA")] {
            assert_eq!(col_name(c), n);
            assert_eq!(parse_addr(&format!("{n}1")).unwrap().col, c);
        }
    }

    #[test]
    fn parses_and_rejects() {
        assert_eq!(parse_addr("b12"), Some(Addr::new(1, 11)));
        assert_eq!(parse_addr("$C$3"), Some(Addr::new(2, 2)));
        for bad in ["", "A", "1", "A0", "1A", "A1B", "ABCD1", "A-1", "A99999999999"] {
            assert_eq!(parse_addr(bad), None, "{bad}");
        }
        assert_eq!(Addr::new(27, 4).label(), "AB5");
    }
}
