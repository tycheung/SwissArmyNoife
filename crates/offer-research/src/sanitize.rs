//! Strip scripts/styles and collapse markup for untrusted research bodies (`sak241`).

/// Sanitize untrusted HTML/text into plain-ish readable content.
#[must_use]
pub fn sanitize_untrusted(raw: &str) -> String {
    let without_scripts = strip_tag_blocks(raw, "script");
    let without_styles = strip_tag_blocks(&without_scripts, "style");
    let no_tags = strip_tags(&without_styles);
    collapse_ws(&html_unescape_basic(&no_tags))
}

fn strip_tag_blocks(input: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let lower = input.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if let Some(rel) = lower[i..].find(&open) {
            let start = i + rel;
            out.push_str(&input[i..start]);
            let after_open = start + open.len();
            let end = lower[after_open..]
                .find(&close)
                .map_or(input.len(), |r| after_open + r + close.len());
            i = end;
            while i < input.len() && !input.is_char_boundary(i) {
                i += 1;
            }
        } else {
            out.push_str(&input[i..]);
            break;
        }
    }
    out
}

fn strip_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn html_unescape_basic(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn collapse_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_and_tags() {
        let raw = "<html><script>alert(1)</script><p>Hello&nbsp;<b>world</b></p></html>";
        let clean = sanitize_untrusted(raw);
        assert!(!clean.contains("alert"));
        assert!(clean.contains("Hello world"));
        assert!(!clean.contains('<'));
    }
}
