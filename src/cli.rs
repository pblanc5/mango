use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use clap::{Args, Parser, Subcommand};

use crate::{
    build::{
        generate::{assets, content, section},
        index, output,
    },
    content::loader,
    error::MangoError,
    render::template,
};

#[derive(Args, Debug)]
struct BuildOpts {
    /// Path to the templates directory
    #[arg(long, default_value = "meta/templates")]
    templates: String,

    /// Path to the assets directory
    #[arg(long, default_value = "meta/assets")]
    assets: String,

    /// Path to the site content directory
    #[arg(long, default_value = "site")]
    site: String,

    /// Output directory for the built site
    #[arg(short, long, default_value = "dist")]
    output: String,
}

// The dev server is future work; the options are kept so the CLI surface
// stays stable, but nothing reads them yet.
#[allow(dead_code)]
#[derive(Args, Debug)]
struct ServerOpts {
    /// Address for the dev server
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,

    /// Port for the dev server
    #[arg(short, long, default_value = "8080")]
    port: u16,
}

#[derive(Args, Debug)]
struct CleanOpts {
    /// Output directory to remove
    #[arg(short, long, default_value = "dist")]
    dist: String,
}

#[derive(Subcommand, Debug)]
enum MangoActions {
    /// Build the site
    Build(BuildOpts),

    /// Run the dev server (not implemented yet)
    Run(ServerOpts),

    /// Publish the site (not implemented yet)
    Publish,

    /// Remove the output directory
    Clean(CleanOpts),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct MangoCli {
    #[command(subcommand)]
    command: MangoActions,
}

pub fn run() -> Result<(), MangoError> {
    let args = MangoCli::parse();
    let project_path = Path::new(".");

    match args.command {
        MangoActions::Build(opts) => build(project_path, opts),

        MangoActions::Run(_) => Err(not_implemented("run")),

        MangoActions::Publish => Err(not_implemented("publish")),

        MangoActions::Clean(opts) => {
            let dist_path = Path::new(&opts.dist);
            clean(dist_path)
        }
    }
}

fn not_implemented(command: &str) -> MangoError {
    MangoError::General(format!("the '{command}' command is not implemented yet"))
}

fn build(project_path: &Path, opts: BuildOpts) -> Result<(), MangoError> {
    let site_path = project_path.join(&opts.site);
    let templates = Path::new(&opts.templates);
    let assets = Path::new(&opts.assets);
    let dist = Path::new(&opts.output);

    // Everything that can fail on bad input happens before the output
    // folder is touched, so a failed build leaves the previous output intact.
    let pages = loader::load(site_path.as_path())?;
    let tera = template::load_templates(templates)?;

    let asset_dest = match assets.file_name() {
        Some(name) => dist.join(name),
        None => {
            let msg = format!(
                "the assets path '{}' has no directory name to copy into the output",
                assets.display()
            );
            return Err(MangoError::General(msg));
        }
    };

    let page_items = content::build(&pages)?;
    let si = index::section::build_section_index(&pages);
    let section_items = section::build(si);

    output::check_collisions(dist, page_items.iter().chain(section_items.iter()))?;

    let rendered_pages = output::render(&tera, dist, &page_items)?;
    let rendered_sections = output::render(&tera, dist, &section_items)?;

    let cwd = current_dir()?;
    ensure_safe_to_clean(dist, &cwd, &[site_path.as_path(), templates, assets])?;
    clean_contents(dist)?;

    output::write(&rendered_pages)?;
    output::write(&rendered_sections)?;

    assets::build(assets, &asset_dest)?;

    Ok(())
}

fn clean(dist: &Path) -> Result<(), MangoError> {
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

fn current_dir() -> Result<PathBuf, MangoError> {
    std::env::current_dir().map_err(|e| MangoError::io_at(".", e))
}

/// Refuses to clean `target` if it is the cwd, a parent of the cwd, or equal
/// to / a parent of any existing protected path. Paths are compared
/// canonicalized. A missing `target` is safe: there is nothing to delete.
fn ensure_safe_to_clean(target: &Path, cwd: &Path, protected: &[&Path]) -> Result<(), MangoError> {
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
fn clean_contents(dist: &Path) -> Result<(), MangoError> {
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
        } else {
            fs::remove_file(&path).map_err(|e| MangoError::io_at(&path, e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/cli")
            .join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    // AC-4.4
    #[test]
    fn refuses_cwd() {
        let dir = fixture_dir("refuses_cwd");
        let err = ensure_safe_to_clean(&dir, &dir, &[]).expect_err("cwd must be refused");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert!(
            err.to_string().contains(&dir.display().to_string()),
            "{err}"
        );
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
}
