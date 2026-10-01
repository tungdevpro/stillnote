use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};

/// Renders Markdown to HTML for the preview pane.
/// Raw HTML is shown as text and only http(s)/mailto links are kept, so a
/// note can never run script inside the app.
pub fn render(source: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let events = Parser::new_ext(source, opts).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        Event::Start(Tag::Link { link_type, dest_url, title, id }) => Event::Start(Tag::Link {
            link_type,
            dest_url: safe_url(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image { link_type, dest_url, title, id }) => Event::Start(Tag::Image {
            link_type,
            dest_url: safe_url(dest_url),
            title,
            id,
        }),
        other => other,
    });

    let mut out = String::with_capacity(source.len() * 3 / 2);
    html::push_html(&mut out, events);
    out
}

fn safe_url(url: CowStr<'_>) -> CowStr<'_> {
    let lower = url.trim().to_ascii_lowercase();
    if ["http://", "https://", "mailto:"].iter().any(|p| lower.starts_with(p)) {
        url
    } else {
        CowStr::Borrowed("#")
    }
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn renders_basic_markdown() {
        let out = render("# Tiêu đề\n\n- [x] xong\n\n**đậm**");
        assert!(out.contains("<h1>Tiêu đề</h1>"));
        assert!(out.contains("checkbox"));
        assert!(out.contains("<strong>đậm</strong>"));
    }

    #[test]
    fn escapes_raw_html_and_unsafe_links() {
        let out = render("<script>alert(1)</script>\n\n[x](javascript:alert(1)) <img src=x onerror=y>");
        assert!(!out.contains("<script>"));
        assert!(!out.contains("<img"));
        assert!(out.contains("href=\"#\""));
        assert!(render("[ok](https://example.com)").contains("href=\"https://example.com\""));
    }
}
