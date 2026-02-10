//! Fuzz target for Markdown parsing
//!
//! This target tests the XML tag and import extraction functions from
//! markdown content. These functions use regex internally and must handle
//! arbitrary input without panicking or producing invalid output.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Test extract_xml_tags() - should never panic
    let tags = agnix_core::extract_xml_tags(data);

    // Verify invariants for XML tags:
    for tag in &tags {
        // Byte offsets must be within bounds
        assert!(tag.start_byte <= data.len());
        assert!(tag.end_byte <= data.len());
        assert!(tag.start_byte <= tag.end_byte);

        // UTF-8 boundary validation: byte offsets must be at character boundaries
        assert!(data.is_char_boundary(tag.start_byte), "start_byte must be at UTF-8 boundary");
        assert!(data.is_char_boundary(tag.end_byte), "end_byte must be at UTF-8 boundary");

        // Line/column must be positive (1-indexed)
        assert!(tag.line >= 1);
        assert!(tag.column >= 1);
    }

    // Test check_xml_balance() - should never panic
    let _errors = agnix_core::check_xml_balance(&tags);
    let _errors_with_end =
        agnix_core::check_xml_balance_with_content_end(&tags, Some(data.len()));

    // Test extract_imports() - should never panic
    let imports = agnix_core::extract_imports(data);

    // Verify invariants for imports:
    for import in &imports {
        // Byte offsets must be within bounds
        assert!(import.start_byte <= data.len());
        assert!(import.end_byte <= data.len());
        assert!(import.start_byte <= import.end_byte);

        // UTF-8 boundary validation
        assert!(data.is_char_boundary(import.start_byte), "start_byte must be at UTF-8 boundary");
        assert!(data.is_char_boundary(import.end_byte), "end_byte must be at UTF-8 boundary");

        // Line/column must be positive (1-indexed)
        assert!(import.line >= 1);
        assert!(import.column >= 1);
    }

    // Test extract_markdown_links() - should never panic
    let links = agnix_core::extract_markdown_links(data);

    // Verify invariants for links:
    for link in &links {
        // Byte offsets must be within bounds
        assert!(link.start_byte <= data.len());
        assert!(link.end_byte <= data.len());
        assert!(link.start_byte <= link.end_byte);

        // UTF-8 boundary validation
        assert!(data.is_char_boundary(link.start_byte), "start_byte must be at UTF-8 boundary");
        assert!(data.is_char_boundary(link.end_byte), "end_byte must be at UTF-8 boundary");

        // Line/column must be positive (1-indexed)
        assert!(link.line >= 1);
        assert!(link.column >= 1);
    }
});
