use regex::Regex;

/// Formats HTML with simple block-aware indentation and newline normalization.
pub fn format_html(raw: &str) -> String {
    if raw.trim().is_empty() {
        return raw.to_string();
    }

    if raw.matches('<').count() != raw.matches('>').count() {
        return raw.to_string();
    }

    let tag_re = Regex::new(r"(?s)(<[^>]+>)").expect("tag regex should compile");
    let close_re = Regex::new(r"^</\s*([A-Za-z0-9:_-]+)").expect("close regex should compile");
    let open_re = Regex::new(r"^<\s*([A-Za-z0-9:_-]+)").expect("open regex should compile");

    let block_closers = [
        "</div>",
        "</p>",
        "</li>",
        "</tr>",
        "</td>",
        "</th>",
        "</ul>",
        "</ol>",
        "</table>",
        "</section>",
        "</article>",
        "</header>",
        "</footer>",
        "</nav>",
        "</main>",
    ];
    let void_tags = ["br", "hr", "img", "meta", "link", "input"];

    let mut depth = 0usize;
    let mut stack: Vec<String> = Vec::new();
    let mut out = String::new();

    let mut tokens: Vec<String> = Vec::new();
    let mut last = 0usize;
    for m in tag_re.find_iter(raw) {
        if m.start() > last {
            tokens.push(raw[last..m.start()].to_string());
        }
        tokens.push(m.as_str().to_string());
        last = m.end();
    }
    if last < raw.len() {
        tokens.push(raw[last..].to_string());
    }

    for token in tokens {
        if token.starts_with('<') && token.ends_with('>') {
            let line = token.trim();

            if let Some(caps) = close_re.captures(line) {
                let name = caps[1].to_ascii_lowercase();
                if let Some(last) = stack.pop() {
                    if last != name {
                        return raw.to_string();
                    }
                } else {
                    return raw.to_string();
                }
                depth = depth.saturating_sub(1);
            }

            out.push_str(&" ".repeat(depth * 2));
            out.push_str(line);
            out.push('\n');

            if let Some(caps) = open_re.captures(line) {
                let name = caps[1].to_ascii_lowercase();
                let is_closing = line.starts_with("</");
                let self_closing = line.ends_with("/>") || void_tags.contains(&name.as_str());
                if !is_closing && !self_closing {
                    stack.push(name);
                    depth += 1;
                }
            }

            if block_closers
                .iter()
                .any(|closer| line.eq_ignore_ascii_case(closer))
            {
                out.push('\n');
            }
        } else {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                out.push_str(&" ".repeat(depth * 2));
                out.push_str(trimmed);
                out.push('\n');
            }
        }
    }

    if !stack.is_empty() {
        return raw.to_string();
    }

    let compact_re = Regex::new(r"\n{3,}").expect("newline regex should compile");
    compact_re.replace_all(out.trim_end(), "\n\n").to_string()
}

#[cfg(test)]
mod tests {
    use super::format_html;

    #[test]
    fn block_tags_get_newlines() {
        let raw = "<div><p>Hello</p><p>World</p></div>";
        let out = format_html(raw);
        assert!(out.contains("</p>\n"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn malformed_passthrough() {
        let raw = "<div><p>oops</div>";
        let out = format_html(raw);
        assert_eq!(out, raw);
    }
}
