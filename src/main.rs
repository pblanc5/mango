mod cli;
mod content;
mod error;

fn main() {
    match cli::run() {
        Ok(_) => (),
        Err(e) => {
            println!("{}", e.to_string())
        }
    }
}
