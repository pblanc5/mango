use std::{error::Error as _, path::PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MangoError {
    #[error("Mango I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Mango I/O Error at '{}': {source}", path.display())]
    IoPath {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Mango Frontmatter Error: {0}")]
    Frontmatter(String),

    #[error("Mango Error: {0}")]
    General(String),

    #[error("Mango Template Error: {}", tera_chain(.0))]
    Template(#[from] tera::Error),
}

impl MangoError {
    /// I/O error tied to the file or directory it happened on.
    pub fn io_at(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        MangoError::IoPath {
            path: path.into(),
            source,
        }
    }
}

/// Tera's top-level message is often generic ("Failed to render 'x'"); the
/// real cause lives in the source chain, so join every level.
fn tera_chain(err: &tera::Error) -> String {
    let mut msg = err.to_string();
    let mut source = err.source();
    while let Some(s) = source {
        msg.push_str(": ");
        msg.push_str(&s.to_string());
        source = s.source();
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    // AC-2.1
    #[test]
    fn io_display_includes_underlying_message() {
        let err = MangoError::from(Error::new(ErrorKind::NotFound, "disk went away"));
        assert_eq!(err.to_string(), "Mango I/O Error: disk went away");
    }

    // AC-2.2, AC-1.6, AC-2.7
    #[test]
    fn io_path_display_includes_path_and_cause() {
        let err = MangoError::io_at(
            "site/posts/x.md",
            Error::new(ErrorKind::PermissionDenied, "denied"),
        );
        let msg = err.to_string();
        assert!(msg.contains("site/posts/x.md"), "{msg}");
        assert!(msg.contains("denied"), "{msg}");
    }

    // AC-2.3
    #[test]
    fn template_display_includes_source_chain() {
        let tera_err = tera::Error::chain(
            "Failed to render 'page.html'",
            tera::Error::msg("Variable `x` not found"),
        );
        let msg = MangoError::from(tera_err).to_string();
        assert!(msg.contains("Failed to render 'page.html'"), "{msg}");
        assert!(msg.contains("Variable `x` not found"), "{msg}");
    }
}
