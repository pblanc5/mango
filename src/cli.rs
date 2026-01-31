use std::path::Path;

use clap::{Args, Parser, Subcommand};

use crate::{content::{loader, }, error::MBError};

#[derive(Args, Debug)]
struct BuildOpts {
    // path to templates
    #[arg(long, default_value = "templates")]
    templates: String,

    // path to themes
    #[arg(long, default_value = "themes")]
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
enum MBActions {
    // compile site
    Build(BuildOpts),
     
    // run site
    Run(ServerOpts)
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct MBCli {
    // project root
    #[arg(default_value = ".")]
    project: String,

    // mango actions
    #[command(subcommand)]
    command: MBActions,
}

pub fn run() -> Result<(), MBError> {
    let args = MBCli::parse();
    //let path = Path::new(&args.directory);
    //_ = compile_site(path);

    match args.command {
        MBActions::Build(opts) => {

            let project_path = Path::new(&args.project);
            let site_path = project_path.join(&opts.site);
            println!("{}", site_path.to_str().unwrap_or_else(|| "no path found"));
            let pages = loader::load(site_path.as_path())?;

            //let templates = &opts.templates;
            //let template_glob = String::from(templates) + "/**/*.html";
            //let tera = match Tera::new(&template_glob) {
            //    Ok(t) => t,
            //    Err(e) => {
            //        eprintln!("{}", e.to_string());
            //        ::std::process::exit(1);
            //    }
            //};

            //use tera::Context;
            //let mut context = Context::new();
            //let page = Page {
            //    title: String::from("Test Page"),
            //    author: String::from("me"),
            //    date: String::from("today"),
            //    slug: String::from("test-page"),
            //    tags: vec![String::from("test")],
            //    content: String::from("Welcome to my page")
            //};

            //context.insert("page", &page);
            //let template = "page.html";
            //let html = match tera.render(template, &context) {
            //    Ok(r) => r,
            //    Err(e) => {
            //        eprintln!("{}", e.to_string());
            //        ::std::process::exit(1);
            //
            //    }
            //};


            for page in pages {
                println!("{:?}", page);
            }
            

        },

        MBActions::Run(opts) => {
            print!("{}:{}", opts.address, opts.port);
            return Ok(())
        }

        
    };

    Ok(())
}