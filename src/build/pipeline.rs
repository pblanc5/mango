use std::path::{Path, PathBuf};

use crate::{
    build::{
        clean::{clean_contents, current_dir, ensure_safe_to_clean},
        generate::{assets, content, feed, home, section, sitemap, tag},
        index,
        output::{self, Contents, RenderedOutput},
    },
    config,
    content::loader,
    error::MangoError,
    render::template,
};

/// The inputs a build needs: the same paths `mango build` accepts. Every path
/// is used exactly as given except `site`, which `plan` joins onto `.`.
#[derive(Debug)]
pub struct BuildOptions {
    pub site: PathBuf,
    pub templates: PathBuf,
    pub assets: PathBuf,
    pub output: PathBuf,
    /// `None` reads `mango.json` from the current directory, and falls back
    /// to the defaults when that file does not exist.
    pub config: Option<PathBuf>,
}

/// A complete, fully rendered build, held in memory and bound to the one
/// output folder it was planned against. Only [`plan`] can produce one and
/// only [`commit`] can write one out.
#[derive(Debug)]
pub struct BuildPlan {
    output_dir: PathBuf,
    /// Input paths the output folder may neither be nor contain.
    protected: Vec<PathBuf>,
    /// Every output, rendered, in the order it is written: files first, then
    /// asset copies (the planned list puts assets last).
    outputs: Vec<RenderedOutput>,
}

/// One thing a [`BuildPlan`] would put in the output folder.
#[derive(Debug, PartialEq, Eq)]
pub enum PlannedOutput<'a> {
    /// A rendered or generated file: path relative to the output folder, and
    /// the exact contents that will be written.
    File { path: &'a Path, contents: &'a str },
    /// An asset copy: destination relative to the output folder, and the
    /// source file it is copied from.
    Copy { path: &'a Path, source: &'a Path },
}

impl BuildPlan {
    /// The output folder this plan was planned against and the only folder it
    /// can be committed into.
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Every output this plan would produce: files first (pages, section
    /// indexes, home, tag index and tag pages, then the feed and sitemap),
    /// then asset copies — the order [`commit`] writes them in.
    pub fn outputs(&self) -> impl Iterator<Item = PlannedOutput<'_>> {
        self.outputs.iter().map(|output| {
            let path = self.relative(&output.path);
            match &output.contents {
                Contents::Text(contents) => PlannedOutput::File {
                    path,
                    contents: contents.as_str(),
                },
                Contents::Copy(source) => PlannedOutput::Copy {
                    path,
                    source: source.as_path(),
                },
            }
        })
    }

    /// Paths are stored absolute-to-`output_dir` because collision messages
    /// and `output::write` use the full path. Every one of them is built by
    /// joining onto `output_dir` inside `plan`, the only constructor, so
    /// `strip_prefix` cannot fail here.
    fn relative<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.output_dir).unwrap_or(path)
    }
}

/// Loads, checks and renders everything, writing nothing. Every error a build
/// can raise on bad input (content, file names, tags, dates, config,
/// templates, assets, collisions, rendering) happens here, in this order, so
/// a plan that exists is a build that cannot fail on its inputs any more.
pub fn plan(opts: &BuildOptions) -> Result<BuildPlan, MangoError> {
    // The `.` join is preserved from the pre-split `cli::build`, so an error
    // about the site folder still names it `./<site>`. ARCH-7 removes it.
    let site_path = Path::new(".").join(&opts.site);
    let templates = opts.templates.as_path();
    let assets = opts.assets.as_path();
    let dist = opts.output.as_path();
    let config_explicit = opts.config.is_some();
    let config_path = opts
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new(config::DEFAULT_CONFIG_PATH));

    let pages = loader::load(site_path.as_path())?;
    let config = config::load(config_path, config_explicit)?;
    let tera = template::load_templates(templates)?;

    // The folder under the output folder that assets are copied into.
    let asset_folder = match assets.file_name() {
        Some(name) => PathBuf::from(name),
        None => {
            let msg = format!(
                "the assets path '{}' has no directory name to copy into the output",
                assets.display()
            );
            return Err(MangoError::General(msg));
        }
    };
    // Listed now, copied by `commit`: a missing assets folder or an asset
    // that clashes with a page must fail before anything is cleaned.
    let asset_outputs = assets::plan(assets, &asset_folder)?;

    // One list, in the order outputs are checked, rendered, enumerated and
    // written: pages, sections, home, tag index and tag pages, feed, sitemap,
    // then asset copies (last, so `commit` writes every file before copying).
    let mut outputs = content::build(&pages, &config)?;
    let si = index::section::build_section_index(&pages);
    // The home output reads the index, so build it before `section::build`
    // consumes it.
    let home_output = home::build(&pages, &si, &config);
    outputs.extend(section::build(si, &config));
    outputs.push(home_output);
    outputs.extend(tag::build(index::tag::build_tag_index(&pages), &config));
    // The feed and sitemap need `base_url` and are skipped without it. The
    // sitemap picks the HTML outputs out of the list by kind.
    outputs.extend(feed::build(&pages, &config));
    let sitemap = sitemap::build(&outputs, &config);
    outputs.extend(sitemap);
    outputs.extend(asset_outputs);

    output::check_collisions(dist, &outputs)?;
    let outputs = output::render(&tera, dist, outputs)?;

    Ok(BuildPlan {
        output_dir: dist.to_path_buf(),
        protected: vec![
            site_path,
            templates.to_path_buf(),
            assets.to_path_buf(),
            config_path.to_path_buf(),
        ],
        outputs,
    })
}

/// Writes a plan out. This is the only operation in the crate that empties or
/// writes into a build's output folder, and it takes nothing but a plan, so
/// the "nothing is touched until everything has succeeded" guarantee is a
/// property of the seam rather than of statement order.
pub fn commit(plan: BuildPlan) -> Result<(), MangoError> {
    let cwd = current_dir()?;
    let protected: Vec<&Path> = plan.protected.iter().map(PathBuf::as_path).collect();
    ensure_safe_to_clean(&plan.output_dir, &cwd, &protected)?;
    clean_contents(&plan.output_dir)?;

    output::write(&plan.outputs)?;

    Ok(())
}
