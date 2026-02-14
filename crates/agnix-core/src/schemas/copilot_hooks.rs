//! GitHub Copilot hooks and setup workflow schema helpers.

use serde_json::Value;
use serde_yaml::{Mapping, Value as YamlValue};

/// Valid event names for Copilot coding agent hooks.
pub const VALID_EVENTS: &[&str] = &[
    "sessionStart",
    "sessionEnd",
    "userPromptSubmitted",
    "preToolUse",
    "postToolUse",
    "errorOccurred",
];

/// Parse hooks JSON and return a generic JSON value.
pub fn parse_hooks_json(content: &str) -> Result<Value, String> {
    serde_json::from_str(content).map_err(|e| e.to_string())
}

/// Validate Copilot hooks schema.
///
/// Accepts two shapes for compatibility:
/// - `{ "version": 1, "hooks": [{ "type": "command", "events": [...], "command": {...} }] }`
/// - `{ "version": 1, "hooks": { "sessionStart": [{ "type": "command", "command": {...} }] } }`
pub fn validate_hooks_schema(root: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    let Some(obj) = root.as_object() else {
        errors.push("root must be a JSON object".to_string());
        return errors;
    };

    match obj.get("version") {
        Some(Value::Number(n)) if n.as_i64() == Some(1) => {}
        Some(_) => errors.push("version must be number 1".to_string()),
        None => errors.push("missing required field 'version'".to_string()),
    }

    let Some(hooks) = obj.get("hooks") else {
        errors.push("missing required field 'hooks'".to_string());
        return errors;
    };

    if let Some(hook_list) = hooks.as_array() {
        validate_array_format(hook_list, &mut errors);
    } else if let Some(hook_map) = hooks.as_object() {
        validate_event_map_format(hook_map, &mut errors);
    } else {
        errors.push("hooks must be an array or object".to_string());
    }

    errors
}

fn validate_array_format(hooks: &[Value], errors: &mut Vec<String>) {
    for (idx, hook) in hooks.iter().enumerate() {
        let Some(obj) = hook.as_object() else {
            errors.push(format!("hooks[{idx}] must be an object"));
            continue;
        };

        let Some(events) = obj.get("events").and_then(Value::as_array) else {
            errors.push(format!(
                "hooks[{idx}] missing required array field 'events'"
            ));
            continue;
        };
        if events.is_empty() {
            errors.push(format!("hooks[{idx}].events must not be empty"));
        }
        for (event_idx, event) in events.iter().enumerate() {
            match event.as_str() {
                Some(name) if VALID_EVENTS.contains(&name) => {}
                Some(name) => errors.push(format!(
                    "hooks[{idx}].events[{event_idx}] has invalid event '{name}'"
                )),
                None => errors.push(format!("hooks[{idx}].events[{event_idx}] must be a string")),
            }
        }

        match obj.get("type") {
            Some(Value::String(kind)) if kind == "command" => {}
            Some(Value::String(kind)) => errors.push(format!(
                "hooks[{idx}].type must be 'command' (found '{kind}')"
            )),
            Some(_) => errors.push(format!("hooks[{idx}].type must be string 'command'")),
            None => errors.push(format!("hooks[{idx}] missing required field 'type'")),
        }

        match obj.get("command") {
            Some(command) => {
                validate_command_object(command, &format!("hooks[{idx}].command"), errors)
            }
            None => errors.push(format!("hooks[{idx}] missing required field 'command'")),
        }
    }
}

fn validate_event_map_format(hooks: &serde_json::Map<String, Value>, errors: &mut Vec<String>) {
    for (event, entries) in hooks {
        if !VALID_EVENTS.contains(&event.as_str()) {
            errors.push(format!("invalid hook event '{event}'"));
            continue;
        }

        let Some(list) = entries.as_array() else {
            errors.push(format!("hooks.{event} must be an array"));
            continue;
        };

        for (idx, entry) in list.iter().enumerate() {
            let Some(obj) = entry.as_object() else {
                errors.push(format!("hooks.{event}[{idx}] must be an object"));
                continue;
            };

            match obj.get("type") {
                Some(Value::String(kind)) if kind == "command" => {}
                Some(Value::String(kind)) => errors.push(format!(
                    "hooks.{event}[{idx}].type must be 'command' (found '{kind}')"
                )),
                Some(_) => errors.push(format!(
                    "hooks.{event}[{idx}].type must be string 'command'"
                )),
                None => errors.push(format!(
                    "hooks.{event}[{idx}] missing required field 'type'"
                )),
            }

            match obj.get("command") {
                Some(command) => validate_command_object(
                    command,
                    &format!("hooks.{event}[{idx}].command"),
                    errors,
                ),
                None => errors.push(format!(
                    "hooks.{event}[{idx}] missing required field 'command'"
                )),
            }
        }
    }
}

fn validate_command_object(command: &Value, context: &str, errors: &mut Vec<String>) {
    let Some(obj) = command.as_object() else {
        errors.push(format!("{context} must be an object"));
        return;
    };

    let bash = obj.get("bash").and_then(Value::as_str);
    let powershell = obj.get("powershell").and_then(Value::as_str);
    let has_bash = bash.is_some_and(|v| !v.trim().is_empty());
    let has_powershell = powershell.is_some_and(|v| !v.trim().is_empty());

    if !has_bash && !has_powershell {
        errors.push(format!(
            "{context} must include non-empty 'bash' or 'powershell'"
        ));
    }
}

/// Parse Copilot setup workflow YAML.
pub fn parse_setup_steps_yaml(content: &str) -> Result<YamlValue, String> {
    serde_yaml::from_str(content).map_err(|e| e.to_string())
}

/// Return true when workflow YAML contains a valid `jobs.copilot-setup-steps` job.
pub fn has_copilot_setup_steps_job(workflow: &YamlValue) -> bool {
    let Some(root) = workflow.as_mapping() else {
        return false;
    };
    let Some(jobs) = yaml_get(root, "jobs").and_then(YamlValue::as_mapping) else {
        return false;
    };
    let Some(job) = yaml_get(jobs, "copilot-setup-steps") else {
        return false;
    };
    let Some(job_map) = job.as_mapping() else {
        return false;
    };

    // Minimal shape validation to reduce false negatives on malformed jobs.
    let has_steps = yaml_get(job_map, "steps")
        .and_then(YamlValue::as_sequence)
        .is_some_and(|steps| !steps.is_empty());
    let has_runs_on = yaml_get(job_map, "runs-on").is_some_and(has_supported_ubuntu_runner);

    has_steps && has_runs_on
}

fn has_supported_ubuntu_runner(runs_on: &YamlValue) -> bool {
    match runs_on {
        YamlValue::String(label) => is_supported_runner_label(label),
        YamlValue::Sequence(labels) => labels
            .iter()
            .filter_map(YamlValue::as_str)
            .any(is_supported_runner_label),
        _ => false,
    }
}

fn is_supported_runner_label(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    lower.contains("ubuntu") || lower.contains("${{")
}

fn yaml_get<'a>(map: &'a Mapping, key: &str) -> Option<&'a YamlValue> {
    map.get(&YamlValue::String(key.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_array_format_ok() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": [
    { "type": "command", "events": ["sessionStart"], "command": { "bash": "echo ok" } }
  ]
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn validate_reports_non_object_root() {
        let value = parse_hooks_json(r#"["not-an-object"]"#).expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("root must be a JSON object"))
        );
    }

    #[test]
    fn validate_reports_invalid_version() {
        let value = parse_hooks_json(
            r#"{
  "version": 2,
  "hooks": []
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("version must be number 1"))
        );
    }

    #[test]
    fn validate_reports_missing_hooks() {
        let value = parse_hooks_json(
            r#"{
  "version": 1
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missing required field 'hooks'"))
        );
    }

    #[test]
    fn validate_reports_invalid_hooks_shape() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": 42
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("hooks must be an array or object"))
        );
    }

    #[test]
    fn validate_map_format_ok() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": {
    "sessionStart": [{ "type": "command", "command": { "bash": "echo ok" } }]
  }
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn validate_map_reports_invalid_event_key() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": {
    "notReal": [{ "type": "command", "command": { "bash": "echo ok" } }]
  }
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(errors.iter().any(|e| e.contains("invalid hook event")));
    }

    #[test]
    fn validate_map_reports_non_array_entries() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": {
    "sessionStart": { "type": "command", "command": { "bash": "echo ok" } }
  }
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("hooks.sessionStart must be an array"))
        );
    }

    #[test]
    fn validate_map_reports_missing_type() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": {
    "sessionStart": [{ "command": { "bash": "echo ok" } }]
  }
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("hooks.sessionStart[0] missing required field 'type'"))
        );
    }

    #[test]
    fn validate_map_reports_missing_command() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": {
    "sessionStart": [{ "type": "command" }]
  }
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("hooks.sessionStart[0] missing required field 'command'"))
        );
    }

    #[test]
    fn validate_reports_invalid_event() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": [
    { "type": "command", "events": ["notReal"], "command": { "bash": "echo ok" } }
  ]
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(errors.iter().any(|e| e.contains("invalid event")));
    }

    #[test]
    fn validate_reports_missing_type() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": [
    { "events": ["sessionStart"], "command": { "bash": "echo ok" } }
  ]
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missing required field 'type'"))
        );
    }

    #[test]
    fn validate_reports_missing_command() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": [
    { "type": "command", "events": ["sessionStart"] }
  ]
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missing required field 'command'"))
        );
    }

    #[test]
    fn validate_reports_missing_shell_command() {
        let value = parse_hooks_json(
            r#"{
  "version": 1,
  "hooks": [
    { "type": "command", "events": ["sessionStart"], "command": {} }
  ]
}"#,
        )
        .expect("expected valid json");
        let errors = validate_hooks_schema(&value);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("must include non-empty 'bash' or 'powershell'"))
        );
    }

    #[test]
    fn setup_job_detection() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  copilot-setup-steps:
    runs-on: ubuntu-latest
    steps:
      - run: echo hi
"#,
        )
        .expect("expected valid yaml");
        assert!(has_copilot_setup_steps_job(&yaml));
    }

    #[test]
    fn setup_job_missing_detection() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  build:
    runs-on: ubuntu-latest
"#,
        )
        .expect("expected valid yaml");
        assert!(!has_copilot_setup_steps_job(&yaml));
    }

    #[test]
    fn wrong_job_name_not_accepted() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  setup:
    runs-on: ubuntu-latest
    steps:
      - run: echo hi
"#,
        )
        .expect("expected valid yaml");
        assert!(!has_copilot_setup_steps_job(&yaml));
    }

    #[test]
    fn unsupported_runner_not_accepted() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  copilot-setup-steps:
    runs-on: windows-latest
    steps:
      - run: echo hi
"#,
        )
        .expect("expected valid yaml");
        assert!(!has_copilot_setup_steps_job(&yaml));
    }

    #[test]
    fn expression_runner_is_accepted() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  copilot-setup-steps:
    runs-on: ${{ matrix.runner }}
    steps:
      - run: echo hi
"#,
        )
        .expect("expected valid yaml");
        assert!(has_copilot_setup_steps_job(&yaml));
    }

    #[test]
    fn missing_steps_not_accepted() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  copilot-setup-steps:
    runs-on: ubuntu-latest
"#,
        )
        .expect("expected valid yaml");
        assert!(!has_copilot_setup_steps_job(&yaml));
    }
}
