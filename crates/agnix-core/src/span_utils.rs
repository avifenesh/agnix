//! Byte-level span-finding utilities for auto-fix helpers.
//!
//! These functions replace dynamic `Regex::new()` calls in span-finding code
//! with direct byte scanning. They are simpler, faster (no regex compilation),
//! and produce identical results for the structured patterns they replace.
//!
//! All functions operate on `&str` or `&[u8]` and return byte-offset spans
//! compatible with `Fix::replacement`.
//!
//! Note: String value parsing uses `[^"]*` semantics (no escape handling),
//! matching the behavior of the regex patterns these functions replace.
//! This is correct for agent configuration files which don't use escaped quotes.

/// Advance past ASCII whitespace (space, tab, newline, carriage return).
/// Returns the first position that is not whitespace, or `content.len()`.
/// Skip inline whitespace (space and tab only, not newlines).
/// Used when scanning within a single line.
fn skip_inline_whitespace(bytes: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    i
}

fn skip_whitespace(content: &[u8], pos: usize) -> usize {
    let mut i = pos;
    while i < content.len() && matches!(content[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum StructuredFieldFormat {
    Json,
    Yaml,
}

/// Build a formatted field insertion line for JSON/YAML objects based on local indentation.
///
/// This helper is intended for autofix generation where a field needs to be inserted before a
/// closing brace or at a known object insertion point while keeping consistent indentation.
///
/// `insertion_byte` should point at the target insertion location in `content`.
#[allow(dead_code)]
pub(crate) fn build_structured_field_insertion(
    content: &str,
    insertion_byte: usize,
    key: &str,
    value: &str,
    format: StructuredFieldFormat,
    trailing_separator: bool,
) -> Option<String> {
    if insertion_byte > content.len() || !content.is_char_boundary(insertion_byte) {
        return None;
    }

    let line_start = content[..insertion_byte]
        .rfind('\n')
        .map_or(0, |idx| idx + 1);
    let line_end = content[insertion_byte..]
        .find('\n')
        .map_or(content.len(), |idx| insertion_byte + idx);
    let current_line = &content[line_start..line_end];
    let current_indent: String = current_line
        .chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .collect();

    let previous_indent = content[..line_start]
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .collect::<String>()
        });

    let field_indent = if let Some(prev) = previous_indent {
        if prev.len() > current_indent.len() {
            prev
        } else if current_line.trim_start().starts_with('}')
            || current_line.trim_start().starts_with(']')
        {
            format!("{}{}", current_indent, detect_indent_unit(content))
        } else {
            current_indent
        }
    } else if current_line.trim_start().starts_with('}')
        || current_line.trim_start().starts_with(']')
    {
        format!("{}{}", current_indent, detect_indent_unit(content))
    } else {
        current_indent
    };

    let line = match format {
        StructuredFieldFormat::Json => {
            let comma = if trailing_separator { "," } else { "" };
            format!(r#"{field_indent}"{key}": {value}{comma}"#)
        }
        StructuredFieldFormat::Yaml => format!("{field_indent}{key}: {value}"),
    };

    Some(format!("{line}\n"))
}

fn detect_indent_unit(content: &str) -> &'static str {
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let leading: String = line
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .collect();
        if leading.is_empty() {
            continue;
        }
        if leading.starts_with('\t') {
            return "\t";
        }
        if leading.len() >= 4 && leading.len().is_multiple_of(4) {
            return "    ";
        }
        return "  ";
    }
    "  "
}

/// Find all byte positions where `"key"` appears followed by optional whitespace and `:`.
/// Returns a vec of (key_start, key_end, after_colon) tuples.
fn find_all_json_key_colon_positions(content: &str, key: &str) -> Vec<(usize, usize, usize)> {
    debug_assert!(!key.contains('"'), "key must not contain quote characters");
    let bytes = content.as_bytes();
    let mut needle = String::with_capacity(key.len() + 2);
    needle.push('"');
    needle.push_str(key);
    needle.push('"');
    let needle_bytes = needle.as_bytes();
    let mut results = Vec::new();

    let mut search_start = 0;
    while let Some(rel) = content[search_start..].find(&needle) {
        let pos = search_start + rel;
        let key_end = pos + needle_bytes.len();
        let colon_pos = skip_whitespace(bytes, key_end);
        if colon_pos < bytes.len() && bytes[colon_pos] == b':' {
            results.push((pos, key_end, colon_pos + 1));
        }
        search_start = pos + 1;
    }
    results
}

/// Parse a JSON string value starting at `pos` (which should point at the opening `"`).
/// Returns `(inner_start, inner_end, outer_end)` where inner is without quotes,
/// outer_end is the position after the closing `"`.
fn parse_string_value_at(bytes: &[u8], pos: usize) -> Option<(usize, usize, usize)> {
    if pos >= bytes.len() || bytes[pos] != b'"' {
        return None;
    }
    let inner_start = pos + 1;
    // Find the closing quote (no escape handling, matches regex `[^"]*`)
    let mut i = inner_start;
    while i < bytes.len() && bytes[i] != b'"' {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    Some((inner_start, i, i + 1))
}

/// Parse a JSON scalar value at `pos`: string, number, bool, or null.
/// Returns `(value_start, value_end)` spanning the full value (including quotes for strings).
fn parse_scalar_at(bytes: &[u8], pos: usize) -> Option<(usize, usize)> {
    if pos >= bytes.len() {
        return None;
    }
    match bytes[pos] {
        b'"' => {
            let (_, _, outer_end) = parse_string_value_at(bytes, pos)?;
            Some((pos, outer_end))
        }
        b't' => {
            if bytes.len() >= pos + 4 && &bytes[pos..pos + 4] == b"true" {
                Some((pos, pos + 4))
            } else {
                None
            }
        }
        b'f' => {
            if bytes.len() >= pos + 5 && &bytes[pos..pos + 5] == b"false" {
                Some((pos, pos + 5))
            } else {
                None
            }
        }
        b'n' => {
            if bytes.len() >= pos + 4 && &bytes[pos..pos + 4] == b"null" {
                Some((pos, pos + 4))
            } else {
                None
            }
        }
        b'-' | b'0'..=b'9' => {
            let mut i = pos;
            if bytes[i] == b'-' {
                i += 1;
                if i >= bytes.len() || !bytes[i].is_ascii_digit() {
                    return None;
                }
            }
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                let frac_start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i == frac_start {
                    return None;
                }
            }
            Some((pos, i))
        }
        _ => None,
    }
}

/// Find the first `"event"` key followed by whitespace+`:` in the content.
/// Returns the span of the quoted key including both quotes.
///
/// Replaces `find_event_key_position` in hooks/helpers.rs.
pub(crate) fn find_event_key_span(content: &str, event: &str) -> Option<(usize, usize)> {
    let positions = find_all_json_key_colon_positions(content, event);
    let (key_start, key_end, _) = positions.into_iter().next()?;
    Some((key_start, key_end))
}

/// Find `"key" : serialized_value` exactly once; return the span of the value.
///
/// Replaces `find_unique_json_key_value_span` in hooks/helpers.rs.
pub(crate) fn find_unique_json_key_value(
    content: &str,
    key: &str,
    serialized_value: &str,
) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let sv_bytes = serialized_value.as_bytes();
    let positions = find_all_json_key_colon_positions(content, key);

    let mut found: Option<(usize, usize)> = None;
    for (_, _, after_colon) in &positions {
        let val_start = skip_whitespace(bytes, *after_colon);
        let val_end = val_start + sv_bytes.len();
        if val_end <= bytes.len() && &bytes[val_start..val_end] == sv_bytes {
            if found.is_some() {
                return None;
            }
            found = Some((val_start, val_end));
        }
    }
    found
}

/// Find an entire line like `  "field": scalar_value,\n` exactly once.
/// Returns the span from leading whitespace through the trailing newline.
///
/// Matches the regex: `(?m)^[ \t]*"field"\s*:\s*(?:"[^"]*"|true|false|null|\d+(?:\.\d+)?)\s*,?\r?\n?`
///
/// Replaces `find_unique_json_field_line_span` in hooks/helpers.rs.
pub(crate) fn find_unique_json_field_line(
    content: &str,
    field_name: &str,
) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let positions = find_all_json_key_colon_positions(content, field_name);

    let mut found: Option<(usize, usize)> = None;
    for (key_start, _, after_colon) in &positions {
        let mut line_start = *key_start;
        while line_start > 0 {
            let prev = bytes[line_start - 1];
            if prev == b' ' || prev == b'\t' {
                line_start -= 1;
            } else {
                break;
            }
        }
        if line_start > 0 && bytes[line_start - 1] != b'\n' {
            continue;
        }

        let val_start = skip_whitespace(bytes, *after_colon);
        // Original regex didn't support negative numbers (no -?), reject them
        if val_start < bytes.len() && bytes[val_start] == b'-' {
            continue;
        }
        let (_, val_end) = match parse_scalar_at(bytes, val_start) {
            Some(span) => span,
            None => continue,
        };

        let mut end = val_end;
        end = skip_inline_whitespace(bytes, end);
        if end < bytes.len() && bytes[end] == b',' {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'\r' {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'\n' {
            end += 1;
        }

        if found.is_some() {
            return None;
        }
        found = Some((line_start, end));
    }
    found
}

/// Find a line matching `  "matcher": "value",\n` exactly once.
/// Returns the full line span from leading whitespace through trailing newline.
///
/// Matches the regex: `(?m)^[ \t]*"matcher"\s*:\s*"value"\s*,?\r?\n?`
///
/// Replaces `find_unique_matcher_line_span` in hooks/helpers.rs.
pub(crate) fn find_unique_json_matcher_line(
    content: &str,
    matcher_value: &str,
) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let positions = find_all_json_key_colon_positions(content, "matcher");

    let mut found: Option<(usize, usize)> = None;
    for (key_start, _, after_colon) in &positions {
        let mut line_start = *key_start;
        while line_start > 0 {
            let prev = bytes[line_start - 1];
            if prev == b' ' || prev == b'\t' {
                line_start -= 1;
            } else {
                break;
            }
        }
        if line_start > 0 && bytes[line_start - 1] != b'\n' {
            continue;
        }

        let val_start = skip_whitespace(bytes, *after_colon);
        let (inner_start, inner_end, outer_end) = match parse_string_value_at(bytes, val_start) {
            Some(v) => v,
            None => continue,
        };

        if &bytes[inner_start..inner_end] != matcher_value.as_bytes() {
            continue;
        }

        let mut end = outer_end;
        end = skip_inline_whitespace(bytes, end);
        if end < bytes.len() && bytes[end] == b',' {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'\r' {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'\n' {
            end += 1;
        }

        if found.is_some() {
            return None;
        }
        found = Some((line_start, end));
    }
    found
}

/// Find `"key" : "inner_value"` exactly once and return the span of the inner value
/// (without quotes).
///
/// Replaces `find_unique_json_string_value_span` in rules/mod.rs.
pub(crate) fn find_unique_json_string_inner(
    content: &str,
    key: &str,
    inner_value: &str,
) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let positions = find_all_json_key_colon_positions(content, key);

    let mut found: Option<(usize, usize)> = None;
    for (_, _, after_colon) in &positions {
        let val_start = skip_whitespace(bytes, *after_colon);
        let (inner_start, inner_end, _) = match parse_string_value_at(bytes, val_start) {
            Some(v) => v,
            None => continue,
        };

        if &bytes[inner_start..inner_end] != inner_value.as_bytes() {
            continue;
        }

        if found.is_some() {
            return None;
        }
        found = Some((inner_start, inner_end));
    }
    found
}

/// Find a TOML `key = "value"` at the start of a line (allowing leading whitespace).
/// Returns the span of the inner string value (without quotes).
/// Enforces uniqueness: returns `None` if 0 or 2+ matches.
///
/// Matches the regex: `(?:^|\n)\s*key\s*=\s*"(value)"`
///
/// Replaces `find_toml_string_value_span` in rules/codex.rs.
pub(crate) fn find_unique_toml_string_value(
    content: &str,
    key: &str,
    value: &str,
) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let key_bytes = key.as_bytes();
    let value_bytes = value.as_bytes();

    let mut found: Option<(usize, usize)> = None;
    // Scan for key at line starts
    let mut pos = 0;
    while pos < bytes.len() {
        // At pos, we should be at the start of a line or at position 0
        // Skip leading whitespace
        let key_area = skip_inline_whitespace(bytes, pos);
        // Check if the key matches here
        if key_area + key_bytes.len() <= bytes.len()
            && &bytes[key_area..key_area + key_bytes.len()] == key_bytes
        {
            let after_key = key_area + key_bytes.len();
            // Skip whitespace, expect '='
            let eq_pos = skip_inline_whitespace(bytes, after_key);
            if eq_pos < bytes.len() && bytes[eq_pos] == b'=' {
                // Skip whitespace after '='
                let val_pos = skip_inline_whitespace(bytes, eq_pos + 1);
                // Expect '"value"'
                if let Some((inner_start, inner_end, _)) = parse_string_value_at(bytes, val_pos) {
                    if &bytes[inner_start..inner_end] == value_bytes {
                        if found.is_some() {
                            return None;
                        }
                        found = Some((inner_start, inner_end));
                    }
                }
            }
        }

        // Advance to next line
        match content[pos..].find('\n') {
            Some(nl) => pos = pos + nl + 1,
            None => break,
        }
    }
    found
}

/// Find `"key" : "<captured>"` exactly once. Returns the inner string span
/// plus the captured string content.
///
/// Replaces `find_unique_json_string_value_range` in rules/plugin.rs.
pub(crate) fn find_unique_json_string_value_range(
    content: &str,
    key: &str,
) -> Option<(usize, usize, String)> {
    let bytes = content.as_bytes();
    let positions = find_all_json_key_colon_positions(content, key);

    let mut found: Option<(usize, usize, String)> = None;
    for (_, _, after_colon) in &positions {
        let val_start = skip_whitespace(bytes, *after_colon);
        let (inner_start, inner_end, _) = match parse_string_value_at(bytes, val_start) {
            Some(v) => v,
            None => continue,
        };

        if found.is_some() {
            return None;
        }
        debug_assert!(
            content.is_char_boundary(inner_start) && content.is_char_boundary(inner_end),
            "span_utils: computed span is not on a UTF-8 char boundary"
        );
        let captured = content[inner_start..inner_end].to_string();
        found = Some((inner_start, inner_end, captured));
    }
    found
}

/// Find `"key" : <scalar>` exactly once. Returns the span of the full scalar value
/// (including quotes for strings).
///
/// Scalar types: `"string"`, `-?digits(.digits)?`, `true`, `false`, `null`.
///
/// Replaces `find_unique_json_scalar_value_span` in rules/mcp.rs.
pub(crate) fn find_unique_json_scalar_span(content: &str, key: &str) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let positions = find_all_json_key_colon_positions(content, key);

    let mut found: Option<(usize, usize)> = None;
    for (_, _, after_colon) in &positions {
        let val_start = skip_whitespace(bytes, *after_colon);
        let (scalar_start, scalar_end) = match parse_scalar_at(bytes, val_start) {
            Some(span) => span,
            None => continue,
        };

        if found.is_some() {
            return None;
        }
        found = Some((scalar_start, scalar_end));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== skip_whitespace =====

    #[test]
    fn skip_whitespace_basic() {
        assert_eq!(skip_whitespace(b"  hello", 0), 2);
        assert_eq!(skip_whitespace(b"\t\n\r x", 0), 4);
        assert_eq!(skip_whitespace(b"hello", 0), 0);
        assert_eq!(skip_whitespace(b"   ", 0), 3);
        assert_eq!(skip_whitespace(b"", 0), 0);
    }

    #[test]
    fn skip_whitespace_from_offset() {
        assert_eq!(skip_whitespace(b"ab  cd", 2), 4);
    }

    // ===== structured field insertion helper =====

    #[test]
    fn structured_json_insertion_respects_indent_before_closing_brace() {
        let content = "{\n  \"type\": \"stdio\",\n  \"command\": \"node\"\n}\n";
        let insert_at = content.find("}\n").unwrap();
        let insertion = build_structured_field_insertion(
            content,
            insert_at,
            "timeout",
            "30",
            StructuredFieldFormat::Json,
            true,
        )
        .unwrap();

        assert_eq!(insertion, "  \"timeout\": 30,\n");
    }

    #[test]
    fn structured_json_insertion_without_trailing_separator_omits_comma() {
        let content = "{\n  \"type\": \"stdio\"\n}\n";
        let insert_at = content.find("}\n").unwrap();
        let insertion = build_structured_field_insertion(
            content,
            insert_at,
            "timeout",
            "30",
            StructuredFieldFormat::Json,
            false,
        )
        .unwrap();

        assert_eq!(insertion, "  \"timeout\": 30\n");
    }

    #[test]
    fn structured_yaml_insertion_reuses_nested_indent() {
        let content = "mcp:\n  server:\n    command: node\n";
        let insert_at = content.len();
        let insertion = build_structured_field_insertion(
            content,
            insert_at,
            "timeout",
            "30",
            StructuredFieldFormat::Yaml,
            false,
        )
        .unwrap();

        assert_eq!(insertion, "    timeout: 30\n");
    }

    // ===== find_event_key_span =====

    #[test]
    fn event_key_span_found() {
        let content = r#"{"hooks": {"PreToolExecution": []}}"#;
        let result = find_event_key_span(content, "PreToolExecution");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        assert_eq!(&content[start..end], "\"PreToolExecution\"");
    }

    #[test]
    fn event_key_span_not_found() {
        let content = r#"{"hooks": {"Stop": []}}"#;
        assert!(find_event_key_span(content, "NotPresent").is_none());
    }

    #[test]
    fn event_key_span_key_without_colon() {
        // "Foo" appears as a value, not a key -- should not match
        let content = r#"{"name": "Foo", "bar": 1}"#;
        assert!(find_event_key_span(content, "Foo").is_none());
    }

    #[test]
    fn event_key_span_whitespace_before_colon() {
        let content = "{ \"Stop\"  \t : [] }";
        let result = find_event_key_span(content, "Stop");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "\"Stop\"");
    }

    // ===== find_unique_json_key_value =====

    #[test]
    fn unique_key_value_found() {
        let content = r#"{"type": "command", "cmd": "echo"}"#;
        let result = find_unique_json_key_value(content, "type", "\"command\"");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "\"command\"");
    }

    #[test]
    fn unique_key_value_not_unique() {
        let content = r#"{"type": "command", "other": {"type": "command"}}"#;
        assert!(find_unique_json_key_value(content, "type", "\"command\"").is_none());
    }

    #[test]
    fn unique_key_value_not_found() {
        let content = r#"{"type": "prompt"}"#;
        assert!(find_unique_json_key_value(content, "type", "\"command\"").is_none());
    }

    #[test]
    fn unique_key_value_with_whitespace() {
        let content = "{ \"type\" : \"command\" }";
        let result = find_unique_json_key_value(content, "type", "\"command\"");
        assert!(result.is_some());
    }

    // ===== find_unique_json_field_line =====

    #[test]
    fn field_line_unique() {
        let content = "{\n  \"type\": \"prompt\",\n  \"async\": true,\n  \"prompt\": \"hello\"\n}";
        let result = find_unique_json_field_line(content, "async");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        let matched = &content[s..e];
        assert!(matched.contains("\"async\""));
        assert!(matched.contains("true"));
    }

    #[test]
    fn field_line_duplicate_returns_none() {
        let content = "{\n  \"async\": true,\n  \"other\": 1,\n  \"async\": false\n}";
        assert!(find_unique_json_field_line(content, "async").is_none());
    }

    #[test]
    fn field_line_missing() {
        let content = r#"{ "type": "command", "command": "echo hi" }"#;
        assert!(find_unique_json_field_line(content, "async").is_none());
    }

    #[test]
    fn field_line_string_value() {
        let content = "{\n  \"model\": \"fast\",\n  \"other\": 1\n}";
        let result = find_unique_json_field_line(content, "model");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        let matched = &content[s..e];
        assert!(matched.contains("\"model\""));
        assert!(matched.contains("\"fast\""));
    }

    #[test]
    fn field_line_null_value() {
        let content = "{\n  \"field\": null\n}";
        let result = find_unique_json_field_line(content, "field");
        assert!(result.is_some());
    }

    #[test]
    fn field_line_number_value() {
        let content = "{\n  \"count\": 42\n}";
        let result = find_unique_json_field_line(content, "count");
        assert!(result.is_some());
    }

    #[test]
    fn field_line_float_value() {
        let content = "{\n  \"score\": 3.14\n}";
        let result = find_unique_json_field_line(content, "score");
        assert!(result.is_some());
    }

    #[test]
    fn field_line_with_trailing_comma() {
        let content = "{\n  \"async\": true,\n  \"next\": 1\n}";
        let result = find_unique_json_field_line(content, "async");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        let matched = &content[s..e];
        // Should include the comma
        assert!(matched.ends_with('\n') || matched.ends_with(','));
    }

    #[test]
    fn field_line_crlf() {
        let content = "{\r\n  \"async\": true,\r\n  \"other\": 1\r\n}";
        let result = find_unique_json_field_line(content, "async");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        let matched = &content[s..e];
        assert!(matched.contains("\"async\""));
    }

    #[test]
    fn field_line_not_at_line_start() {
        // "async" appears inline, not on its own line from column 0
        let content = r#"{ "foo": 1, "async": true, "bar": 2 }"#;
        // The key is NOT at the start of a line (preceded by more than just spaces/tabs)
        assert!(find_unique_json_field_line(content, "async").is_none());
    }

    // ===== find_unique_json_matcher_line =====

    #[test]
    fn matcher_line_found() {
        let content = "{\n  \"matcher\": \"Bash\",\n  \"hooks\": []\n}";
        let result = find_unique_json_matcher_line(content, "Bash");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert!(content[s..e].contains("\"matcher\""));
        assert!(content[s..e].contains("\"Bash\""));
    }

    #[test]
    fn matcher_line_not_found() {
        let content = "{\n  \"matcher\": \"Bash\"\n}";
        assert!(find_unique_json_matcher_line(content, "Write").is_none());
    }

    #[test]
    fn matcher_line_duplicate() {
        let content = "{\n  \"matcher\": \"Bash\",\n  \"other\": 1,\n  \"matcher\": \"Bash\"\n}";
        assert!(find_unique_json_matcher_line(content, "Bash").is_none());
    }

    #[test]
    fn matcher_line_crlf() {
        let content = "{\r\n  \"matcher\": \"Bash\",\r\n  \"hooks\": []\r\n}";
        let result = find_unique_json_matcher_line(content, "Bash");
        assert!(result.is_some());
    }

    // ===== find_unique_json_string_inner =====

    #[test]
    fn string_inner_found() {
        let content = r#"{"type": "command"}"#;
        let result = find_unique_json_string_inner(content, "type", "command");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "command");
    }

    #[test]
    fn string_inner_not_unique() {
        let content = r#"{"type": "command", "a": {"type": "command"}}"#;
        assert!(find_unique_json_string_inner(content, "type", "command").is_none());
    }

    #[test]
    fn string_inner_wrong_value() {
        let content = r#"{"type": "prompt"}"#;
        assert!(find_unique_json_string_inner(content, "type", "command").is_none());
    }

    #[test]
    fn string_inner_with_whitespace() {
        let content = "{ \"type\" :  \"command\" }";
        let result = find_unique_json_string_inner(content, "type", "command");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "command");
    }

    // ===== find_unique_toml_string_value =====

    #[test]
    fn toml_value_found() {
        let content = "approvalMode = \"suggest\"\n";
        let result = find_unique_toml_string_value(content, "approvalMode", "suggest");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "suggest");
    }

    #[test]
    fn toml_value_with_spaces() {
        let content = "  approvalMode  =  \"suggest\" \n";
        let result = find_unique_toml_string_value(content, "approvalMode", "suggest");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "suggest");
    }

    #[test]
    fn toml_value_not_found() {
        let content = "approvalMode = \"full-auto\"\n";
        assert!(find_unique_toml_string_value(content, "approvalMode", "suggest").is_none());
    }

    #[test]
    fn toml_value_not_unique() {
        let content = "approvalMode = \"suggest\"\napprovalMode = \"suggest\"\n";
        assert!(find_unique_toml_string_value(content, "approvalMode", "suggest").is_none());
    }

    #[test]
    fn toml_value_on_second_line() {
        let content = "other = \"foo\"\napprovalMode = \"suggest\"\n";
        let result = find_unique_toml_string_value(content, "approvalMode", "suggest");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "suggest");
    }

    // ===== find_unique_json_string_value_range =====

    #[test]
    fn string_value_range_found() {
        let content = r#"{"name": "my-plugin"}"#;
        let result = find_unique_json_string_value_range(content, "name");
        assert!(result.is_some());
        let (s, e, captured) = result.unwrap();
        assert_eq!(&content[s..e], "my-plugin");
        assert_eq!(captured, "my-plugin");
    }

    #[test]
    fn string_value_range_empty_value() {
        let content = r#"{"name": ""}"#;
        let result = find_unique_json_string_value_range(content, "name");
        assert!(result.is_some());
        let (s, e, captured) = result.unwrap();
        assert_eq!(s, e); // empty span
        assert_eq!(captured, "");
    }

    #[test]
    fn string_value_range_not_unique() {
        let content = r#"{"name": "a", "sub": {"name": "b"}}"#;
        assert!(find_unique_json_string_value_range(content, "name").is_none());
    }

    #[test]
    fn string_value_range_no_string_value() {
        let content = r#"{"count": 42}"#;
        // Key "count" exists but value is not a string
        assert!(find_unique_json_string_value_range(content, "count").is_none());
    }

    // ===== find_unique_json_scalar_span =====

    #[test]
    fn scalar_span_string() {
        let content = r#"{"jsonrpc": "2.0"}"#;
        let result = find_unique_json_scalar_span(content, "jsonrpc");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "\"2.0\"");
    }

    #[test]
    fn scalar_span_number() {
        let content = r#"{"count": 42}"#;
        let result = find_unique_json_scalar_span(content, "count");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "42");
    }

    #[test]
    fn scalar_span_negative_number() {
        let content = r#"{"offset": -10}"#;
        let result = find_unique_json_scalar_span(content, "offset");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "-10");
    }

    #[test]
    fn scalar_span_float() {
        let content = r#"{"score": 3.14}"#;
        let result = find_unique_json_scalar_span(content, "score");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "3.14");
    }

    #[test]
    fn scalar_span_bool() {
        let content = r#"{"enabled": true}"#;
        let result = find_unique_json_scalar_span(content, "enabled");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "true");
    }

    #[test]
    fn scalar_span_null() {
        let content = r#"{"value": null}"#;
        let result = find_unique_json_scalar_span(content, "value");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "null");
    }

    #[test]
    fn scalar_span_not_unique() {
        let content = r#"{"a": "x", "b": {"a": "y"}}"#;
        assert!(find_unique_json_scalar_span(content, "a").is_none());
    }

    // ===== Edge cases =====

    #[test]
    fn empty_content() {
        assert!(find_event_key_span("", "key").is_none());
        assert!(find_unique_json_key_value("", "k", "v").is_none());
        assert!(find_unique_json_field_line("", "f").is_none());
        assert!(find_unique_json_matcher_line("", "v").is_none());
        assert!(find_unique_json_string_inner("", "k", "v").is_none());
        assert!(find_unique_toml_string_value("", "k", "v").is_none());
        assert!(find_unique_json_string_value_range("", "k").is_none());
        assert!(find_unique_json_scalar_span("", "k").is_none());
    }

    #[test]
    fn key_at_start_of_content() {
        let content = "\"key\": 42";
        let result = find_unique_json_scalar_span(content, "key");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "42");
    }

    #[test]
    fn key_at_end_of_content() {
        // Key exists but no value follows
        let content = "\"key\":";
        assert!(find_unique_json_scalar_span(content, "key").is_none());
    }

    #[test]
    fn special_chars_in_key() {
        let content = r#"{"my-key.name": "value"}"#;
        let result = find_unique_json_string_inner(content, "my-key.name", "value");
        assert!(result.is_some());
    }

    #[test]
    fn nested_json_keys() {
        // Only one "inner" key with value "hello"
        let content = r#"{"outer": {"inner": "hello"}}"#;
        let result = find_unique_json_string_inner(content, "inner", "hello");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "hello");
    }

    #[test]
    fn field_line_at_first_line() {
        let content = "\"async\": true\n\"other\": 1\n";
        let result = find_unique_json_field_line(content, "async");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "\"async\": true\n");
    }

    #[test]
    fn toml_at_start_of_file() {
        let content = "key = \"val\"";
        let result = find_unique_toml_string_value(content, "key", "val");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "val");
    }

    #[test]
    fn parse_scalar_negative_float() {
        let content = r#"{"x": -1.5}"#;
        let result = find_unique_json_scalar_span(content, "x");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "-1.5");
    }

    #[test]
    fn field_line_inline_does_not_match() {
        // On a single line with other content before the key
        let content = r#"{"foo": 1, "async": true, "bar": 2}"#;
        // "async" is NOT at the start of its line (preceded by other JSON chars)
        assert!(find_unique_json_field_line(content, "async").is_none());
    }

    #[test]
    fn matcher_line_inline_does_not_match() {
        let content = r#"{"x": 1, "matcher": "Bash", "y": 2}"#;
        assert!(find_unique_json_matcher_line(content, "Bash").is_none());
    }

    // ===== UTF-8 / Unicode tests =====

    #[test]
    fn unicode_key_and_value() {
        // cl\u{00e9} = "cle" with e-acute (multi-byte UTF-8)
        let content = "{\"cl\u{00e9}\": \"valeur\"}";
        let result = find_unique_json_string_inner(content, "cl\u{00e9}", "valeur");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "valeur");
        assert!(content.is_char_boundary(s));
        assert!(content.is_char_boundary(e));
    }

    #[test]
    fn emoji_key_and_value() {
        // Build content with actual emoji characters (4-byte UTF-8)
        let content = format!("{{\"{}\":\"{}\"}}", "\u{1f525}", "\u{1f680}");
        let result = find_unique_json_string_inner(&content, "\u{1f525}", "\u{1f680}");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "\u{1f680}");
        assert!(content.is_char_boundary(s));
        assert!(content.is_char_boundary(e));
    }

    #[test]
    fn multibyte_in_scalar_span() {
        let content = "{\"name\": \"\u{65e5}\u{672c}\u{8a9e}\"}";
        let result = find_unique_json_scalar_span(content, "name");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert!(content.is_char_boundary(s));
        assert!(content.is_char_boundary(e));
        // Scalar span includes the quotes for string values
        assert_eq!(&content[s..e], "\"\u{65e5}\u{672c}\u{8a9e}\"");
    }

    #[test]
    fn unicode_in_string_value_range() {
        let content = "{\"path\": \"C:\\\\r\u{00e9}sum\u{00e9}\\\\file.txt\"}";
        let result = find_unique_json_string_value_range(content, "path");
        assert!(result.is_some());
        let (s, e, captured) = result.unwrap();
        assert!(content.is_char_boundary(s));
        assert!(content.is_char_boundary(e));
        assert_eq!(&content[s..e], &captured);
    }

    // ===== Additional CRLF tests =====

    #[test]
    fn event_key_span_crlf() {
        let content = "{\r\n  \"beforeCommand\": {\r\n    \"command\": \"echo\"\r\n  }\r\n}";
        let result = find_event_key_span(content, "beforeCommand");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "\"beforeCommand\"");
    }

    #[test]
    fn json_key_value_crlf() {
        let content = "{\r\n  \"timeout\": 30\r\n}";
        let result = find_unique_json_key_value(content, "timeout", "30");
        assert!(result.is_some());
    }

    #[test]
    fn string_inner_crlf() {
        let content = "{\r\n  \"type\": \"command\"\r\n}";
        let result = find_unique_json_string_inner(content, "type", "command");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert_eq!(&content[s..e], "command");
    }

    // ===== Position boundary tests =====

    #[test]
    fn field_line_at_position_zero() {
        // Key at the very start of content (position 0)
        let content = "\"field\": true\n";
        let result = find_unique_json_field_line(content, "field");
        assert!(result.is_some());
        let (s, _e) = result.unwrap();
        assert_eq!(s, 0, "Should start at position 0");
    }

    #[test]
    fn key_value_at_end_of_content() {
        // Value extends to the very end of content (content.len())
        let content = "\"key\": \"val\"";
        let result = find_unique_json_string_inner(content, "key", "val");
        assert!(result.is_some());
        let (_s, e) = result.unwrap();
        assert_eq!(
            e,
            content.len() - 1,
            "Inner end should be at last quote pos"
        );
    }

    #[test]
    fn skip_whitespace_at_content_len() {
        // Calling skip_whitespace at exactly content.len() should return content.len()
        let content = b"hello";
        assert_eq!(skip_whitespace(content, content.len()), content.len());
    }

    #[test]
    fn skip_whitespace_beyond_content_len() {
        // When pos > len, the while-loop condition (i < content.len()) is immediately false,
        // so the function returns pos unchanged. Callers are responsible for providing
        // valid positions; this test documents the current no-panic guarantee.
        let content = b"hello";
        assert_eq!(
            skip_whitespace(content, content.len() + 10),
            content.len() + 10
        );
    }

    // ===== Unicode multi-byte boundary tests =====

    #[test]
    fn unicode_multibyte_key_boundaries_are_valid() {
        // 3-byte UTF-8 chars in key and value
        let content = "{\"k\u{00e9}y\": \"v\u{00e0}l\"}";
        let result = find_unique_json_string_inner(content, "k\u{00e9}y", "v\u{00e0}l");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert!(content.is_char_boundary(s), "Start must be a char boundary");
        assert!(content.is_char_boundary(e), "End must be a char boundary");
        assert_eq!(&content[s..e], "v\u{00e0}l");
    }

    #[test]
    fn four_byte_unicode_scalar_span_boundaries() {
        // 4-byte emoji chars in value
        let content = "{\"tag\": \"\u{1f600}\u{1f601}\u{1f602}\"}";
        let result = find_unique_json_scalar_span(content, "tag");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert!(content.is_char_boundary(s));
        assert!(content.is_char_boundary(e));
        // Should include the surrounding quotes
        assert_eq!(&content[s..e], "\"\u{1f600}\u{1f601}\u{1f602}\"");
    }

    #[test]
    fn toml_with_unicode_value_boundaries() {
        let content = "name = \"caf\u{00e9}\"\n";
        let result = find_unique_toml_string_value(content, "name", "caf\u{00e9}");
        assert!(result.is_some());
        let (s, e) = result.unwrap();
        assert!(content.is_char_boundary(s));
        assert!(content.is_char_boundary(e));
        assert_eq!(&content[s..e], "caf\u{00e9}");
    }
}
