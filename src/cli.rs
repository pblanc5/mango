use std::path::Path;

use clap::{Args, Parser, Subcommand};

use crate::{build::output, content::loader, error::MangoError, render::template::render_page};

#[derive(Args, Debug)]
struct BuildOpts {
    // path to templates
    #[arg(long, default_value = "meta/templates")]
    templates: String,

    // path to themes
    #[arg(long, default_value = "meta/themes")]
    themes: String,

    // site directory name
    #[arg(long, default_value = "site")]
    site: String,

    // output path of compiled site
    #[arg(short, long, default_value = "dist")]
    output: String,
}

#[derive(Args, Debug)]
struct ServerOpts {
    // server address
    #[arg(short, long, default_value = "127.0.0.1")]
    address: String,

    // server port
    #[arg(short, long, default_value = "8080")]
    port: u16
}

#[derive(Subcommand, Debug)]
enum MangoActions {
    // compile site
    Build(BuildOpts),
     
    // run site
    Run(ServerOpts)
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct MangoCli {
    // mango actions
    #[command(subcommand)]
    command: MangoActions,
}

pub fn run() -> Result<(), MangoError> {
    let args = MangoCli::parse();
    let project_path = Path::new(".");

    match args.command {
        MangoActions::Build(opts) => build(project_path, opts),

        MangoActions::Run(opts) => {
            print!("{}:{}", opts.address, opts.port);
            Ok(())
        }   
    }
}

fn build(project_path: &Path, opts: BuildOpts) -> Result<(), MangoError> {
    
    let site_path = project_path.join(&opts.site);
    let pages = loader::load(site_path.as_path())?;

    for page in pages {
        let templates = Path::new(&opts.templates);
        let html = render_page(&page, templates)?;
        let dist = Path::new(&opts.output);
        output::write(dist, page.slug.clone(), html)?;
        println!("{}", page.slug);
    }

    Ok(())
}