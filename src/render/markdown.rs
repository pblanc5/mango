pub fn to_html(content: String) -> String {
    println!("parsing html");
    let parser = pulldown_cmark::Parser::new(&content);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    return html;
}
