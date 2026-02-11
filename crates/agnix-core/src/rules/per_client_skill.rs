//! Per-client skill validation
//!
//! Detects unsupported frontmatter fields in SKILL.md files based on the
//! client directory they reside in. For example, a skill in `.cursor/skills/`
//! should not use fields that Cursor does not support.

use crate::config::LintConfig;
use crate::diagnostics::{Diagnostic, Fix};
use crate::parsers::frontmatter::split_frontmatter;
use crate::rules::{Validator, ValidatorMetadata};
use rust_i18n::t;
use std::path::Path;

/// Known clients that host SKILL.md files.
///
/// The `Amp` variant is defined for completeness (AMP-SK-001 exists in rules.json)
/// but `.agents/` paths are mapped to `Codex` since both clients share that directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SkillClient {
    ClaudeCode,
    Cursor,
    Cline,
    Copilot,
    Codex,
    OpenCode,
    Windsurf,
    Kiro,
    Amp,
    RooCode,
    Unknown,
}

/// Universal frontmatter fields supported by the Agent Skills specification.
/// These are safe to use in any client.
const UNIVERSAL_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// Check whether a client supports a given frontmatter field.
///
/// Claude Code supports all extension fields. Cursor additionally supports
/// `disable-model-invocation`. All other clients only support universal fields.
fn is_field_supported(client: SkillClient, field: &str) -> bool {
    if UNIVERSAL_FIELDS.contains(&field) {
        return true;
    }
    match client {
        SkillClient::ClaudeCode => true,
        SkillClient::Cursor => field == "disable-model-invocation",
        _ => false,
    }
}

/// Detect which client owns a SKILL.md based on its path components.
///
/// Iterates path components looking for the `skills` directory name, then
/// checks the parent component for a known client directory marker.
fn detect_client(path: &Path) -> SkillClient {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Walk backwards from the end to find "skills", then look at the component before it.
    let mut found_skills = false;
    for component in components.iter().rev() {
        if *component == "skills" {
            found_skills = true;
            continue;
        }
        if found_skills {
            return match *component {
                ".claude" => SkillClient::ClaudeCode,
                ".cursor" => SkillClient::Cursor,
                ".cline" => SkillClient::Cline,
                ".github" => SkillClient::Copilot,
                ".agents" => SkillClient::Codex, // NOTE: Amp also uses .agents; AMP-SK-001 unreachable without config context
                ".opencode" => SkillClient::OpenCode,
                ".windsurf" => SkillClient::Windsurf,
                ".kiro" => SkillClient::Kiro,
                ".roo" => SkillClient::RooCode,
                _ => SkillClient::Unknown,
            };
        }
    }

    SkillClient::Unknown
}

/// Return the rule ID for a per-client unsupported-field warning.
fn rule_id_for_client(client: SkillClient) -> Option<&'static str> {
    match client {
        SkillClient::Cursor => Some("CR-SK-001"),
        SkillClient::Cline => Some("CL-SK-001"),
        SkillClient::Copilot => Some("CP-SK-001"),
        SkillClient::Codex => Some("CX-SK-001"),
        SkillClient::OpenCode => Some("OC-SK-001"),
        SkillClient::Windsurf => Some("WS-SK-001"),
        SkillClient::Kiro => Some("KR-SK-001"),
        SkillClient::Amp => Some("AMP-SK-001"),
        SkillClient::RooCode => Some("RC-SK-001"),
        SkillClient::ClaudeCode | SkillClient::Unknown => None,
    }
}

/// Return a human-readable display name for a client.
fn client_display_name(client: SkillClient) -> &'static str {
    match client {
        SkillClient::ClaudeCode => "Claude Code",
        SkillClient::Cursor => "Cursor",
        SkillClient::Cline => "Cline",
        SkillClient::Copilot => "GitHub Copilot",
        SkillClient::Codex => "Codex CLI",
        SkillClient::OpenCode => "OpenCode",
        SkillClient::Windsurf => "Windsurf",
        SkillClient::Kiro => "Kiro",
        SkillClient::Amp => "Amp",
        SkillClient::RooCode => "Roo Code",
        SkillClient::Unknown => "Unknown",
    }
}

/// Return the i18n key prefix for a per-client rule.
fn i18n_key_for_client(client: SkillClient) -> Option<&'static str> {
    match client {
        SkillClient::Cursor => Some("cr_sk_001"),
        SkillClient::Cline => Some("cl_sk_001"),
        SkillClient::Copilot => Some("cp_sk_001"),
        SkillClient::Codex => Some("cx_sk_001"),
        SkillClient::OpenCode => Some("oc_sk_001"),
        SkillClient::Windsurf => Some("ws_sk_001"),
        SkillClient::Kiro => Some("kr_sk_001"),
        SkillClient::Amp => Some("amp_sk_001"),
        SkillClient::RooCode => Some("rc_sk_001"),
        SkillClient::ClaudeCode | SkillClient::Unknown => None,
    }
}

/// Compute line starts for position tracking.
fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

/// Convert a byte offset to (line, column), both 1-indexed.
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
    (low + 1, offset.saturating_sub(line_start) + 1)
}

/// Advance `fm_offset` past the current line and its newline terminator.
fn advance_past_line(fm_offset: usize, line_len: usize, fm_bytes: &[u8]) -> usize {
    let line_end = fm_offset + line_len;
    if line_end < fm_bytes.len() {
        if fm_bytes[line_end] == b'\n' {
            line_end + 1
        } else if line_end + 1 < fm_bytes.len()
            && fm_bytes[line_end] == b'\r'
            && fm_bytes[line_end + 1] == b'\n'
        {
            line_end + 2
        } else {
            line_end
        }
    } else {
        line_end
    }
}

const RULE_IDS: &[&str] = &[
    "AMP-SK-001",
    "CL-SK-001",
    "CP-SK-001",
    "CR-SK-001",
    "CX-SK-001",
    "KR-SK-001",
    "OC-SK-001",
    "RC-SK-001",
    "WS-SK-001",
    "XP-SK-001",
];

pub struct PerClientSkillValidator;

impl Validator for PerClientSkillValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let parts = split_frontmatter(content);
        if !parts.has_frontmatter || !parts.has_closing {
            return diagnostics;
        }

        let client = detect_client(path);

        let per_client_rule = rule_id_for_client(client);
        let has_per_client = per_client_rule
            .map(|r| config.is_rule_enabled(r))
            .unwrap_or(false);
        let has_xp = config.is_rule_enabled("XP-SK-001");

        if !has_per_client && !has_xp {
            return diagnostics;
        }

        let line_starts = compute_line_starts(content);
        let frontmatter_str = &parts.frontmatter;
        let fm_bytes = frontmatter_str.as_bytes();
        let mut fm_offset = 0usize;

        for line in frontmatter_str.lines() {
            let trimmed = line.trim_start();
            let leading_ws = line.len() - trimmed.len();

            // Skip empty lines, comments, and indented lines (nested YAML values
            // like sub-keys under `metadata:` are not top-level fields)
            if trimmed.is_empty() || trimmed.starts_with('#') || leading_ws > 0 {
                fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
                continue;
            }

            // Extract key from top-level "key: value" or "key:" pattern
            let key = match trimmed.find(':') {
                Some(colon_pos) => trimmed[..colon_pos].trim(),
                None => {
                    fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
                    continue;
                }
            };

            // Universal fields are safe everywhere - skip them
            if UNIVERSAL_FIELDS.contains(&key) {
                fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
                continue;
            }

            // Non-universal top-level key: check against client support
            let abs_key_start = parts.frontmatter_start + fm_offset;
            let (line_num, col) = line_col_at(abs_key_start, &line_starts);

            // Compute byte range for the full line including trailing newline
            let abs_line_start = parts.frontmatter_start + fm_offset;
            let mut abs_line_end = abs_line_start + line.len();
            if abs_line_end < content.len() && content.as_bytes()[abs_line_end] == b'\n' {
                abs_line_end += 1;
            } else if abs_line_end + 1 < content.len()
                && content.as_bytes()[abs_line_end] == b'\r'
                && content.as_bytes()[abs_line_end + 1] == b'\n'
            {
                abs_line_end += 2;
            }

            // Check whether the next line is indented (multi-line YAML value).
            // If so, deleting only the key line would leave orphaned content,
            // so the auto-fix is marked unsafe.
            let next_line_indented = content
                .get(abs_line_end..)
                .and_then(|rest| rest.lines().next())
                .is_some_and(|next| {
                    let nxt = next.trim_start();
                    !nxt.is_empty() && next.len() > nxt.len()
                });
            let fix_is_safe = !next_line_indented;

            // Per-client rule: warn if client does not support this field
            if has_per_client {
                if let Some(rule_id) = per_client_rule {
                    if !is_field_supported(client, key) {
                        let i18n_key = i18n_key_for_client(client).unwrap_or("cr_sk_001");
                        let msg_key = format!("rules.{}.message", i18n_key);
                        let sug_key = format!("rules.{}.suggestion", i18n_key);

                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                line_num,
                                col,
                                rule_id,
                                t!(&msg_key, field = key, client = client_display_name(client)),
                            )
                            .with_suggestion(t!(
                                &sug_key,
                                field = key,
                                client = client_display_name(client)
                            ))
                            .with_fix(Fix::delete(
                                abs_line_start,
                                abs_line_end,
                                format!(
                                    "Remove unsupported field '{}' for {}",
                                    key,
                                    client_display_name(client)
                                ),
                                fix_is_safe,
                            )),
                        );
                    }
                }
            }

            // XP-SK-001: cross-platform portability warning for any skill
            // with non-universal fields, except Claude Code (which supports all)
            if has_xp && client != SkillClient::ClaudeCode {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line_num,
                        col,
                        "XP-SK-001",
                        t!("rules.xp_sk_001.message", field = key),
                    )
                    .with_suggestion(t!("rules.xp_sk_001.suggestion", field = key)),
                );
            }

            fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::rules::Validator;

    fn make_skill(frontmatter: &str, body: &str) -> String {
        format!("---\n{}\n---\n{}", frontmatter, body)
    }

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = PerClientSkillValidator;
        let config = LintConfig::default();
        validator.validate(Path::new(path), content, &config)
    }

    // ===== detect_client tests =====

    #[test]
    fn test_detect_client_claude() {
        assert_eq!(
            detect_client(Path::new(".claude/skills/my-skill/SKILL.md")),
            SkillClient::ClaudeCode
        );
    }

    #[test]
    fn test_detect_client_cursor() {
        assert_eq!(
            detect_client(Path::new(".cursor/skills/my-skill/SKILL.md")),
            SkillClient::Cursor
        );
    }

    #[test]
    fn test_detect_client_cline() {
        assert_eq!(
            detect_client(Path::new(".cline/skills/my-skill/SKILL.md")),
            SkillClient::Cline
        );
    }

    #[test]
    fn test_detect_client_cline_alt() {
        // .clinerules is for Cline rules files, not skills; skills should be under .cline/
        assert_eq!(
            detect_client(Path::new(".clinerules/skills/my-skill/SKILL.md")),
            SkillClient::Unknown
        );
    }

    #[test]
    fn test_detect_client_copilot() {
        assert_eq!(
            detect_client(Path::new(".github/skills/my-skill/SKILL.md")),
            SkillClient::Copilot
        );
    }

    #[test]
    fn test_detect_client_codex() {
        assert_eq!(
            detect_client(Path::new(".agents/skills/my-skill/SKILL.md")),
            SkillClient::Codex
        );
    }

    #[test]
    fn test_detect_client_opencode() {
        assert_eq!(
            detect_client(Path::new(".opencode/skills/my-skill/SKILL.md")),
            SkillClient::OpenCode
        );
    }

    #[test]
    fn test_detect_client_windsurf() {
        assert_eq!(
            detect_client(Path::new(".windsurf/skills/my-skill/SKILL.md")),
            SkillClient::Windsurf
        );
    }

    #[test]
    fn test_detect_client_kiro() {
        assert_eq!(
            detect_client(Path::new(".kiro/skills/my-skill/SKILL.md")),
            SkillClient::Kiro
        );
    }

    #[test]
    fn test_detect_client_roo_code() {
        assert_eq!(
            detect_client(Path::new(".roo/skills/my-skill/SKILL.md")),
            SkillClient::RooCode
        );
    }

    #[test]
    fn test_detect_client_unknown_root() {
        assert_eq!(detect_client(Path::new("SKILL.md")), SkillClient::Unknown);
    }

    #[test]
    fn test_detect_client_unknown_no_skills() {
        assert_eq!(
            detect_client(Path::new("some/path/SKILL.md")),
            SkillClient::Unknown
        );
    }

    #[test]
    fn test_detect_client_nested() {
        assert_eq!(
            detect_client(Path::new("projects/foo/.cursor/skills/review/SKILL.md")),
            SkillClient::Cursor
        );
    }

    // ===== Validation tests =====

    #[test]
    fn test_universal_fields_no_diagnostics() {
        let content = make_skill(
            "name: my-skill\ndescription: A test skill\nlicense: MIT",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        assert!(
            diags.is_empty(),
            "Universal-only fields should produce no diagnostics, got {:?}",
            diags
        );
    }

    #[test]
    fn test_claude_code_no_diagnostics() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmodel: opus\ncontext: fork\nagent: general-purpose",
            "Body",
        );
        let diags = validate(".claude/skills/my-skill/SKILL.md", &content);
        // Claude Code supports all extension fields -- no per-client warnings
        let per_client: Vec<_> = diags
            .iter()
            .filter(|d| {
                d.rule.starts_with("CR-SK-")
                    || d.rule.starts_with("CL-SK-")
                    || d.rule.starts_with("CP-SK-")
                    || d.rule.starts_with("CX-SK-")
                    || d.rule.starts_with("OC-SK-")
                    || d.rule.starts_with("WS-SK-")
                    || d.rule.starts_with("KR-SK-")
                    || d.rule.starts_with("AMP-SK-")
                    || d.rule.starts_with("RC-SK-")
                    || d.rule == "XP-SK-001"
            })
            .collect();
        assert!(
            per_client.is_empty(),
            "Claude Code should have no per-client or XP-SK-001 warnings, got {:?}",
            per_client
        );
    }

    #[test]
    fn test_cursor_unsupported_model() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert_eq!(
            cr_diags.len(),
            1,
            "Expected 1 CR-SK-001 for unsupported 'model', got {}",
            cr_diags.len()
        );
        assert!(cr_diags[0].message.contains("model"));
    }

    #[test]
    fn test_cursor_disable_model_invocation_supported() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\ndisable-model-invocation: true",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert!(
            cr_diags.is_empty(),
            "Cursor supports disable-model-invocation, should have no CR-SK-001, got {:?}",
            cr_diags
        );
    }

    #[test]
    fn test_cline_unsupported_context() {
        let content = make_skill("name: my-skill\ndescription: A test\ncontext: fork", "Body");
        let diags = validate(".cline/skills/my-skill/SKILL.md", &content);
        let cl_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CL-SK-001").collect();
        assert_eq!(cl_diags.len(), 1);
        assert!(cl_diags[0].message.contains("context"));
    }

    #[test]
    fn test_copilot_unsupported_agent() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nagent: general-purpose",
            "Body",
        );
        let diags = validate(".github/skills/my-skill/SKILL.md", &content);
        let cp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CP-SK-001").collect();
        assert_eq!(cp_diags.len(), 1);
        assert!(cp_diags[0].message.contains("agent"));
    }

    #[test]
    fn test_codex_unsupported_hooks() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nhooks: some-value",
            "Body",
        );
        let diags = validate(".agents/skills/my-skill/SKILL.md", &content);
        let cx_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CX-SK-001").collect();
        assert_eq!(cx_diags.len(), 1);
        assert!(cx_diags[0].message.contains("hooks"));
    }

    #[test]
    fn test_opencode_unsupported_argument_hint() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nargument-hint: hint",
            "Body",
        );
        let diags = validate(".opencode/skills/my-skill/SKILL.md", &content);
        let oc_diags: Vec<_> = diags.iter().filter(|d| d.rule == "OC-SK-001").collect();
        assert_eq!(oc_diags.len(), 1);
        assert!(oc_diags[0].message.contains("argument-hint"));
    }

    #[test]
    fn test_windsurf_unsupported_user_invocable() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nuser-invocable: true",
            "Body",
        );
        let diags = validate(".windsurf/skills/my-skill/SKILL.md", &content);
        let ws_diags: Vec<_> = diags.iter().filter(|d| d.rule == "WS-SK-001").collect();
        assert_eq!(ws_diags.len(), 1);
        assert!(ws_diags[0].message.contains("user-invocable"));
    }

    #[test]
    fn test_kiro_unsupported_model() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: haiku", "Body");
        let diags = validate(".kiro/skills/my-skill/SKILL.md", &content);
        let kr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "KR-SK-001").collect();
        assert_eq!(kr_diags.len(), 1);
    }

    #[test]
    fn test_roo_code_unsupported_disable_model() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\ndisable-model-invocation: false",
            "Body",
        );
        let diags = validate(".roo/skills/my-skill/SKILL.md", &content);
        let rc_diags: Vec<_> = diags.iter().filter(|d| d.rule == "RC-SK-001").collect();
        assert_eq!(rc_diags.len(), 1);
    }

    #[test]
    fn test_multiple_unsupported_fields() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmodel: opus\ncontext: fork\nagent: general-purpose",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        // model, context, agent are all unsupported by Cursor
        assert_eq!(
            cr_diags.len(),
            3,
            "Expected 3 CR-SK-001 for model/context/agent, got {}",
            cr_diags.len()
        );
    }

    #[test]
    fn test_xp_sk_001_fires_for_non_claude() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert_eq!(
            xp_diags.len(),
            1,
            "Expected 1 XP-SK-001 for extension field in non-Claude skill"
        );
    }

    #[test]
    fn test_xp_sk_001_does_not_fire_for_claude() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".claude/skills/my-skill/SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert!(
            xp_diags.is_empty(),
            "XP-SK-001 should not fire for Claude Code"
        );
    }

    #[test]
    fn test_xp_sk_001_fires_for_unknown() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate("SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert_eq!(
            xp_diags.len(),
            1,
            "XP-SK-001 should fire for root-level skills with non-universal fields"
        );
    }

    #[test]
    fn test_indented_keys_not_flagged() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmetadata:\n  model: some-value\n  context: note",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert!(
            cr_diags.is_empty(),
            "Indented keys under metadata should not trigger per-client warnings, got {:?}",
            cr_diags
        );
    }

    #[test]
    fn test_no_frontmatter_no_diagnostics() {
        let content = "Just body content without frontmatter";
        let diags = validate(".cursor/skills/my-skill/SKILL.md", content);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_fix_attached_for_unsupported_field() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diag = diags.iter().find(|d| d.rule == "CR-SK-001").unwrap();
        assert!(
            cr_diag.has_fixes(),
            "CR-SK-001 should have an auto-fix attached"
        );
        let fix = &cr_diag.fixes[0];
        assert!(fix.is_deletion(), "Fix should be a line deletion");
        assert!(fix.safe, "Fix should be safe");
    }

    #[test]
    fn test_disabled_rule_not_fired() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CR-SK-001".to_string()];
        let validator = PerClientSkillValidator;
        let diags = validator.validate(
            Path::new(".cursor/skills/my-skill/SKILL.md"),
            &content,
            &config,
        );
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert!(cr_diags.is_empty(), "Disabled rule should not fire");
    }

    #[test]
    fn test_line_numbers_correct() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmodel: opus",
            "Body content",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diag = diags.iter().find(|d| d.rule == "CR-SK-001").unwrap();
        // Line 1: ---
        // Line 2: (empty from frontmatter start)
        // Actually frontmatter content begins at byte 3 (after "---")
        // The frontmatter is "\nname: my-skill\ndescription: A test\nmodel: opus"
        // "model: opus" is on line 4 of the full content
        assert_eq!(cr_diag.line, 4, "model should be on line 4");
    }

    #[test]
    fn test_unknown_custom_field_flagged() {
        // A completely unknown field (not in UNIVERSAL_FIELDS or EXTENSION_FIELDS)
        // should be flagged for non-Claude clients
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmy-custom-field: value",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert_eq!(
            cr_diags.len(),
            1,
            "Unknown custom field should trigger CR-SK-001"
        );
        assert!(cr_diags[0].message.contains("my-custom-field"));
    }

    #[test]
    fn test_unknown_custom_field_xp_fires() {
        // XP-SK-001 should fire for any non-universal field, including unknown ones
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmy-custom-field: value",
            "Body",
        );
        let diags = validate("SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert_eq!(
            xp_diags.len(),
            1,
            "XP-SK-001 should fire for unknown custom field"
        );
    }

    #[test]
    fn test_multiline_value_fix_marked_unsafe() {
        // A field with multi-line YAML value (next line is indented) should
        // have its auto-fix marked as unsafe since deleting only the key
        // line would leave orphaned indented content.
        let content = make_skill(
            "name: my-skill\ndescription: A test\nallowed-tools:\n  - Read\n  - Write\nmodel: opus",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        // 'model' is a single-line field -> safe fix
        let model_diag = diags
            .iter()
            .find(|d| d.rule == "CR-SK-001" && d.message.contains("model"))
            .expect("Expected CR-SK-001 for model");
        assert!(
            model_diag.fixes[0].safe,
            "Single-line field fix should be safe"
        );
    }

    #[test]
    fn test_multiline_value_with_list_fix_unsafe() {
        // 'context' key followed by indented lines -> unsafe fix
        let content = make_skill(
            "name: my-skill\ndescription: A test\ncontext:\n  - file1.rs\n  - file2.rs",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let ctx_diag = diags
            .iter()
            .find(|d| d.rule == "CR-SK-001" && d.message.contains("context"))
            .expect("Expected CR-SK-001 for context");
        assert!(
            !ctx_diag.fixes[0].safe,
            "Multi-line field fix should be unsafe"
        );
    }

    #[test]
    fn test_unknown_custom_field_not_flagged_for_claude() {
        // Claude Code accepts all fields, including unknown ones
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmy-custom-field: value",
            "Body",
        );
        let diags = validate(".claude/skills/my-skill/SKILL.md", &content);
        assert!(
            diags.is_empty(),
            "Claude Code should not flag any fields, got {:?}",
            diags
        );
    }
}
