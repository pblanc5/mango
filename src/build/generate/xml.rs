/// Escapes text for XML element content and attribute values: `&`, `<`,
/// `>`, `"` and `'`. Everything else is left unchanged.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-6.5 (batch 4)
    #[test]
    fn escapes_special_characters() {
        assert_eq!(escape("&"), "&amp;");
        assert_eq!(escape("<"), "&lt;");
        assert_eq!(escape(">"), "&gt;");
        assert_eq!(escape("\""), "&quot;");
        assert_eq!(escape("'"), "&apos;");
        assert_eq!(
            escape(r#"<a href="x?a=1&b='2'">"#),
            "&lt;a href=&quot;x?a=1&amp;b=&apos;2&apos;&quot;&gt;"
        );
        assert_eq!(escape("&amp;"), "&amp;amp;", "no double-escape detection");
    }

    // AC-6.5 (batch 4)
    #[test]
    fn leaves_other_text_unchanged() {
        assert_eq!(escape(""), "");
        assert_eq!(
            escape("plain text / café – ünï 123"),
            "plain text / café – ünï 123"
        );
    }
}
