use thiserror::Error;

#[derive(Error, Debug)]
pub enum MangoError {
    #[error("Mango I/O Error")]
    Io(#[from] std::io::Error),

    #[error("Mango Frontmatter Error: {0}")]
    Frontmatter(String),

    #[error("Mango Error: {0}")]
    General(String),

    #[error("Mango Template Error")]
    Template(#[from] tera::Error)
}