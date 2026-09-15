use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::MangoError;

/// One asset file to copy into the output.
#[derive(Debug)]
pub struct AssetFile {
    pub source: PathBuf,
    /// Final path under the output folder.
    pub dest: PathBuf,
    /// Human-readable origin, used in collision errors: `asset '<relative path>'`.
    pub label: String,
}

/// Lists every file under `assets_source` (recursively, sorted by
/// destination) without copying anything, so a missing assets folder and
/// output conflicts are caught before the output folder is cleaned.
pub fn plan(assets_source: &Path, asset_dest: &Path) -> Result<Vec<AssetFile>, MangoError> {
    if !assets_source.is_dir() {
        let msg = format!(
            "the path '{}' is not a directory",
            assets_source.to_str().unwrap_or_default()
        );
        return Err(MangoError::General(msg));
    }

    let mut files = Vec::new();
    collect(assets_source, assets_source, asset_dest, &mut files)?;
    files.sort_by(|a, b| a.dest.cmp(&b.dest));
    Ok(files)
}

fn collect(
    root: &Path,
    dir: &Path,
    asset_dest: &Path,
    files: &mut Vec<AssetFile>,
) -> Result<(), MangoError> {
    for child in fs::read_dir(dir).map_err(|e| MangoError::io_at(dir, e))? {
        let child = child.map_err(|e| MangoError::io_at(dir, e))?;
        let source = child.path();
        let file_type = child
            .file_type()
            .map_err(|e| MangoError::io_at(&source, e))?;
        if file_type.is_dir() {
            collect(root, &source, asset_dest, files)?;
            continue;
        }

        let rel = source.strip_prefix(root).map_err(|_| {
            MangoError::General(format!(
                "asset '{}' is outside the assets folder '{}'",
                source.display(),
                root.display()
            ))
        })?;
        files.push(AssetFile {
            dest: asset_dest.join(rel),
            label: format!("asset '{}'", rel.to_string_lossy().replace('\\', "/")),
            source,
        });
    }

    Ok(())
}

/// Copies planned assets, creating parent folders. Any failure fails the build.
pub fn copy(files: &[AssetFile]) -> Result<(), MangoError> {
    for file in files {
        if let Some(parent) = file.dest.parent() {
            fs::create_dir_all(parent).map_err(|e| MangoError::io_at(parent, e))?;
        }
        fs::copy(&file.source, &file.dest).map_err(|e| MangoError::io_at(&file.source, e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

        copy(&plan(&src, &dest).unwrap()).unwrap();

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

        let err = copy(&plan(&src, &dest).unwrap()).expect_err("copy onto a directory must fail");
        assert!(err.to_string().contains("style.css"), "{err}");
    }

    #[test]
    fn plan_is_sorted_labelled_and_touches_nothing() {
        let dir = fixture_dir("plan_is_sorted");
        let src = dir.join("src");
        let dest = dir.join("dest");
        write_file(&src.join("z.txt"), "z");
        write_file(&src.join("a/b.css"), "b");

        let files = plan(&src, &dest).unwrap();

        let labels: Vec<_> = files.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(labels, ["asset 'a/b.css'", "asset 'z.txt'"]);
        assert_eq!(files[0].dest, dest.join("a/b.css"));
        assert!(!dest.exists(), "plan must not create the destination");
    }

    #[test]
    fn plan_fails_on_missing_assets_folder() {
        let dir = fixture_dir("plan_missing_folder");
        let missing = dir.join("nope");

        let err = plan(&missing, &dir.join("dest")).expect_err("missing assets folder");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert!(err.to_string().contains("nope"), "{err}");
    }
}
