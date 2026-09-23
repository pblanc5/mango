use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::error::MangoError;

/// Removes the output directory entirely (`mango clean`). A missing directory
/// is success. The same safety check as a build applies, plus the default
/// `site`, `meta/templates` and `meta/assets` folders under the current
/// directory; there is no config path to protect, since `clean` has no
/// `--config`.
pub fn clean(dist: &Path) -> Result<(), MangoError> {
    let cwd = current_dir()?;
    let protected = [
        cwd.join("site"),
        cwd.join("meta/templates"),
        cwd.join("meta/assets"),
    ];
    let protected: Vec<&Path> = protected.iter().map(PathBuf::as_path).collect();
    ensure_safe_to_clean(dist, &cwd, &protected)?;

    match fs::remove_dir_all(dist) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(MangoError::io_at(dist, e)),
    }
}

pub(crate) fn current_dir() -> Result<PathBuf, MangoError> {
    std::env::current_dir().map_err(|e| MangoError::io_at(".", e))
}

/// Refuses to clean `target` if it is the cwd, a parent of the cwd, or equal
/// to / a parent of any existing protected path. Paths are compared
/// canonicalized. A missing `target` is safe: there is nothing to delete.
pub(crate) fn ensure_safe_to_clean(
    target: &Path,
    cwd: &Path,
    protected: &[&Path],
) -> Result<(), MangoError> {
    let canonical_target = match target.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(MangoError::io_at(target, e)),
    };

    let canonical_cwd = cwd.canonicalize().map_err(|e| MangoError::io_at(cwd, e))?;
    if canonical_cwd.starts_with(&canonical_target) {
        let msg = format!(
            "refusing to clean output path '{}': it is or contains the current directory '{}'",
            target.display(),
            cwd.display()
        );
        return Err(MangoError::General(msg));
    }

    for path in protected {
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if canonical.starts_with(&canonical_target) {
            let msg = format!(
                "refusing to clean output path '{}': it is or contains the input path '{}'",
                target.display(),
                path.display()
            );
            return Err(MangoError::General(msg));
        }
    }

    Ok(())
}

/// Removes everything inside `dist` but keeps the folder. Symlinks are
/// removed, never followed.
pub(crate) fn clean_contents(dist: &Path) -> Result<(), MangoError> {
    let metadata = match fs::metadata(dist) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(MangoError::io_at(dist, e)),
    };

    if !metadata.is_dir() {
        let msg = format!(
            "the output path '{}' exists but is not a directory",
            dist.display()
        );
        return Err(MangoError::General(msg));
    }

    for entry in fs::read_dir(dist).map_err(|e| MangoError::io_at(dist, e))? {
        let entry = entry.map_err(|e| MangoError::io_at(dist, e))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| MangoError::io_at(&path, e))?;

        if file_type.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| MangoError::io_at(&path, e))?;
        } else if let Err(e) = fs::remove_file(&path) {
            // On Windows a symlink to a directory must be removed with
            // `remove_dir`; it still removes only the link.
            if !file_type.is_symlink() {
                return Err(MangoError::io_at(&path, e));
            }
            fs::remove_dir(&path).map_err(|e| MangoError::io_at(&path, e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/clean")
            .join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    // AC-4.4; AC-6.2 (batch 3): a non-canonical target, so the message can
    // be seen to name both the output path and the cwd.
    #[test]
    fn refuses_cwd() {
        let dir = fixture_dir("refuses_cwd");
        fs::create_dir_all(dir.join("child")).unwrap();
        let target = dir.join("child/..");

        let err = ensure_safe_to_clean(&target, &dir, &[]).expect_err("cwd must be refused");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains(&format!("'{}'", target.display())), "{msg}");
        assert!(msg.contains(&format!("'{}'", dir.display())), "{msg}");
    }

    // AC-4.5
    #[test]
    fn refuses_parent_of_cwd() {
        let dir = fixture_dir("refuses_parent_of_cwd");
        let cwd = dir.join("child");
        fs::create_dir_all(&cwd).unwrap();

        let err = ensure_safe_to_clean(&dir, &cwd, &[]).expect_err("parent of cwd refused");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert!(
            err.to_string().contains(&cwd.display().to_string()),
            "{err}"
        );
    }

    // AC-4.6
    #[test]
    fn refuses_protected_path_or_its_parent() {
        let dir = fixture_dir("refuses_protected_path_or_its_parent");
        let cwd = dir.join("cwd");
        let project = dir.join("project");
        let site = project.join("site");
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&site).unwrap();
        let missing = dir.join("does-not-exist");

        for target in [&site, &project] {
            let err = ensure_safe_to_clean(target, &cwd, &[missing.as_path(), &site])
                .expect_err("protected path or its parent must be refused");
            assert!(matches!(err, MangoError::General(_)), "{err:?}");
            let msg = err.to_string();
            assert!(msg.contains(&target.display().to_string()), "{msg}");
            assert!(msg.contains(&site.display().to_string()), "{msg}");
        }

        // A sibling of the protected path is fine.
        let sibling = project.join("dist");
        fs::create_dir_all(&sibling).unwrap();
        ensure_safe_to_clean(&sibling, &cwd, &[&site]).unwrap();
    }

    // AC-4.7
    #[test]
    fn missing_target_is_safe_and_clean_contents_is_noop() {
        let dir = fixture_dir("missing_target_is_safe");
        let missing = dir.join("dist");
        ensure_safe_to_clean(&missing, &dir, &[]).unwrap();
        clean_contents(&missing).unwrap();
        assert!(!missing.exists());
    }

    // AC-4.8
    #[test]
    fn clean_contents_errors_when_target_is_a_file() {
        let dir = fixture_dir("clean_contents_errors_when_target_is_a_file");
        let file = dir.join("dist");
        write_file(&file, "not a dir");

        let err = clean_contents(&file).expect_err("a file is not an output dir");
        assert!(
            err.to_string().contains(&file.display().to_string()),
            "{err}"
        );
        assert!(file.is_file());
    }

    // AC-4.9
    #[test]
    fn clean_contents_empties_but_keeps_dir() {
        let dir = fixture_dir("clean_contents_empties_but_keeps_dir");
        let dist = dir.join("dist");
        let outside = dir.join("outside");
        write_file(&dist.join("a/b/index.html"), "x");
        write_file(&dist.join("stray.txt"), "x");
        write_file(&outside.join("keep.txt"), "keep");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, dist.join("link")).unwrap();

        clean_contents(&dist).unwrap();

        assert!(dist.is_dir(), "output folder itself must be kept");
        assert_eq!(fs::read_dir(&dist).unwrap().count(), 0);
        assert!(
            outside.join("keep.txt").is_file(),
            "symlink target must not be followed"
        );
    }

    // AC-6.3: on Windows a directory symlink can't be removed with
    // `remove_file`. Skips when the process may not create symlinks.
    #[cfg(windows)]
    #[test]
    fn clean_contents_removes_dir_symlink() {
        let dir = fixture_dir("clean_contents_removes_dir_symlink");
        let dist = dir.join("dist");
        let outside = dir.join("outside");
        write_file(&outside.join("keep.txt"), "keep");
        fs::create_dir_all(&dist).unwrap();

        if let Err(e) = std::os::windows::fs::symlink_dir(&outside, dist.join("link")) {
            eprintln!("skipping: cannot create directory symlink: {e}");
            return;
        }

        clean_contents(&dist).unwrap();

        assert!(dist.is_dir());
        assert_eq!(fs::read_dir(&dist).unwrap().count(), 0);
        assert!(outside.join("keep.txt").is_file());
    }
}
