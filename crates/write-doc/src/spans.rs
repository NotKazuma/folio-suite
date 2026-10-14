//! Inline formatting: `**bold**` and `*italic*` markers inside a block's text.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    /// The `*` / `**` characters themselves: drawn dimmed, dropped from plain text.
    pub marker: bool,
}

/// Splits `text` into styled spans. Unmatched markers are ordinary text. The pieces always
/// concatenate back to exactly `text`, so the editor can map cursor positions one to one.
pub fn spans(text: &str) -> Vec<Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<Span> = Vec::new();
    let (mut bold, mut italic) = (false, false);
    let mut buf = String::new();
    let flush = |buf: &mut String, out: &mut Vec<Span>, bold: bool, italic: bool| {
        if !buf.is_empty() {
            out.push(Span { text: std::mem::take(buf), bold, italic, marker: false });
        }
    };
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '*' {
            let double = chars.get(i + 1) == Some(&'*');
            let len = if double { 2 } else { 1 };
            // A marker opens only if it has a matching closer later in the text.
            let closes = if double { bold } else { italic };
            let rest: String = chars[i + len..].iter().collect();
            let has_closer = rest.contains(if double { "**" } else { "*" });
            if closes || has_closer {
                flush(&mut buf, &mut out, bold, italic);
                let marker = "*".repeat(len);
                if double {
                    bold = !bold;
                } else {
                    italic = !italic;
                }
                out.push(Span { text: marker, bold, italic, marker: true });
                i += len;
                continue;
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush(&mut buf, &mut out, bold, italic);
    out
}

/// `text` without its formatting markers.
pub fn plain_text(text: &str) -> String {
    spans(text).into_iter().filter(|s| !s.marker).map(|s| s.text).collect()
}

/// Wraps the character range `[from, to)` of `text` in `marker` (`"**"` or `"*"`), or removes the
/// marker when the range is already wrapped. Returns the new text and the new selection.
pub fn toggle_marker(text: &str, from: usize, to: usize, marker: &str) -> (String, usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let (from, to) = (from.min(to).min(chars.len()), from.max(to).min(chars.len()));
    let m: Vec<char> = marker.chars().collect();
    let n = m.len();
    let wrapped = from >= n && chars[from - n..from] == m[..] && chars.get(to..to + n).is_some_and(|s| s == &m[..]);
    let mut out: Vec<char> = Vec::with_capacity(chars.len() + 2 * n);
    if wrapped {
        out.extend(&chars[..from - n]);
        out.extend(&chars[from..to]);
        out.extend(&chars[to + n..]);
        (out.into_iter().collect(), from - n, to - n)
    } else {
        out.extend(&chars[..from]);
        out.extend(&m);
        out.extend(&chars[from..to]);
        out.extend(&m);
        out.extend(&chars[to..]);
        (out.into_iter().collect(), from + n, to + n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rebuilt(text: &str) -> String {
        spans(text).into_iter().map(|s| s.text).collect()
    }

    #[test]
    fn styles_and_round_trips() {
        let s = spans("a **b** *c*");
        let styled: Vec<_> = s.iter().filter(|x| !x.marker).map(|x| (x.text.as_str(), x.bold, x.italic)).collect();
        assert_eq!(styled, vec![("a ", false, false), ("b", true, false), (" ", false, false), ("c", false, true)]);
        assert_eq!(rebuilt("a **b** *c*"), "a **b** *c*");
    }

    #[test]
    fn unmatched_markers_stay_text() {
        assert_eq!(plain_text("2 * 3 = 6"), "2 * 3 = 6");
        assert_eq!(plain_text("**open"), "**open");
        assert_eq!(rebuilt("**open and *half"), "**open and *half");
    }

    #[test]
    fn nested_and_empty() {
        assert_eq!(plain_text("**bold *and italic***"), "bold and italic");
        assert_eq!(plain_text(""), "");
        assert_eq!(rebuilt("***"), "***");
    }

    #[test]
    fn toggling_wraps_and_unwraps() {
        let (t, a, b) = toggle_marker("make this bold", 5, 9, "**");
        assert_eq!(t, "make **this** bold");
        assert_eq!((a, b), (7, 11));
        let (back, a2, b2) = toggle_marker(&t, a, b, "**");
        assert_eq!(back, "make this bold");
        assert_eq!((a2, b2), (5, 9));
        // a reversed or out-of-range selection is clamped, not a panic
        let (t3, _, _) = toggle_marker("abc", 99, 1, "*");
        assert_eq!(t3, "a*bc*");
    }
}
