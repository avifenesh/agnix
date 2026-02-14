use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn validate_returns_diagnostics_for_claude_md() {
    let result = agnix_wasm::validate("CLAUDE.md", "<unclosed>", None);
    assert!(!result.is_null());
    // Verify it's an object with diagnostics array
    let diagnostics = js_sys::Reflect::get(&result, &JsValue::from_str("diagnostics")).unwrap();
    assert!(js_sys::Array::is_array(&diagnostics));
    let arr = js_sys::Array::from(&diagnostics);
    assert!(
        arr.length() > 0,
        "Should find issues in content with unclosed XML tag"
    );
}

#[wasm_bindgen_test]
fn validate_returns_empty_for_unknown_type() {
    let result = agnix_wasm::validate("main.rs", "fn main() {}", None);
    assert!(!result.is_null());
    let diagnostics = js_sys::Reflect::get(&result, &JsValue::from_str("diagnostics")).unwrap();
    let arr = js_sys::Array::from(&diagnostics);
    assert_eq!(
        arr.length(),
        0,
        "Unknown file type should produce no diagnostics"
    );
}

#[wasm_bindgen_test]
fn validate_returns_file_type() {
    let result = agnix_wasm::validate("CLAUDE.md", "", None);
    let file_type = js_sys::Reflect::get(&result, &JsValue::from_str("file_type")).unwrap();
    assert_eq!(file_type.as_string().unwrap(), "ClaudeMd");
}

#[wasm_bindgen_test]
fn validate_with_tool_filter() {
    let result = agnix_wasm::validate("CLAUDE.md", "# Project", Some("cursor".to_string()));
    assert!(!result.is_null());
}

#[wasm_bindgen_test]
fn validate_rejects_oversized_content() {
    let big = "x".repeat(2_000_000);
    let result = agnix_wasm::validate("CLAUDE.md", &big, None);
    let diagnostics = js_sys::Reflect::get(&result, &JsValue::from_str("diagnostics")).unwrap();
    let arr = js_sys::Array::from(&diagnostics);
    assert_eq!(
        arr.length(),
        0,
        "Oversized content should return empty diagnostics"
    );
}

#[wasm_bindgen_test]
fn get_supported_file_types_returns_array() {
    let types = agnix_wasm::get_supported_file_types();
    assert!(!types.is_null());
    assert!(js_sys::Array::is_array(&types));
    let arr = js_sys::Array::from(&types);
    assert!(arr.length() > 0);
}

#[wasm_bindgen_test]
fn get_supported_file_types_includes_cursor_hooks_agents_environment() {
    let types = agnix_wasm::get_supported_file_types();
    let arr = js_sys::Array::from(&types);

    let mut pairs = std::collections::HashSet::new();
    for value in arr.iter() {
        let pair = js_sys::Array::from(&value);
        let filename = pair.get(0).as_string().unwrap_or_default();
        let file_type = pair.get(1).as_string().unwrap_or_default();
        pairs.insert((filename, file_type));
    }

    assert!(pairs.contains(&(".cursor/hooks.json".to_string(), "CursorHooks".to_string())));
    assert!(pairs.contains(&(
        ".cursor/agents/reviewer.md".to_string(),
        "CursorAgent".to_string()
    )));
    assert!(pairs.contains(&(
        ".cursor/environment.json".to_string(),
        "CursorEnvironment".to_string()
    )));
}

#[wasm_bindgen_test]
fn get_supported_tools_returns_array() {
    let tools = agnix_wasm::get_supported_tools();
    assert!(!tools.is_null());
    assert!(js_sys::Array::is_array(&tools));
    let arr = js_sys::Array::from(&tools);
    assert!(arr.length() > 0);
}

#[wasm_bindgen_test]
fn detect_type_known_file() {
    let result = agnix_wasm::detect_type("CLAUDE.md");
    assert_eq!(result, "ClaudeMd");
}

#[wasm_bindgen_test]
fn detect_type_cursor_auxiliary_files() {
    assert_eq!(agnix_wasm::detect_type(".cursor/hooks.json"), "CursorHooks");
    assert_eq!(
        agnix_wasm::detect_type(".cursor/agents/reviewer.md"),
        "CursorAgent"
    );
    assert_eq!(
        agnix_wasm::detect_type(".cursor/environment.json"),
        "CursorEnvironment"
    );
}

#[wasm_bindgen_test]
fn detect_type_unknown_file() {
    let result = agnix_wasm::detect_type("main.rs");
    assert!(result.is_empty(), "Unknown type should return empty string");
}
