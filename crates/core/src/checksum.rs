//! Reading `SHA256SUMS.txt` files (`<64 hex digits>  <file name>` per line).

use std::collections::HashMap;

/// Maps file name to lowercase hex digest. Malformed lines are skipped.
pub fn parse_sums(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim_end();
        let Some((digest, rest)) = line.split_once(char::is_whitespace) else { continue };
        let name = rest.trim_start().trim_start_matches('*');
        if digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()) && !name.is_empty() {
            out.insert(name.to_string(), digest.to_ascii_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_good_lines_and_skips_junk() {
        let h = "a".repeat(64);
        let text = format!("{h}  one.msi\nnot a hash line\n{} *two.dmg\n{h}\n", h.to_uppercase());
        let m = parse_sums(&text);
        assert_eq!(m.get("one.msi"), Some(&h));
        assert_eq!(m.get("two.dmg"), Some(&h));
        assert_eq!(m.len(), 2);
    }
}
