mod cli;
mod content;
mod render;
mod build;
mod error;

fn main() {
    match cli::run() {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e)
        }
    }
}
