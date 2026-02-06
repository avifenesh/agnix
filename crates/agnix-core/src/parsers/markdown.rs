//! Markdown parser for extracting @imports, links, and checking XML tags
//!
//! ## Security
//!
//! This module includes size limits to prevent ReDoS (Regular Expression Denial
//! of Service) attacks. The `MAX_REGEX_INPUT_SIZE` constant limits the size of
//! content that will be processed by regex operations.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::ops::Range;
use std::panic::{self, AssertUnwindSafe};

use crate::regex_util::static_regex;

static_regex!(fn xml_tag_regex, r"<(/?)([a-zA-Z_][a-zA-Z0-9_-]*)(?:\s+[^>]*?)?(/?)>");

/// Maximum size (in bytes) for content processed by regex operations.
/// This prevents ReDoS attacks by limiting input size to 64KB.
/// Content larger than this will be processed without regex-based extraction.
///
/// **Design Decision**: 64KB was chosen as a balance between:
/// - Large enough to handle typical documentation files (most are <10KB)
/// - Small enough to prevent ReDoS on pathological regex input
/// - Matches typical page sizes for text processing
/// - 2^16 is a clean power-of-2 boundary
///
/// This limit applies to XML tag extraction, import detection, and link parsing.
/// Files larger than 1 MiB are rejected earlier by `DEFAULT_MAX_FILE_SIZE` in file_utils.rs.
pub const MAX_REGEX_INPUT_SIZE: usize = 65536; // 64KB

/// Extract @import references from markdown content (excluding code blocks/spans)
///
/// # Security
///
/// This function is NOT subject to `MAX_REGEX_INPUT_SIZE` limits because it uses
/// byte-by-byte scanning instead of regex. The limit only applies to regex-based
/// extraction functions (`extract_xml_tags`, `extract_markdown_links`).
pub fn extract_imports(content: &str) -> Vec<Import> {
    // Catch upstream parser panics (e.g., pulldown-cmark bugs) gracefully
    panic::catch_unwind(AssertUnwindSafe(|| extract_imports_inner(content))).unwrap_or_default()
}

fn extract_imports_inner(content: &str) -> Vec<Import> {
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
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn extract_xml_tags(content: &str) -> Vec<XmlTag> {
    // Security: Skip regex processing for oversized content to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    // Catch upstream parser panics (e.g., pulldown-cmark bugs) gracefully
    panic::catch_unwind(AssertUnwindSafe(|| extract_xml_tags_inner(content))).unwrap_or_default()
}

fn extract_xml_tags_inner(content: &str) -> Vec<XmlTag> {
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
///
/// # Security
///
/// This function is NOT subject to `MAX_REGEX_INPUT_SIZE` limits because it uses
/// pulldown-cmark's parser instead of regex. The limit only applies to regex-based
/// extraction (`extract_xml_tags`).
pub fn extract_markdown_links(content: &str) -> Vec<MarkdownLink> {
    // Catch upstream parser panics (e.g., pulldown-cmark bugs) gracefully
    panic::catch_unwind(AssertUnwindSafe(|| extract_markdown_links_inner(content)))
        .unwrap_or_default()
}

fn extract_markdown_links_inner(content: &str) -> Vec<MarkdownLink> {
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

/// Returns true if the tag name is an HTML5 void element.
/// Void elements are elements that cannot have child nodes and never
/// need closing tags (e.g., `<br>`, `<img>`, `<hr>`).
/// See: https://html.spec.whatwg.org/multipage/syntax.html#void-elements
fn is_html5_void_element(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Returns true if the tag name looks like a programming language type parameter
/// rather than an XML/HTML tag. Common in TypeScript, Rust, Java, C# code examples.
fn is_likely_type_parameter(name: &str) -> bool {
    // Single uppercase letter: <T>, <K>, <V>, <E>, <R>
    if name.len() == 1 && name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return true;
    }
    // Common generic type parameter names (PascalCase, all start with uppercase)
    matches!(
        name,
        "Key"
            | "Value"
            | "Item"
            | "Element"
            | "Result"
            | "Error"
            | "Input"
            | "Output"
            | "State"
            | "Props"
            | "Args"
            | "Return"
            | "Option"
            | "Type"
            | "Param"
            | "Config"
            | "Context"
            | "TableName"
            | "DeviceID"
            | "FieldName"
    )
}

fn scan_xml_tags_in_text(
    text: &str,
    range: Range<usize>,
    line_starts: &[usize],
    tags: &mut Vec<XmlTag>,
) {
    // Regex to match XML/HTML tags:
    // - Group 1: "/" if closing tag (e.g., </tag>)
    // - Group 2: tag name
    // - Group 3: "/" if self-closing tag (e.g., <br/> or <img src="..." />)
    // The (?:\s+[^>]*?)? handles attributes like <a id="foo"> or <img src="bar">
    let re = xml_tag_regex();

    for cap in re.captures_iter(text) {
        let is_closing = cap.get(1).is_some_and(|m| m.as_str() == "/");
        let is_self_closing = cap.get(3).is_some_and(|m| m.as_str() == "/");

        // Skip self-closing tags - they don't need balance checking
        // Examples: <br/>, <hr />, <img src="..." />
        if is_self_closing {
            continue;
        }

        if let Some(name_match) = cap.get(2) {
            let name = name_match.as_str().to_string();

            // Skip HTML5 void elements - they never need closing tags.
            // In markdown, these are commonly written without self-closing syntax
            // (e.g., <br> instead of <br/>, <img src="..."> instead of <img ... />)
            if is_html5_void_element(&name) && !is_closing {
                continue;
            }

            // Skip likely type parameters from programming languages.
            // Single uppercase letters (T, K, V) and PascalCase names ending
            // in common generic suffixes are almost always type parameters,
            // not XML tags. These appear frequently in API docs and code examples
            // within agent instructions.
            if !is_closing && is_likely_type_parameter(&name) {
                continue;
            }
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
    // Absolute paths (start with / or \) are always considered imports
    // (they will be rejected later in validation)
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }

    // If path contains directory separator, require a file extension
    // to avoid false positives like "@import/reference" (a label, not a file)
    if path.contains('/') || path.contains('\\') {
        // Must have a file extension (.md, .txt, etc.)
        let has_extension = path.rfind('.').is_some_and(|dot_pos| {
            let after_dot = &path[dot_pos + 1..];
            let last_sep = path.rfind('/').or_else(|| path.rfind('\\'));
            // Extension must come after the last separator
            last_sep.is_none_or(|sep| dot_pos > sep) && !after_dot.is_empty()
        });
        return has_extension;
    }

    // For paths without separators, use existing heuristics
    if path.starts_with('~') || path.contains('.') || path.contains(':') {
        return true;
    }
    path.chars().any(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = xml_tag_regex();
    }

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
    fn test_xml_tags_with_attributes() {
        // HTML anchor tags with id attribute should be properly balanced
        let content = r#"<a id="test"></a>"#;
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2);
        assert!(!tags[0].is_closing); // <a id="test">
        assert!(tags[1].is_closing); // </a>
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty(), "Tags with attributes should balance");
    }

    #[test]
    fn test_xml_tags_with_multiple_attributes() {
        let content = r#"<div class="foo" id="bar">content</div>"#;
        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2);
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_xml_self_closing_tags() {
        // Self-closing tags like <br/> should not cause balance errors
        let content = "<br/>";
        let tags = extract_xml_tags(content);
        assert!(tags.is_empty(), "Self-closing tags should be skipped");
    }

    #[test]
    fn test_xml_self_closing_with_space() {
        // Self-closing tags with space like <br /> should also be skipped
        let content = "<br />";
        let tags = extract_xml_tags(content);
        assert!(
            tags.is_empty(),
            "Self-closing tags with space should be skipped"
        );
    }

    #[test]
    fn test_xml_self_closing_with_attributes() {
        // Self-closing tags with attributes should be skipped
        let content = r#"<img src="test.png" />"#;
        let tags = extract_xml_tags(content);
        assert!(
            tags.is_empty(),
            "Self-closing tags with attributes should be skipped"
        );
    }

    #[test]
    fn test_xml_mixed_tags_and_self_closing() {
        // Mix of regular tags and self-closing tags
        let content = r#"<div><br/><span>text</span><hr /></div>"#;
        let tags = extract_xml_tags(content);
        // Should have: <div>, <span>, </span>, </div> (br and hr are self-closing)
        assert_eq!(tags.len(), 4);
        let errors = check_xml_balance(&tags);
        assert!(errors.is_empty());
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

    // ===== Security: Regex DoS Protection Tests =====

    #[test]
    fn test_xml_tags_skipped_for_oversized_content() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let content_with_tags = format!("<example>{}</example>", large_content);

        let tags = extract_xml_tags(&content_with_tags);
        // Tags should be empty because content exceeds size limit
        // This prevents potential ReDoS attacks
        assert!(
            tags.is_empty(),
            "Oversized content should skip XML tag extraction for security"
        );
    }

    #[test]
    fn test_xml_tags_processed_for_normal_sized_content() {
        // Content just under the limit should still be processed
        let content = "<example>test</example>";
        assert!(content.len() < MAX_REGEX_INPUT_SIZE);

        let tags = extract_xml_tags(content);
        assert_eq!(tags.len(), 2, "Normal sized content should be processed");
    }

    #[test]
    fn test_max_regex_input_size_constant() {
        // Verify the constant is set to 64KB as documented
        assert_eq!(MAX_REGEX_INPUT_SIZE, 65536);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // ===== extract_xml_tags() invariants =====

        #[test]
        fn extract_xml_tags_never_panics(content in ".*") {
            // Should never panic on any input
            let _ = extract_xml_tags(&content);
        }

        #[test]
        fn extract_xml_tags_valid_offsets(content in "[a-zA-Z0-9<>/\\-_ ]{0,1000}") {
            let tags = extract_xml_tags(&content);
            for tag in &tags {
                // Byte offsets must be within bounds
                prop_assert!(tag.start_byte <= content.len());
                prop_assert!(tag.end_byte <= content.len());
                prop_assert!(tag.start_byte <= tag.end_byte);

                // Line/column must be positive (1-indexed)
                prop_assert!(tag.line >= 1);
                prop_assert!(tag.column >= 1);
            }
        }

        #[test]
        fn check_xml_balance_never_panics(content in ".*") {
            let tags = extract_xml_tags(&content);
            // Should never panic on any input
            let _ = check_xml_balance(&tags);
            let _ = check_xml_balance_with_content_end(&tags, Some(content.len()));
        }

        // ===== extract_imports() invariants =====

        #[test]
        fn extract_imports_never_panics(content in ".*") {
            // Should never panic on any input
            let _ = extract_imports(&content);
        }

        #[test]
        fn extract_imports_valid_offsets(content in "[a-zA-Z0-9@./\\-_ ]{0,1000}") {
            let imports = extract_imports(&content);
            for import in &imports {
                // Byte offsets must be within bounds
                prop_assert!(import.start_byte <= content.len());
                prop_assert!(import.end_byte <= content.len());
                prop_assert!(import.start_byte <= import.end_byte);

                // Line/column must be positive (1-indexed)
                prop_assert!(import.line >= 1);
                prop_assert!(import.column >= 1);
            }
        }

        // ===== extract_markdown_links() invariants =====

        #[test]
        fn extract_markdown_links_never_panics(content in ".*") {
            // Should never panic on any input
            let _ = extract_markdown_links(&content);
        }

        #[test]
        fn extract_markdown_links_valid_offsets(content in "[a-zA-Z0-9\\[\\]()./\\-_ ]{0,1000}") {
            let links = extract_markdown_links(&content);
            for link in &links {
                // Byte offsets must be within bounds
                prop_assert!(link.start_byte <= content.len());
                prop_assert!(link.end_byte <= content.len());
                prop_assert!(link.start_byte <= link.end_byte);

                // Line/column must be positive (1-indexed)
                prop_assert!(link.line >= 1);
                prop_assert!(link.column >= 1);
            }
        }

        // ===== Well-formed input generation =====

        #[test]
        fn xml_tags_balanced_detected(
            tag in "[a-z]+",
            content in "[a-zA-Z0-9 ]{0,100}"
        ) {
            let input = format!("<{}>{}</{}>", tag, content, tag);
            let tags = extract_xml_tags(&input);
            let errors = check_xml_balance(&tags);

            // Well-formed tags should have no balance errors
            prop_assert!(
                errors.is_empty(),
                "Well-formed XML should have no balance errors: {:?}",
                errors
            );
        }

        #[test]
        fn xml_unclosed_detected(tag in "[a-z]+") {
            let input = format!("<{}>content", tag);
            let tags = extract_xml_tags(&input);
            let errors = check_xml_balance(&tags);

            // Unclosed tag should be detected
            prop_assert!(
                errors.len() == 1,
                "Unclosed tag should be detected"
            );
        }

        #[test]
        fn import_with_extension_detected(
            dir in "[a-z]+",
            file in "[a-z]+",
            ext in "(md|txt|json|yaml)"
        ) {
            let input = format!("See @{}/{}.{} for details", dir, file, ext);
            let imports = extract_imports(&input);

            prop_assert!(
                !imports.is_empty(),
                "Import with extension should be detected"
            );
            prop_assert_eq!(
                &imports[0].path,
                &format!("{}/{}.{}", dir, file, ext)
            );
        }
    }
}
