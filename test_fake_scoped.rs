#[test]
fn test_cc_sk_008_scoped_unknown_tool() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool(scope:*) Read Write
---
Body"#;

    let validator = crate::rules::skill::SkillValidator;
    let config = crate::config::LintConfig::default();
    let diagnostics = validator.validate(std::path::Path::new("test.md"), content, &config);

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 1, "Should detect FakeTool as unknown even when scoped");
    assert!(cc_sk_008[0].message.contains("FakeTool"));
}
