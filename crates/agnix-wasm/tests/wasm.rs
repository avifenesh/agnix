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
    assert!(arr.length() > 0, "Should find issues in content with unclosed XML tag");
}

#[wasm_bindgen_test]
fn validate_returns_empty_for_unknown_type() {
    let result = agnix_wasm::validate("main.rs", "fn main() {}", None);
    assert!(!result.is_null());
    let diagnostics = js_sys::Reflect::get(&result, &JsValue::from_str("diagnostics")).unwrap();
    let arr = js_sys::Array::from(&diagnostics);
    assert_eq!(arr.length(), 0, "Unknown file type should produce no diagnostics");
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
    assert_eq!(arr.length(), 0, "Oversized content should return empty diagnostics");
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
fn detect_type_unknown_file() {
    let result = agnix_wasm::detect_type("main.rs");
    assert!(result.is_empty(), "Unknown type should return empty string");
}
