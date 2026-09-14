use std::{fs, io::ErrorKind, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::MangoError;

/// Config file used when `--config` is not given, relative to the cwd.
pub const DEFAULT_CONFIG_PATH: &str = "mango.json";

const DEFAULT_RECENT_COUNT: usize = 10;

/// Site-wide metadata exposed to every template as `config`.
///
/// Unset string fields serialize as `null` (not `""`): Tera renders `null`
/// as empty text and treats it as falsy, and unlike `""` it still triggers
/// the `default` filter, so `{{ config.title | default(value="My Site") }}`
/// works. Every key is always present.
#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct SiteConfig {
    pub title: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
    pub recent_count: usize,
}

impl Default for SiteConfig {
    fn default() -> Self {
        SiteConfig {
            title: None,
            author: None,
            description: None,
            base_url: None,
            recent_count: DEFAULT_RECENT_COUNT,
        }
    }
}

/// Loads the site config. A missing file is fine unless the path was given
/// explicitly; malformed JSON or unknown fields are errors naming the file.
pub fn load(path: &Path, explicit: bool) -> Result<SiteConfig, MangoError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound && !explicit => {
            return Ok(SiteConfig::default());
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let msg = format!("config file '{}' does not exist", path.display());
            return Err(MangoError::Config(msg));
        }
        Err(e) => return Err(MangoError::io_at(path, e)),
    };

    serde_json::from_str(&content)
        .map_err(|e| MangoError::Config(format!("'{}': {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/config")
            .join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_config(test_name: &str, content: &str) -> PathBuf {
        let path = fixture_dir(test_name).join("mango.json");
        fs::write(&path, content).unwrap();
        path
    }

    // AC-1.2, AC-1.9
    #[test]
    fn missing_default_file_uses_defaults() {
        let path = fixture_dir("missing_default_file_uses_defaults").join("mango.json");
        let config = load(&path, false).unwrap();
        assert_eq!(config, SiteConfig::default());
        assert_eq!(config.title, None);
        assert_eq!(config.author, None);
        assert_eq!(config.description, None);
        assert_eq!(config.base_url, None);
        assert_eq!(config.recent_count, 10);
    }

    // AC-1.4
    #[test]
    fn explicit_missing_file_is_config_error_naming_path() {
        let path = fixture_dir("explicit_missing_file").join("nope.json");
        let err = load(&path, true).expect_err("explicit missing config must fail");
        assert!(matches!(err, MangoError::Config(_)), "{err:?}");
        assert!(
            err.to_string().contains(&path.display().to_string()),
            "{err}"
        );
    }

    // AC-1.3, AC-1.9
    #[test]
    fn partial_config_keeps_defaults() {
        let path = write_config("partial_config_keeps_defaults", r#"{"title": "T"}"#);
        let config = load(&path, false).unwrap();
        assert_eq!(config.title.as_deref(), Some("T"));
        assert_eq!(config.author, None);
        assert_eq!(config.recent_count, 10);

        let path = write_config(
            "full_config",
            r#"{"title": "T", "author": "A", "description": "D", "base_url": "https://x", "recent_count": 3}"#,
        );
        let config = load(&path, true).unwrap();
        assert_eq!(config.author.as_deref(), Some("A"));
        assert_eq!(config.description.as_deref(), Some("D"));
        assert_eq!(config.base_url.as_deref(), Some("https://x"));
        assert_eq!(config.recent_count, 3);
    }

    // AC-1.5
    #[test]
    fn malformed_json_is_config_error_naming_path() {
        let bad = "{not valid json";
        let path = write_config("malformed_json", bad);
        let serde_msg = serde_json::from_str::<serde_json::Value>(bad)
            .unwrap_err()
            .to_string();

        let err = load(&path, false).expect_err("malformed config must fail");
        assert!(matches!(err, MangoError::Config(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains(&path.display().to_string()), "{msg}");
        assert!(msg.contains(&serde_msg), "expected '{serde_msg}' in: {msg}");
    }

    // AC-1.6
    #[test]
    fn unknown_field_is_config_error_naming_path_and_field() {
        let path = write_config("unknown_field", r#"{"titel": "Typo"}"#);
        let err = load(&path, false).expect_err("unknown field must fail");
        assert!(matches!(err, MangoError::Config(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains(&path.display().to_string()), "{msg}");
        assert!(msg.contains("titel"), "{msg}");
    }

    // AC-1.7
    #[test]
    fn negative_recent_count_is_config_error() {
        for (i, value) in ["-1", "2.5", "\"3\""].iter().enumerate() {
            let path = write_config(
                &format!("bad_recent_count_{i}"),
                &format!(r#"{{"recent_count": {value}}}"#),
            );
            let err = load(&path, false).expect_err(value);
            assert!(matches!(err, MangoError::Config(_)), "{value}: {err:?}");
            assert!(
                err.to_string().contains(&path.display().to_string()),
                "{value}: {err}"
            );
        }
    }

    // AC-1.10
    #[test]
    fn unset_fields_serialize_as_null() {
        let json = serde_json::to_value(SiteConfig::default()).unwrap();
        for key in ["title", "author", "description", "base_url"] {
            assert!(json.get(key).is_some_and(|v| v.is_null()), "{key}: {json}");
        }
        assert_eq!(json["recent_count"], 10);
    }
}
