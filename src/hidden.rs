use std::ffi::OsStr;

/// An entry is hidden when its own name, as listed in its parent folder,
/// starts with `.` (RISK-10). Hidden entries under the site and assets
/// folders are skipped without being resolved, read or checked.
pub(crate) fn is_hidden(name: &OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-risk-10.3.1, AC-risk-10.3.2, AC-risk-10.3.3
    #[test]
    fn names_starting_with_a_dot_are_hidden() {
        for name in [
            ".notes.md",
            ".drafts",
            ".git",
            ".DS_Store",
            ".#post.md",
            "...md",
            "..md",
        ] {
            assert!(is_hidden(OsStr::new(name)), "{name}");
        }
    }

    // AC-risk-10.3.1, AC-risk-10.3.2, AC-risk-10.3.3
    #[test]
    fn other_names_are_not_hidden() {
        for name in [
            "notes.md",
            "_notes.md",
            "a.b",
            "notes.",
            "#post.md#",
            "~notes.md",
            "",
        ] {
            assert!(!is_hidden(OsStr::new(name)), "{name}");
        }
    }

    // AC-risk-10.3.2: only the first byte counts, even for a name that is
    // not UTF-8.
    #[cfg(unix)]
    #[test]
    fn non_utf8_names_are_judged_by_their_first_byte() {
        use std::os::unix::ffi::OsStrExt;

        assert!(is_hidden(OsStr::from_bytes(b".\xff")));
        assert!(!is_hidden(OsStr::from_bytes(b"\xff.")));
    }
}
