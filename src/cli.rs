use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};

use mango::{BuildOptions, MangoError};

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

    /// Path to the site config file (default: mango.json)
    #[arg(long)]
    config: Option<String>,
}

impl BuildOpts {
    fn into_options(self) -> BuildOptions {
        BuildOptions {
            site: PathBuf::from(self.site),
            templates: PathBuf::from(self.templates),
            assets: PathBuf::from(self.assets),
            output: PathBuf::from(self.output),
            config: self.config.map(PathBuf::from),
        }
    }
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
    Run,

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

    match args.command {
        MangoActions::Build(opts) => mango::commit(mango::plan(&opts.into_options())?),

        MangoActions::Run => Err(not_implemented("run")),

        MangoActions::Clean(opts) => mango::clean(Path::new(&opts.dist)),
    }
}

fn not_implemented(command: &str) -> MangoError {
    MangoError::General(format!("the '{command}' command is not implemented yet"))
}
