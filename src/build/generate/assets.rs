use std::{fs, path::Path};

use crate::error::MangoError;

pub fn build(assets_source: &Path, asset_dest: &Path) -> Result<(), MangoError> {
    if !assets_source.is_dir() {
        let msg = format!(
            "the path '{}' is not a directory",
            assets_source.to_str().unwrap_or_default()
        );
        return Err(MangoError::General(msg));
    }

    fs::create_dir_all(asset_dest).map_err(|e| MangoError::io_at(asset_dest, e))?;

    let children = fs::read_dir(assets_source).map_err(|e| MangoError::io_at(assets_source, e))?;
    for child in children {
        let child = child.map_err(|e| MangoError::io_at(assets_source, e))?;
        let filename = &child.file_name();
        let name = Path::new(filename);
        let source_path = child.path();
        let dest_path = asset_dest.join(name);

        let file_type = child
            .file_type()
            .map_err(|e| MangoError::io_at(&source_path, e))?;
        if file_type.is_dir() {
            build(&source_path, &dest_path)?;
            continue;
        }

        fs::copy(&source_path, &dest_path).map_err(|e| MangoError::io_at(&source_path, e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

        build(&src, &dest).unwrap();

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

        let err = build(&src, &dest).expect_err("copy onto a directory must fail");
        assert!(err.to_string().contains("style.css"), "{err}");
    }
}
