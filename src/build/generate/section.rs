use crate::{
    build::index::section::SectionIndex,
    render::template::{self, RenderItem},
};

/// Render items come out in ascending section-slug order (the index is a
/// `BTreeMap`).
pub fn build(si: SectionIndex) -> Vec<RenderItem> {
    si.sections
        .into_iter()
        .map(|(s, p)| template::render_section_page(s, p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build::index::section::build_section_index,
        content::{
            frontmatter::MangoFrontmatter,
            page::{Page, PageType},
        },
    };

    fn page_with_slug(slug: &str) -> Page {
        let fm = MangoFrontmatter {
            title: slug.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: None,
            tags: None,
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General).unwrap();
        page.slug = slug.to_string();
        page
    }

    // AC-2.4, AC-2.5
    #[test]
    fn render_items_in_section_slug_order() {
        let pages = vec![
            page_with_slug("projects/x"),
            page_with_slug("posts/x"),
            page_with_slug("a/b/c"),
            page_with_slug("a/x"),
        ];

        let items = build(build_section_index(&pages));
        let slugs: Vec<_> = items.iter().map(|i| i.slug.as_str()).collect();
        assert_eq!(slugs, ["a", "a/b", "posts", "projects"]);
        assert_eq!(items[2].source, "section index 'posts'");
    }
}
