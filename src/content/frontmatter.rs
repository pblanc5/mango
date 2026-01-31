use serde::Deserialize;

use crate::error::MangoError;

const FRONTMATTER_DELIMITER: &str = "---";

#[derive(Deserialize, Debug)]
pub struct MangoFrontmatter {
    pub title: String,
    pub author: String,
    pub date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub draft: bool
}

pub fn parse(content: String) -> Result<(Option<MangoFrontmatter>, String), MangoError> {
    let mut lines = content.lines();

    if lines.next().map(|l| l.trim()) != Some(FRONTMATTER_DELIMITER) {
        return Ok((None, content.to_string()));
    }

    let mut json_lines = Vec::new();

    for line in lines.by_ref() {
        if line.trim() == FRONTMATTER_DELIMITER {
            let json = json_lines.join("\n");
            let body = lines.collect::<Vec<_>>().join("\n");
            let fm = serde_json::from_str::<MangoFrontmatter>(&json)
                .map_err(|e| MangoError::Frontmatter(e.to_string()))?;

            return Ok((Some(fm), body));
        }

        json_lines.push(line);
    }

    Err(MangoError::Frontmatter("unterminated frontmatter block".into()))
}