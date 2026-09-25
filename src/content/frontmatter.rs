use serde::Deserialize;

use crate::error::MangoError;

const FRONTMATTER_DELIMITER: &str = "---";
const BYTE_ORDER_MARK: char = '\u{feff}';

/// Every key a page's frontmatter may contain, in the README field table's
/// order. Must match the fields of `MangoFrontmatter`; a unit test keeps the
/// two in step.
const ACCEPTED_KEYS: [&str; 6] = ["title", "author", "description", "date", "tags", "draft"];

/// The keys a page's frontmatter must contain, in the order they are reported.
const REQUIRED_KEYS: [&str; 4] = ["title", "author", "description", "draft"];

// No `deny_unknown_fields`: on invalid JSON the typed parse would then report
// an unknown key met before the syntax error instead of today's error.
// `check_keys` rejects unknown keys; `accepted_keys_match_the_struct_fields`
// keeps `ACCEPTED_KEYS` and these fields in step.
#[derive(Deserialize, Debug)]
pub struct MangoFrontmatter {
    pub title: String,
    pub author: String,
    pub description: String,
    pub date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub draft: bool,
}

/// Splits a leading `---` JSON `---` block from the markdown body. A UTF-8
/// byte order mark (added by some Windows editors) is ignored.
pub fn parse(content: String) -> Result<(Option<MangoFrontmatter>, String), MangoError> {
    let content = match content.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => rest.to_owned(),
        None => content,
    };
    let mut lines = content.lines();

    if lines.next().map(|l| l.trim()) != Some(FRONTMATTER_DELIMITER) {
        return Ok((None, content.to_string()));
    }

    let mut json_lines = Vec::new();

    for line in lines.by_ref() {
        if line.trim() == FRONTMATTER_DELIMITER {
            let json = json_lines.join("\n");
            let body = lines.collect::<Vec<_>>().join("\n");
            let fm = parse_json(&json)?;

            return Ok((Some(fm), body));
        }

        json_lines.push(line);
    }

    Err(MangoError::Frontmatter(
        "unterminated frontmatter block".into(),
    ))
}

/// Checks the keys of a JSON object first, so every unknown and missing key
/// is reported together and ahead of any other problem, whatever order the
/// keys appear in. Anything else (invalid JSON, a non-object, or an object
/// whose keys are fine) goes to the typed parse and keeps its exact error.
fn parse_json(json: &str) -> Result<MangoFrontmatter, MangoError> {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) {
        check_keys(&map)?;
    }
    serde_json::from_str::<MangoFrontmatter>(json)
        .map_err(|e| MangoError::Frontmatter(e.to_string()))
}

/// One error listing every unknown key (sorted byte-wise) and then every
/// missing required key (in `REQUIRED_KEYS` order), plus the accepted keys
/// when any key is unknown. A present key counts whatever its value.
fn check_keys(map: &serde_json::Map<String, serde_json::Value>) -> Result<(), MangoError> {
    let mut unknown: Vec<&str> = map
        .keys()
        .map(String::as_str)
        .filter(|key| !ACCEPTED_KEYS.contains(key))
        .collect();
    unknown.sort_unstable();
    let missing = REQUIRED_KEYS.iter().filter(|key| !map.contains_key(**key));

    let problems: Vec<String> = unknown
        .iter()
        .map(|key| format!("unknown '{key}'"))
        .chain(missing.map(|key| format!("missing '{key}'")))
        .collect();
    if problems.is_empty() {
        return Ok(());
    }

    let mut msg = format!("invalid frontmatter keys: {}", problems.join(", "));
    if !unknown.is_empty() {
        let accepted: Vec<String> = ACCEPTED_KEYS.iter().map(|key| format!("'{key}'")).collect();
        msg.push_str(&format!("; accepted keys are {}", accepted.join(", ")));
    }
    Err(MangoError::Frontmatter(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JSON: &str = r#"{
        "title": "Post One",
        "author": "tester",
        "description": "my first post",
        "date": "2026-01-24",
        "tags": ["blog"],
        "draft": false
    }"#;

    #[test]
    fn parses_valid_frontmatter_and_returns_body() {
        let content = format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld");
        let (fm, body) = parse(content).unwrap();

        let fm = fm.expect("frontmatter should be present");
        assert_eq!(fm.title, "Post One");
        assert_eq!(fm.author, "tester");
        assert_eq!(fm.date.as_deref(), Some("2026-01-24"));
        assert_eq!(fm.tags, Some(vec!["blog".to_string()]));
        assert!(!fm.draft);
        assert_eq!(body, "# Hello\n\nworld");
    }

    #[test]
    fn byte_order_mark_is_ignored() {
        let content = format!("\u{feff}---\n{VALID_JSON}\n---\nbody");
        let (fm, body) = parse(content).unwrap();

        assert_eq!(fm.expect("frontmatter behind a BOM").title, "Post One");
        assert_eq!(body, "body");
    }

    #[test]
    fn optional_fields_may_be_omitted() {
        let json = r#"{"title": "t", "author": "a", "description": "d", "draft": true}"#;
        let content = format!("---\n{json}\n---\nbody");
        let (fm, _) = parse(content).unwrap();

        let fm = fm.unwrap();
        assert_eq!(fm.date, None);
        assert_eq!(fm.tags, None);
    }

    #[test]
    fn returns_none_when_no_frontmatter() {
        let content = "# Just markdown\n\nno frontmatter here".to_string();
        let (fm, body) = parse(content.clone()).unwrap();

        assert!(fm.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn malformed_json_is_an_error() {
        let content = "---\n{not valid json\n---\nbody".to_string();
        assert!(matches!(parse(content), Err(MangoError::Frontmatter(_))));
    }

    /// Today's typed-parse error text for `json`, so the pins below compare
    /// against serde's own wording instead of hard-coding it.
    fn typed_parse_error(json: &str) -> String {
        serde_json::from_str::<MangoFrontmatter>(json)
            .unwrap_err()
            .to_string()
    }

    fn frontmatter_error(json: &str) -> String {
        match parse(format!("---\n{json}\n---\nbody")) {
            Err(MangoError::Frontmatter(msg)) => msg,
            other => panic!("expected a Frontmatter error for {json}, got {other:?}"),
        }
    }

    // AC-risk-4.1.2, AC-risk-4.5.4 [baseline] A wrong-typed value is serde's
    // `invalid type` error, unchanged.
    #[test]
    fn wrong_typed_value_is_the_typed_parse_error() {
        let json = r#"{"title": 5, "author": "a", "description": "d", "draft": false}"#;
        let msg = frontmatter_error(json);
        assert_eq!(msg, typed_parse_error(json));
        assert!(msg.contains("invalid type"), "{msg}");
    }

    // AC-risk-4.1.2, AC-risk-4.5.4 [baseline] A duplicate known key is serde's
    // `duplicate field` error, unchanged.
    #[test]
    fn duplicate_known_key_is_the_typed_parse_error() {
        let json =
            r#"{"title": "t", "title": "u", "author": "a", "description": "d", "draft": false}"#;
        let msg = frontmatter_error(json);
        assert_eq!(msg, typed_parse_error(json));
        assert!(msg.contains("duplicate field"), "{msg}");
    }

    // AC-risk-4.1.2, AC-risk-4.5.1 [baseline] Valid JSON that is not an object
    // keeps the typed parser's error.
    #[test]
    fn non_object_json_is_the_typed_parse_error() {
        for json in ["42", r#""text""#] {
            assert_eq!(frontmatter_error(json), typed_parse_error(json), "{json}");
        }
    }

    // AC-risk-4.5.1 Invalid JSON keeps the typed parser's error, with no key
    // errors, even when an unknown key comes before the syntax error (which
    // is why `MangoFrontmatter` has no `deny_unknown_fields`).
    #[test]
    fn invalid_json_is_the_typed_parse_error() {
        let json = r#"{"titel": "t", "tag": []"#;
        let msg = frontmatter_error(json);
        assert_eq!(msg, typed_parse_error(json));
        assert!(!msg.contains("unknown"), "{msg}");
    }

    const ACCEPTED_SUFFIX: &str =
        "; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'";

    // AC-risk-4.3.1, AC-risk-4.4.2, AC-risk-4.4.3
    #[test]
    fn unknown_key_is_an_error_listing_the_accepted_keys() {
        let json =
            r#"{"title": "t", "author": "a", "description": "d", "draft": false, "tag": ["rust"]}"#;
        assert_eq!(
            frontmatter_error(json),
            format!("invalid frontmatter keys: unknown 'tag'{ACCEPTED_SUFFIX}")
        );
    }

    // AC-risk-4.3.2
    #[test]
    fn keys_are_compared_exactly() {
        for key in ["Title", "TAGS", "title "] {
            let json = format!(
                r#"{{"title": "t", "author": "a", "description": "d", "draft": false, "{key}": 1}}"#
            );
            assert_eq!(
                frontmatter_error(&json),
                format!("invalid frontmatter keys: unknown '{key}'{ACCEPTED_SUFFIX}"),
                "{key:?}"
            );
        }
    }

    // AC-risk-4.3.4
    #[test]
    fn only_accepted_keys_parse_as_before() {
        let (fm, _) = parse(format!("---\n{VALID_JSON}\n---\nbody")).unwrap();
        assert_eq!(fm.expect("frontmatter").title, "Post One");
        for json in [
            r#"{"title": "t", "author": "a", "description": "d", "draft": false}"#,
            r#"{"title": "t", "author": "a", "description": "d", "date": "2026-01-24", "draft": false}"#,
            r#"{"title": "t", "author": "a", "description": "d", "tags": [], "draft": false}"#,
        ] {
            let (fm, _) = parse(format!("---\n{json}\n---\nbody")).unwrap();
            assert_eq!(fm.expect("frontmatter").title, "t", "{json}");
        }
    }

    // AC-risk-4.4.2, AC-risk-4.4.4
    #[test]
    fn every_unknown_and_missing_key_is_listed_in_a_fixed_order() {
        let expected = format!(
            "invalid frontmatter keys: unknown 'tag', unknown 'titel', missing 'title'{ACCEPTED_SUFFIX}"
        );
        for json in [
            r#"{"titel": "t", "tag": [], "author": "a", "description": "d", "draft": false}"#,
            r#"{"draft": false, "description": "d", "tag": [], "author": "a", "titel": "t"}"#,
        ] {
            assert_eq!(frontmatter_error(json), expected, "{json}");
        }
    }

    // AC-risk-4.4.4
    #[test]
    fn unknown_keys_are_sorted_byte_wise_and_missing_keys_in_field_order() {
        let json = r#"{"b": 1, "a": 1, "Z": 1}"#;
        assert_eq!(
            frontmatter_error(json),
            format!(
                "invalid frontmatter keys: unknown 'Z', unknown 'a', unknown 'b', missing 'title', missing 'author', missing 'description', missing 'draft'{ACCEPTED_SUFFIX}"
            )
        );
    }

    // AC-risk-4.4.5
    #[test]
    fn empty_object_reports_every_missing_key_without_the_accepted_list() {
        assert_eq!(
            frontmatter_error("{}"),
            "invalid frontmatter keys: missing 'title', missing 'author', missing 'description', missing 'draft'"
        );
    }

    // AC-risk-4.4.6
    #[test]
    fn a_repeated_key_is_named_once() {
        for json in [
            r#"{"title": "t", "author": "a", "description": "d", "draft": false, "tag": 1, "tag": 2}"#,
            r#"{"title": "t", "title": "u", "author": "a", "description": "d", "draft": false, "tag": 1}"#,
        ] {
            let msg = frontmatter_error(json);
            assert_eq!(
                msg,
                format!("invalid frontmatter keys: unknown 'tag'{ACCEPTED_SUFFIX}"),
                "{json}"
            );
            assert!(!msg.contains("duplicate field"), "{msg}");
        }
    }

    // AC-risk-4.4 (REQ intro): a present key counts whatever its value.
    #[test]
    fn null_valued_key_counts_as_present() {
        let json =
            r#"{"title": null, "author": "a", "description": "d", "draft": false, "tag": 1}"#;
        let msg = frontmatter_error(json);
        assert_eq!(
            msg,
            format!("invalid frontmatter keys: unknown 'tag'{ACCEPTED_SUFFIX}")
        );
        assert!(!msg.contains("missing"), "{msg}");
    }

    // AC-risk-4.5.2
    #[test]
    fn key_errors_win_over_wrong_typed_values_in_any_order() {
        let expected = format!("invalid frontmatter keys: unknown 'tag'{ACCEPTED_SUFFIX}");
        for json in [
            r#"{"title": 5, "author": "a", "description": "d", "draft": false, "tag": 1}"#,
            r#"{"tag": 1, "title": 5, "author": "a", "description": "d", "draft": false}"#,
            r#"{"title": "t", "author": "a", "description": "d", "draft": "no", "tag": 1}"#,
            r#"{"tag": 1, "title": "t", "author": "a", "description": "d", "draft": "no"}"#,
        ] {
            let msg = frontmatter_error(json);
            assert_eq!(msg, expected, "{json}");
            assert!(!msg.contains("invalid type"), "{msg}");
        }
    }

    // AC-risk-4.1.1 The accepted keys and the struct's fields cannot drift
    // apart: the typed parser alone accepts exactly `ACCEPTED_KEYS`.
    #[test]
    fn accepted_keys_match_the_struct_fields() {
        use serde::de::{Error as _, Visitor, value::Error};

        /// Answers `deserialize_struct` with the struct's field names, in
        /// declaration order, as the error text.
        struct FieldNames;
        impl<'de> serde::Deserializer<'de> for FieldNames {
            type Error = Error;
            fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
                Err(Error::custom("not a struct"))
            }
            fn deserialize_struct<V: Visitor<'de>>(
                self,
                _: &'static str,
                fields: &'static [&'static str],
                _: V,
            ) -> Result<V::Value, Error> {
                Err(Error::custom(fields.join(",")))
            }
            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                tuple_struct map enum identifier ignored_any
            }
        }

        let fields = MangoFrontmatter::deserialize(FieldNames)
            .unwrap_err()
            .to_string();
        assert_eq!(fields, ACCEPTED_KEYS.join(","));

        // Exactly `REQUIRED_KEYS` are required by the typed parser.
        let all = serde_json::json!({
            "title": "t", "author": "a", "description": "d",
            "date": "2026-01-24", "tags": [], "draft": false
        });
        serde_json::from_value::<MangoFrontmatter>(all.clone()).expect("all keys");
        for key in ACCEPTED_KEYS {
            let mut without = all.clone();
            without.as_object_mut().unwrap().remove(key);
            let result = serde_json::from_value::<MangoFrontmatter>(without);
            assert_eq!(result.is_err(), REQUIRED_KEYS.contains(&key), "'{key}'");
        }
    }

    #[test]
    fn unterminated_block_is_an_error() {
        let content = "---\n{\"title\": \"t\"}\nno closing delimiter".to_string();
        assert!(matches!(parse(content), Err(MangoError::Frontmatter(_))));
    }

    // The CRLF tests below are characterization tests: they pin what `parse`
    // already does with Windows line endings so a regression is caught. Inputs
    // are built from the LF source text by `crlf`, never written with literal
    // `\r\n` in a multi-line literal, so they hold however this file is checked
    // out (`core.autocrlf`, `.gitattributes`).

    /// Rewrites every line ending to CRLF, whatever it started as.
    fn crlf(text: &str) -> String {
        lf(text).replace('\n', "\r\n")
    }

    /// Rewrites every line ending to LF, whatever it started as.
    fn lf(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    // AC-12.1
    #[test]
    fn parses_crlf_frontmatter_and_normalizes_body() {
        let content = crlf(&format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld\n"));
        let (fm, body) = parse(content).unwrap();

        let fm = fm.expect("frontmatter should be present");
        assert_eq!(fm.title, "Post One");
        assert_eq!(fm.author, "tester");
        assert_eq!(fm.description, "my first post");
        assert_eq!(fm.date.as_deref(), Some("2026-01-24"));
        assert_eq!(fm.tags, Some(vec!["blog".to_string()]));
        assert!(!fm.draft);
        for field in [&fm.title, &fm.author, &fm.description] {
            assert!(!field.contains('\r'), "carriage return in {field:?}");
        }
        // The body is normalized to LF and its final newline is dropped,
        // exactly as for the LF input in `parses_valid_frontmatter_and_returns_body`.
        assert_eq!(body, "# Hello\n\nworld");
        assert!(!body.contains('\r'), "carriage return in body: {body:?}");
    }

    // AC-12.2
    #[test]
    fn crlf_and_lf_documents_parse_identically() {
        // Both inputs come from the same source text, so they cannot drift apart.
        let source = format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld\n");
        let (lf_fm, lf_body) = parse(lf(&source)).unwrap();
        let (crlf_fm, crlf_body) = parse(crlf(&source)).unwrap();

        let lf_fm = lf_fm.expect("LF frontmatter");
        let crlf_fm = crlf_fm.expect("CRLF frontmatter");
        assert_eq!(crlf_body, lf_body);
        // Equality alone is only a symmetry check: a change that gave *both*
        // inputs CRLF bodies would keep it green. Pin the shared value too, so
        // this test fails on the direction of the normalization, not just its
        // consistency.
        assert_eq!(crlf_body, "# Hello\n\nworld");
        assert!(!crlf_body.contains('\r'), "carriage return in body");
        assert_eq!(crlf_fm.title, lf_fm.title);
        assert_eq!(crlf_fm.author, lf_fm.author);
        assert_eq!(crlf_fm.description, lf_fm.description);
        assert_eq!(crlf_fm.date, lf_fm.date);
        assert_eq!(crlf_fm.tags, lf_fm.tags);
        assert_eq!(crlf_fm.draft, lf_fm.draft);
    }

    // AC-12.3
    #[test]
    fn byte_order_mark_with_crlf_is_ignored() {
        // The BOM is stripped before the content is split into lines, so the
        // two quirks compose.
        let content = format!(
            "\u{feff}{}",
            crlf(&format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld\n"))
        );
        let (fm, body) = parse(content).unwrap();

        assert_eq!(fm.expect("frontmatter behind a BOM").title, "Post One");
        assert_eq!(body, "# Hello\n\nworld");
    }

    // AC-12.4
    #[test]
    fn parses_mixed_lf_and_crlf_line_endings() {
        // Written with explicit escapes: the endings differ line by line.
        let content = concat!(
            "---\r\n",
            "{\"title\": \"t\", \"author\": \"a\",\r\n",
            " \"description\": \"d\", \"draft\": false}\n",
            "---\n",
            "# Hello\r\n",
            "\n",
            "world\r\n",
        )
        .to_string();
        let (fm, body) = parse(content).unwrap();

        assert_eq!(fm.expect("frontmatter should be present").title, "t");
        assert_eq!(body, "# Hello\n\nworld");
        assert!(!body.contains('\r'), "carriage return in body: {body:?}");
    }

    // AC-12.5
    #[test]
    fn unterminated_crlf_block_is_an_error() {
        let content = crlf("---\n{\"title\": \"t\"}\nno closing delimiter\n");
        assert!(matches!(parse(content), Err(MangoError::Frontmatter(_))));
    }

    // AC-12.6
    #[test]
    fn crlf_body_is_returned_verbatim_when_no_frontmatter() {
        // The "no frontmatter" early return hands back the input unchanged, so
        // this is the one path that does not normalize CRLF to LF. It cannot be
        // reached from the CLI today (`loader::load` turns `None` into the
        // `missing frontmatter` build error); the assertion records current
        // behavior rather than a guarantee. ARCH-7 proposes replacing this
        // return with an error, which would retire this test.
        let content = crlf("# Just markdown\n\nno frontmatter here\n");
        let (fm, body) = parse(content.clone()).unwrap();

        assert!(fm.is_none());
        assert_eq!(body, content);
        assert!(body.contains("\r\n"), "CRLF should survive: {body:?}");
    }
}
