use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    build::output::{Body, Output, OutputKind},
    error::MangoError,
};

/// Lists every file under `assets_source` (recursively, sorted by path, so by
/// destination) as one asset-copy output each, copied into the `folder`
/// folder under the output folder. Nothing is copied here: `commit` does that,
/// so a missing assets folder and output conflicts are caught before the
/// output folder is cleaned.
pub fn plan(assets_source: &Path, folder: &Path) -> Result<Vec<Output>, MangoError> {
    if !assets_source.is_dir() {
        let msg = format!(
            "the path '{}' is not a directory",
            assets_source.to_str().unwrap_or_default()
        );
        return Err(MangoError::General(msg));
    }

    let mut files = Vec::new();
    collect(assets_source, assets_source, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files
        .into_iter()
        .map(|(rel, source)| Output {
            kind: OutputKind::Asset {
                folder: folder.to_path_buf(),
                rel,
            },
            body: Body::Copy(source),
        })
        .collect())
}

/// Gathers `(path relative to root, source path)` for every file under `dir`.
fn collect(root: &Path, dir: &Path, files: &mut Vec<(PathBuf, PathBuf)>) -> Result<(), MangoError> {
    for child in fs::read_dir(dir).map_err(|e| MangoError::io_at(dir, e))? {
        let child = child.map_err(|e| MangoError::io_at(dir, e))?;
        let source = child.path();
        let file_type = child
            .file_type()
            .map_err(|e| MangoError::io_at(&source, e))?;
        let rel = source
            .strip_prefix(root)
            .map_err(|_| {
                MangoError::General(format!(
                    "asset '{}' is outside the assets folder '{}'",
                    source.display(),
                    root.display()
                ))
            })?
            .to_path_buf();

        // `file_type()` above does not follow symlinks; `metadata` does, so a
        // link to a folder is recognised here instead of blowing up in the
        // commit-time copy after the output folder was cleaned. A dangling
        // link or a symlink loop fails here too, for the same reason.
        let target = fs::metadata(&source).map_err(|e| MangoError::io_at(&source, e))?;
        if target.is_dir() {
            if file_type.is_symlink() {
                return Err(MangoError::General(format!(
                    "asset '{}' is a symlink to a directory ('{}'): symlinked folders are not copied; replace it with a real folder",
                    rel.to_string_lossy().replace('\\', "/"),
                    source.display()
                )));
            }
            collect(root, &source, files)?;
            continue;
        }

        files.push((rel, source));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::output;
    use tera::Tera;

    /// Renders and writes planned assets under `dir`, the way `commit` does.
    fn copy_into(dir: &Path, outputs: Vec<Output>) -> Result<(), MangoError> {
        output::write(&output::render(&Tera::default(), dir, outputs)?)
    }

    fn fixture_dir(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/assets")
            .join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    // AC-4.1
    #[test]
    fn copies_all_files_in_all_nested_directories() {
        let dir = fixture_dir("copies_all_files");
        let src = dir.join("src");
        let dest = dir.join("dest");
        let files = [
            "a_dir/a.css",
            "a_dir/nested/deep.js",
            "b_dir/b.css",
            "one.txt",
            "two.txt",
            "three.txt",
        ];
        for f in files {
            write_file(&src.join(f), f);
        }

        copy_into(&dir, plan(&src, Path::new("dest")).unwrap()).unwrap();

        for f in files {
            let copied = dest.join(f);
            assert!(copied.is_file(), "missing {f}");
            assert_eq!(fs::read_to_string(copied).unwrap(), f);
        }
    }

    // AC-4.2
    #[test]
    fn copy_failure_is_an_error_naming_the_path() {
        let dir = fixture_dir("copy_failure");
        let src = dir.join("src");
        let dest = dir.join("dest");
        write_file(&src.join("style.css"), "body {}");
        // Destination already exists as a directory, so the copy must fail.
        fs::create_dir_all(dest.join("style.css")).unwrap();

        let err = copy_into(&dir, plan(&src, Path::new("dest")).unwrap())
            .expect_err("copy onto a directory must fail");
        assert!(err.to_string().contains("style.css"), "{err}");
    }

    // AC-arch-2.5.4: a copy failure names the asset's source path, not its
    // destination (RISK-7).
    #[test]
    fn copy_failure_names_the_source_path() {
        let dir = fixture_dir("copy_failure_names_the_source_path");
        let src = dir.join("src");
        let dest = dir.join("dest");
        write_file(&src.join("style.css"), "body {}");
        fs::create_dir_all(dest.join("style.css")).unwrap();

        match copy_into(&dir, plan(&src, Path::new("dest")).unwrap()) {
            Err(MangoError::IoPath { path, .. }) => assert_eq!(path, src.join("style.css")),
            other => panic!("expected IoPath, got {other:?}"),
        }
    }

    #[test]
    fn plan_is_sorted_labelled_and_touches_nothing() {
        let dir = fixture_dir("plan_is_sorted");
        let src = dir.join("src");
        let dest = dir.join("dest");
        write_file(&src.join("z.txt"), "z");
        write_file(&src.join("a/b.css"), "b");

        let files = plan(&src, Path::new("dest")).unwrap();

        let labels: Vec<_> = files.iter().map(|f| f.kind.to_string()).collect();
        assert_eq!(labels, ["asset 'a/b.css'", "asset 'z.txt'"]);
        assert_eq!(files[0].path(&dir), dest.join("a/b.css"));
        assert!(!dest.exists(), "plan must not create the destination");
    }

    // AC-10.1, AC-10.2, AC-10.3
    #[cfg(unix)]
    #[test]
    fn plan_rejects_symlinked_directory() {
        use std::os::unix::fs::symlink;

        for (link, expected) in [("vendor", "vendor"), ("css/vendor", "css/vendor")] {
            let dir = fixture_dir("plan_rejects_symlinked_directory");
            let src = dir.join("src");
            let dest = dir.join("dest");
            write_file(&src.join("css/main.css"), "body {}");
            write_file(&dir.join("theme/reset.css"), "* {}");
            fs::create_dir_all(src.join(link).parent().unwrap()).unwrap();
            symlink(dir.join("theme"), src.join(link)).unwrap();

            let err = plan(&src, Path::new("dest")).expect_err("symlinked folder must be rejected");

            assert!(matches!(err, MangoError::General(_)), "{err:?}");
            let msg = err.to_string();
            assert!(msg.contains(&format!("asset '{expected}'")), "{msg}");
            assert!(msg.contains("symlink to a directory"), "{msg}");
            assert!(!dest.exists(), "plan must not create the destination");
        }
    }

    // AC-10.5
    #[cfg(unix)]
    #[test]
    fn plan_fails_on_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let dir = fixture_dir("plan_fails_on_dangling_symlink");
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        symlink(dir.join("nowhere"), src.join("broken.css")).unwrap();

        let err = plan(&src, Path::new("dest")).expect_err("dangling symlink must fail");
        assert!(matches!(err, MangoError::IoPath { .. }), "{err:?}");
        assert!(err.to_string().contains("broken.css"), "{err}");
    }

    // AC-10.6
    #[cfg(unix)]
    #[test]
    fn plan_fails_on_symlink_loop() {
        use std::os::unix::fs::symlink;

        let dir = fixture_dir("plan_fails_on_symlink_loop");
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        symlink(src.join("b"), src.join("a")).unwrap();
        symlink(src.join("a"), src.join("b")).unwrap();

        let err = plan(&src, Path::new("dest")).expect_err("symlink loop must fail");
        assert!(matches!(err, MangoError::IoPath { .. }), "{err:?}");
        let msg = err.to_string();
        let a = src.join("a").display().to_string();
        let b = src.join("b").display().to_string();
        assert!(msg.contains(&a) || msg.contains(&b), "{msg}");
    }

    // AC-10.7
    #[cfg(unix)]
    #[test]
    fn plan_and_copy_follow_symlinked_files() {
        use std::os::unix::fs::symlink;

        let dir = fixture_dir("plan_and_copy_follow_symlinked_files");
        let src = dir.join("src");
        let dest = dir.join("dest");
        fs::create_dir_all(&src).unwrap();
        write_file(&dir.join("shared/reset.css"), "* { margin: 0 }");
        symlink(dir.join("shared/reset.css"), src.join("reset.css")).unwrap();

        let files = plan(&src, Path::new("dest")).unwrap();

        let labels: Vec<_> = files.iter().map(|f| f.kind.to_string()).collect();
        assert_eq!(labels, ["asset 'reset.css'"]);
        copy_into(&dir, files).unwrap();
        let copied = dest.join("reset.css");
        assert!(!copied.is_symlink(), "the link target must be copied");
        assert_eq!(fs::read_to_string(copied).unwrap(), "* { margin: 0 }");
    }

    #[test]
    fn plan_fails_on_missing_assets_folder() {
        let dir = fixture_dir("plan_missing_folder");
        let missing = dir.join("nope");

        let err = plan(&missing, Path::new("dest")).expect_err("missing assets folder");
        assert!(
            !dir.join("dest").exists(),
            "plan must not create the destination"
        );
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert!(err.to_string().contains("nope"), "{err}");
    }
}
