//! Markdown parser for extracting @imports, links, and checking XML tags

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::ops::Range;
use std::sync::OnceLock;

static XML_TAG_REGEX: OnceLock<Regex> = OnceLock::new();

/// Extract @import references from markdown content (excluding code blocks/spans)
pub fn extract_imports(content: &str) -> Vec<Import> {
    let line_starts = compute_line_starts(content);
    let mut imports = Vec::new();

    let parser = Parser::new_ext(content, Options::all()).into_offset_iter();
    let mut in_code_block = false;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Code(_) => {}
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) if !in_code_block => {
                scan_imports_in_text(&text, range, &line_starts, &mut imports);
            }
            _ => {}
        }
    }

    imports
}

/// Extract XML tags for balance checking (excluding code blocks/spans)
pub fn extract_xml_tags(content: &str) -> Vec<XmlTag> {
    let line_starts = compute_line_starts(content);
    let mut tags = Vec::new();

    let parser = Parser::new_ext(content, Options::all()).into_offset_iter();
    let mut in_code_block = false;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Code(_) => {}
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) if !in_code_block => {
                scan_xml_tags_in_text(&text, range, &line_starts, &mut tags);
            }
            _ => {}
        }
    }

    tags
}

/// Extract markdown links from content (excluding code blocks/spans)
///
/// This extracts both regular links `[text](url)` and image links `![alt](url)`.
pub fn extract_markdown_links(content: &str) -> Vec<MarkdownLink> {
    let line_starts = compute_line_starts(content);
    let mut links = Vec::new();

    let parser = Parser::new_ext(content, Options::all()).into_offset_iter();
    let mut in_code_block = false;

    // Track current link being built
    let mut current_link: Option<(String, bool, Range<usize>)> = None; // (url, is_image, range)
    let mut link_text = String::new();

    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Code(_) => {}

            Event::Start(Tag::Link { dest_url, .. }) if !in_code_block => {
                current_link = Some((dest_url.to_string(), false, range));
                link_text.clear();
            }

            Event::Start(Tag::Image { dest_url, .. }) if !in_code_block => {
                current_link = Some((dest_url.to_string(), true, range));
                link_text.clear();
            }

            Event::Text(text) if current_link.is_some() && !in_code_block => {
                link_text.push_str(&text);
            }

            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) if !in_code_block => {
                if let Some((url, is_image, link_range)) = current_link.take() {
                    let (line, column) = line_col_at(link_range.start, &line_starts);
                    links.push(MarkdownLink {
                        url,
                        text: std::mem::take(&mut link_text),
                        is_image,
                        line,
                        column,
                        start_byte: link_range.start,
                        end_byte: link_range.end,
                    });
                }
            }

            _ => {}
        }
    }

    links
}

/// Check if XML tags are balanced
pub fn check_xml_balance(tags: &[XmlTag]) -> Vec<XmlBalanceError> {
    check_xml_balance_with_content_end(tags, None)
}

/// Check if XML tags are balanced, with optional content length for auto-fix byte positions
pub fn check_xml_balance_with_content_end(
    tags: &[XmlTag],
    content_len: Option<usize>,
) -> Vec<XmlBalanceError> {
    let mut stack: Vec<&XmlTag> = Vec::new();
    let mut errors = Vec::new();

    for tag in tags {
        if tag.is_closing {
            if let Some(last) = stack.last() {
                if last.name == tag.name {
                    stack.pop();
                } else {
                    errors.push(XmlBalanceError::Mismatch {
                        expected: last.name.clone(),
                        found: tag.name.clone(),
                        line: tag.line,
                        column: tag.column,
                    });
                }
            } else {
                errors.push(XmlBalanceError::UnmatchedClosing {
                    tag: tag.name.clone(),
                    line: tag.line,
                    column: tag.column,
                });
            }
        } else {
            stack.push(tag);
        }
    }

    // Unclosed tags - compute content_end_byte for auto-fix
    // For each unclosed tag, the closing tag should be inserted at the end of content
    // (or at the start of the next tag at the same/lower nesting level)
    let content_end = content_len.unwrap_or_else(|| tags.last().map(|t| t.end_byte).unwrap_or(0));

    for tag in stack {
        errors.push(XmlBalanceError::Unclosed {
            tag: tag.name.clone(),
            line: tag.line,
            column: tag.column,
            open_tag_end_byte: tag.end_byte,
            content_end_byte: content_end,
        });
    }

    errors
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// A markdown link extracted from content
#[derive(Debug, Clone)]
pub struct MarkdownLink {
    /// The URL/path of the link
    pub url: String,
    /// The link text (alt text for images)
    pub text: String,
    /// Whether this is an image link (![alt](url))
    pub is_image: bool,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset of link start
    pub start_byte: usize,
    /// Byte offset of link end
    pub end_byte: usize,
}

#[derive(Debug, Clone)]
pub struct XmlTag {
    pub name: String,
    pub is_closing: bool,
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone)]
pub enum XmlBalanceError {
    Unclosed {
        tag: String,
        line: usize,
        column: usize,
        /// Byte position of the opening tag (for auto-fix)
        open_tag_end_byte: usize,
        /// Byte position where the closing tag should be inserted (content end)
        content_end_byte: usize,
    },
    UnmatchedClosing {
        tag: String,
        line: usize,
        column: usize,
    },
    Mismatch {
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },
}

fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let mut low = 0usize;
    let mut high = line_starts.len();
    while low + 1 < high {
        let mid = (low + high) / 2;
        if line_starts[mid] <= offset {
            low = mid;
        } else {
            high = mid;
        }
    }
    let line_start = line_starts[low];
    (low + 1, offset - line_start + 1)
}

fn scan_imports_in_text(
    text: &str,
    range: Range<usize>,
    line_starts: &[usize],
    imports: &mut Vec<Import>,
) {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let prev_ok = if i == 0 {
                true
            } else {
                let prev = text[..i].chars().last().unwrap_or(' ');
                !prev.is_alphanumeric() && !matches!(prev, '_' | '-' | '.')
            };
            if !prev_ok {
                i += 1;
                continue;
            }

            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                let b = bytes[j];
                let allowed = b.is_ascii_alphanumeric()
                    || matches!(b, b'_' | b'-' | b'.' | b'/' | b'\\' | b':' | b'~');
                if !allowed {
                    break;
                }
                j += 1;
            }

            if j == start {
                i += 1;
                continue;
            }

            let mut end = j;
            while end > start {
                let b = bytes[end - 1];
                if matches!(b, b'.' | b',' | b';' | b':') {
                    end -= 1;
                } else {
                    break;
                }
            }

            if end == start {
                i = j;
                continue;
            }

            let path = text[start..end].to_string();
            if !is_probable_import_path(&path) {
                i = j;
                continue;
            }
            let start_byte = range.start + i;
            let end_byte = range.start + end;
            let (line, column) = line_col_at(start_byte, line_starts);

            imports.push(Import {
                path,
                line,
                column,
                start_byte,
                end_byte,
            });

            i = j;
            continue;
        }

        i += 1;
    }
}

fn scan_xml_tags_in_text(
    text: &str,
    range: Range<usize>,
    line_starts: &[usize],
    tags: &mut Vec<XmlTag>,
) {
    let re = XML_TAG_REGEX.get_or_init(|| Regex::new(r"<(/?)([a-zA-Z_][a-zA-Z0-9_-]*)>").unwrap());

    for cap in re.captures_iter(text) {
        let is_closing = cap.get(1).is_some_and(|m| m.as_str() == "/");
        if let Some(name_match) = cap.get(2) {
            let name = name_match.as_str().to_string();
            let start = cap.get(0).unwrap().start();
            let end = cap.get(0).unwrap().end();
            let start_byte = range.start + start;
            let end_byte = range.start + end;
            let (line, column) = line_col_at(start_byte, line_starts);
            tags.push(XmlTag {
                name,
                is_closing,
                line,
                column,
                start_byte,
                end_byte,
            });
        }
    }
}

fn is_probable_import_path(path: &str) -> bool {
    if path.starts_with('~')
        || path.contains('/')
        || path.contains('\\')
        || path.contains('.')
        || path.contains(':')
    {
        return true;
    }
    path.chars().any(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_imports() {
        let content = "See @docs/guide.md and @README.md";
        let imports = extract_imports(content);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].path, "docs/guide.md");
        assert_eq!(imports[1].path, "README.md");
    }

    #[test]
    fn test_extract_imports_ignores_inline_code() {
        let content = "Use `@not-an-import.md` but see @real.md";
        let imports = extract_imports(content);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, "real.md");
    }

    #[test]
    fn test_extract_imports_ignores_code_block() {
        let content = "```\nimport x from '@pkg/name'\n```\nSee @actual.md";
        let imports = extract_imports(content);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, "actual.md");
    }

    #[test]
    fn test_extract_imports_ignores_plain_mentions() {
        let content = "Use @import and @imports in docs";
        let imports = extract_imports(content);
        assert!(imports.is_empty());
    }

    #[test]
    fn test_xml_balance() {
        let content = "<example>test</example>";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_xml_ignores_code_block() {
        let content = "```\n<example>test</example>\n```\n";
        let tags = extract_xml_tags(content);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_xml_unclosed() {
        let content = "<example>test";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance(&tags);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], XmlBalanceError::Unclosed { .. }));
    }

    #[test]
    fn test_xml_unclosed_with_content_end() {
        let content = "<example>test content here";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance_with_content_end(&tags, Some(content.len()));
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            XmlBalanceError::Unclosed {
                tag,
                content_end_byte,
                open_tag_end_byte,
                ..
            } => {
                assert_eq!(tag, "example");
                assert_eq!(*content_end_byte, content.len());
                assert_eq!(*open_tag_end_byte, 9); // Length of "<example>"
            }
            _ => panic!("Expected Unclosed error"),
        }
    }

    #[test]
    fn test_xml_balance_multiple_unclosed() {
        let content = "<outer><inner>content";
        let tags = extract_xml_tags(content);
        let errors = check_xml_balance_with_content_end(&tags, Some(content.len()));
        // Both <outer> and <inner> are unclosed
        assert_eq!(errors.len(), 2);
        for err in &errors {
            match err {
                XmlBalanceError::Unclosed {
                    content_end_byte, ..
                } => {
                    assert_eq!(*content_end_byte, content.len());
                }
                _ => panic!("Expected Unclosed error"),
            }
        }
    }

    // ===== Markdown Link Extraction Tests =====

    #[test]
    fn test_extract_markdown_links_basic() {
        let content = "See [guide](docs/guide.md) for more info.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "docs/guide.md");
        assert_eq!(links[0].text, "guide");
        assert!(!links[0].is_image);
    }

    #[test]
    fn test_extract_markdown_links_multiple() {
        let content = "See [one](a.md) and [two](b.md) files.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].url, "a.md");
        assert_eq!(links[1].url, "b.md");
    }

    #[test]
    fn test_extract_markdown_links_image() {
        let content = "Here is ![logo](images/logo.png) image.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "images/logo.png");
        assert_eq!(links[0].text, "logo");
        assert!(links[0].is_image);
    }

    #[test]
    fn test_extract_markdown_links_ignores_code_block() {
        let content = "```\n[link](skip.md)\n```\n[real](keep.md)";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "keep.md");
    }

    #[test]
    fn test_extract_markdown_links_ignores_inline_code() {
        let content = "Use `[not](skip.md)` but see [real](keep.md)";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "keep.md");
    }

    #[test]
    fn test_extract_markdown_links_with_fragment() {
        let content = "See [section](docs/guide.md#section) for details.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "docs/guide.md#section");
    }

    #[test]
    fn test_extract_markdown_links_external() {
        let content = "Visit [GitHub](https://github.com) site.";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://github.com");
    }

    #[test]
    fn test_extract_markdown_links_anchor_only() {
        let content = "Jump to [section](#section-name).";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "#section-name");
    }

    #[test]
    fn test_extract_markdown_links_line_column() {
        let content = "Line one\n[link](file.md)\nLine three";
        let links = extract_markdown_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].line, 2);
        assert_eq!(links[0].column, 1);
    }
}
