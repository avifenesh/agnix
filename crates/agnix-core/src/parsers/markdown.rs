//! Markdown parser for extracting @imports and checking XML tags

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

/// Check if XML tags are balanced
pub fn check_xml_balance(tags: &[XmlTag]) -> Vec<XmlBalanceError> {
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

    // Unclosed tags
    for tag in stack {
        errors.push(XmlBalanceError::Unclosed {
            tag: tag.name.clone(),
            line: tag.line,
            column: tag.column,
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
}
