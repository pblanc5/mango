#[derive(thiserror::Error, Debug)]
pub enum MangoError {
    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("frontmatter error: {0}")]
    Frontmatter(String),

    #[error("user error: {0}")]
    User(String),

    #[error("template error")]
    Template(#[from] tera::Error)
}