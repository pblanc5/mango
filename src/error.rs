#[derive(thiserror::Error, Debug)]
pub enum MangoError {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("frontmatter error: {0}")]
    Frontmatter(String),

    #[error("general error: {0}")]
    General(String),

    #[error("template error")]
    Template(#[from] tera::Error)
}