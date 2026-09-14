mod build;
mod cli;
mod config;
mod content;
mod error;
mod render;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
