use crate::fs::FileSystem;
use crate::parsers::frontmatter::FrontmatterParts;
use std::collections::HashSet;
use std::path::Path;

use super::{
    PathMatch, SkillFrontmatter, reference_path_regex, windows_path_regex, windows_path_token_regex,
};

pub(super) fn parse_frontmatter_fields(
    frontmatter: &str,
) -> Result<SkillFrontmatter, serde_yaml::Error> {
    if frontmatter.trim().is_empty() {
        return Ok(SkillFrontmatter::default());
    }
    serde_yaml::from_str(frontmatter)
}

pub(super) fn extract_reference_paths(body: &str) -> Vec<PathMatch> {
    let re = reference_path_regex();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for m in re.find_iter(body) {
        #[allow(clippy::collapsible_if)]
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    paths
}

/// Check if a string looks like a regex escape sequence rather than a Windows path
pub(super) fn is_regex_escape(s: &str) -> bool {
    // Common regex metacharacter escapes that aren't Windows paths
    // \n \s \d \w \t \r \b \| \. \/ \$ \^ \+ \* \? \{ \} \[ \] \( \)
    static REGEX_ESCAPE_CHARS: &[char] = &[
        'n', 's', 'd', 'w', 't', 'r', 'b', '|', '.', '/', '$', '^', '+', '*', '?', '{', '}', '[',
        ']', '(', ')', 'S', 'D', 'W', 'B', // PCRE-specific assertions and features
        'K', 'A', 'Z', 'z', 'G', 'p', 'P', 'x', 'X',
    ];

    // Check if this looks like a regex pattern (contains common regex escapes)
    let parts: Vec<&str> = s.split('\\').collect();
    if parts.len() < 2 {
        return false;
    }

    // If most backslash-prefixed parts start with regex metacharacters, it's likely a regex
    let regex_like_count = parts[1..]
        .iter()
        .filter(|part| {
            part.chars()
                .next()
                .map(|c| REGEX_ESCAPE_CHARS.contains(&c))
                .unwrap_or(false)
        })
        .count();

    // If more than half of the backslash sequences look like regex escapes, skip it
    regex_like_count > 0 && regex_like_count >= (parts.len() - 1) / 2
}

pub(super) fn extract_windows_paths(body: &str) -> Vec<PathMatch> {
    let re = windows_path_regex();
    let token_re = windows_path_token_regex();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for m in re.find_iter(body) {
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            // Skip regex escape sequences
            if is_regex_escape(&trimmed) {
                continue;
            }
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    for m in token_re.find_iter(body) {
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            // Skip regex escape sequences
            if is_regex_escape(&trimmed) {
                continue;
            }
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    paths
}

pub(super) fn reference_path_too_deep(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut parts = normalized.split('/').filter(|part| !part.is_empty());
    let Some(prefix) = parts.next() else {
        return false;
    };

    // Check for file references (references/, reference/, refs/)
    if !prefix.eq_ignore_ascii_case("references")
        && !prefix.eq_ignore_ascii_case("reference")
        && !prefix.eq_ignore_ascii_case("refs")
    {
        return false;
    }

    // Exclude git refs - they're not file references
    // Git refs look like: refs/remotes/..., refs/heads/..., refs/tags/...
    if prefix.eq_ignore_ascii_case("refs") {
        if let Some(second) = parts.next() {
            if second.eq_ignore_ascii_case("remotes")
                || second.eq_ignore_ascii_case("heads")
                || second.eq_ignore_ascii_case("tags")
                || second.eq_ignore_ascii_case("stash")
            {
                return false; // This is a git ref, not a file reference
            }
        }
        // Reset iterator for depth check
        let parts = normalized.split('/').filter(|part| !part.is_empty());
        return parts.skip(1).count() > 1;
    }

    parts.count() > 1
}

pub(super) fn trim_path_token(token: &str) -> &str {
    token
        .trim_start_matches(['(', '[', '{', '<', '"', '\''])
        .trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '>', '"', '\''])
}

pub(super) fn trim_path_token_with_offset(token: &str) -> Option<(String, usize)> {
    let trimmed = trim_path_token(token);
    if trimmed.is_empty() {
        return None;
    }
    let offset = token.find(trimmed).unwrap_or(0);
    Some((trimmed.to_string(), offset))
}

pub(super) fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

pub(super) fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
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
    (low + 1, offset.saturating_sub(line_start) + 1)
}

pub(super) fn frontmatter_key_line_col(
    parts: &FrontmatterParts,
    key: &str,
    line_starts: &[usize],
) -> (usize, usize) {
    let offset = frontmatter_key_offset(&parts.frontmatter, key)
        .map(|local| parts.frontmatter_start + local)
        .unwrap_or(parts.frontmatter_start);
    line_col_at(offset, line_starts)
}

pub(super) fn frontmatter_key_offset(frontmatter: &str, key: &str) -> Option<usize> {
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            // Calculate actual byte length including newline characters
            let line_end = offset + line.len();
            // Check for CRLF or LF
            if line_end < bytes.len() {
                if bytes[line_end] == b'\n' {
                    offset = line_end + 1; // LF
                } else if line_end + 1 < bytes.len()
                    && bytes[line_end] == b'\r'
                    && bytes[line_end + 1] == b'\n'
                {
                    offset = line_end + 2; // CRLF
                } else {
                    offset = line_end; // No newline (last line)
                }
            } else {
                offset = line_end; // End of string
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(key) {
            if rest.trim_start().starts_with(':') {
                let column = line.len() - trimmed.len();
                return Some(offset + column);
            }
        }
        // Calculate actual byte length including newline characters
        let line_end = offset + line.len();
        if line_end < bytes.len() {
            if bytes[line_end] == b'\n' {
                offset = line_end + 1; // LF
            } else if line_end + 1 < bytes.len()
                && bytes[line_end] == b'\r'
                && bytes[line_end + 1] == b'\n'
            {
                offset = line_end + 2; // CRLF
            } else {
                offset = line_end; // No newline (last line)
            }
        } else {
            offset = line_end; // End of string
        }
    }
    None
}

/// Find the byte range of a YAML value for a given key in frontmatter.
/// Returns (start, end) byte offsets relative to the full content.
/// Handles both quoted and unquoted values.
pub(super) fn frontmatter_value_byte_range(
    _content: &str,
    parts: &FrontmatterParts,
    key: &str,
) -> Option<(usize, usize)> {
    let frontmatter = &parts.frontmatter;
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            // Calculate actual byte length including newline characters
            let line_end = offset + line.len();
            if line_end < bytes.len() {
                if bytes[line_end] == b'\n' {
                    offset = line_end + 1; // LF
                } else if line_end + 1 < bytes.len()
                    && bytes[line_end] == b'\r'
                    && bytes[line_end + 1] == b'\n'
                {
                    offset = line_end + 2; // CRLF
                } else {
                    offset = line_end; // No newline (last line)
                }
            } else {
                offset = line_end; // End of string
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix(key) {
            if let Some(after_colon) = rest.trim_start().strip_prefix(':') {
                // Found the key, now find the value
                let leading_ws = line.len() - trimmed.len();
                let ws_after_key = rest.len() - rest.trim_start().len();
                let key_end = leading_ws + key.len() + ws_after_key + 1; // +1 for ':'

                let value_str = after_colon.trim_start();
                if value_str.is_empty() {
                    // No value on this line (might be multiline YAML)
                    return None;
                }

                // Calculate value start position in line
                let value_offset_in_line = key_end + (after_colon.len() - value_str.len());

                // Handle quoted values
                let (value_start, value_len) = if let Some(inner) = value_str.strip_prefix('"') {
                    // Double-quoted: find closing quote
                    if let Some(end_quote) = inner.find('"') {
                        (value_offset_in_line + 1, end_quote) // Skip opening quote
                    } else {
                        // Unclosed quote - return None for malformed YAML
                        return None;
                    }
                } else if let Some(inner) = value_str.strip_prefix('\'') {
                    // Single-quoted: find closing quote
                    if let Some(end_quote) = inner.find('\'') {
                        (value_offset_in_line + 1, end_quote) // Skip opening quote
                    } else {
                        // Unclosed quote - return None for malformed YAML
                        return None;
                    }
                } else {
                    // Unquoted value: take until end of line or comment
                    // Check for both " #" (space-hash) and "\t#" (tab-hash)
                    let value_end = value_str
                        .find(" #")
                        .or_else(|| value_str.find("\t#"))
                        .unwrap_or(value_str.len());
                    (value_offset_in_line, value_end)
                };

                let abs_start = parts.frontmatter_start + offset + value_start;
                let abs_end = abs_start + value_len;

                return Some((abs_start, abs_end));
            }
        }
        // Calculate actual byte length including newline characters
        let line_end = offset + line.len();
        if line_end < bytes.len() {
            if bytes[line_end] == b'\n' {
                offset = line_end + 1; // LF
            } else if line_end + 1 < bytes.len()
                && bytes[line_end] == b'\r'
                && bytes[line_end + 1] == b'\n'
            {
                offset = line_end + 2; // CRLF
            } else {
                offset = line_end; // No newline (last line)
            }
        } else {
            offset = line_end; // End of string
        }
    }
    None
}

/// Find the full line byte range for a frontmatter key.
/// Returns (line_start, line_end_exclusive) in full content byte offsets.
/// Includes the trailing '\n' when present.
pub(super) fn frontmatter_key_line_byte_range(
    content: &str,
    parts: &FrontmatterParts,
    key: &str,
) -> Option<(usize, usize)> {
    let local_key_start = frontmatter_key_offset(&parts.frontmatter, key)?;
    let abs_start = parts.frontmatter_start + local_key_start;
    if abs_start >= content.len() {
        return None;
    }

    let bytes = content.as_bytes();
    let mut end = abs_start;
    while end < content.len() && bytes[end] != b'\n' {
        end += 1;
    }
    if end < content.len() {
        end += 1;
    }

    Some((abs_start, end))
}

pub(super) fn directory_size_until(path: &Path, max_bytes: u64, fs: &dyn FileSystem) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs.read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries {
            if entry.metadata.is_symlink {
                continue;
            }
            if entry.metadata.is_dir {
                stack.push(entry.path.clone());
                continue;
            }
            if entry.metadata.is_file {
                total = total.saturating_add(entry.metadata.len);
                if total > max_bytes {
                    return total;
                }
            }
        }
    }
    total
}
