# Implementation Plan: LSP Diagnostics and Code Actions (#19)

## Overview

Add real-time diagnostics via `did_change` handler, quick-fix code actions by converting Fix objects to CodeActions, and hover documentation for field values.

**Complexity**: Medium | **Confidence**: High | **Steps**: 16

## Architecture

- Document cache (`RwLock<HashMap<Url, String>>`) stores file contents for real-time validation
- Byte offsets from Fix structs converted to LSP positions using position utility module
- Code actions generated from diagnostics with fixes, stored in `diagnostic.data` as JSON
- Hover documentation from static field docs with YAML position detection

## Implementation Steps

### Step 1: Add position utilities module
**Goal**: Create byte offset to LSP position conversion utilities (UTF-8 aware)

**Files**:
- `crates/agnix-lsp/src/position.rs` (create) - byte_to_position(), byte_range_to_lsp_range()
- `crates/agnix-lsp/src/lib.rs` (modify) - add module declaration

### Step 2: Add document content cache to Backend
**Goal**: Store document contents for real-time validation without disk reads

**Files**:
- `crates/agnix-lsp/src/backend.rs` (modify) - add `documents: RwLock<HashMap<Url, String>>` field

### Step 3: Implement did_change handler for real-time diagnostics
**Goal**: Validate on every keystroke

**Files**:
- `crates/agnix-lsp/src/backend.rs` (modify) - add did_change method, validate_with_content()

### Step 4: Update did_open to cache document content
**Goal**: Populate document cache when file is opened

**Files**:
- `crates/agnix-lsp/src/backend.rs` (modify) - store content in documents cache

### Step 5: Update did_close to clear document cache
**Goal**: Remove document from cache when closed

**Files**:
- `crates/agnix-lsp/src/backend.rs` (modify) - remove from documents HashMap

### Step 6: Create code_actions module
**Goal**: Convert agnix-core Fix objects to LSP CodeAction responses

**Files**:
- `crates/agnix-lsp/src/code_actions.rs` (create) - fix_to_code_action(), create_workspace_edit()
- `crates/agnix-lsp/src/lib.rs` (modify) - add module declaration

### Step 7: Store fix data in diagnostic
**Goal**: Attach fix information to diagnostics for code_action retrieval

**Files**:
- `crates/agnix-lsp/src/diagnostic_mapper.rs` (modify) - serialize fixes into diagnostic.data

### Step 8: Implement textDocument/codeAction handler
**Goal**: Return quick fixes for diagnostics with available fixes

**Files**:
- `crates/agnix-lsp/src/backend.rs` (modify) - add code_action method, register capability

### Step 9: Create hover_provider module
**Goal**: Provide hover documentation for YAML frontmatter fields

**Files**:
- `crates/agnix-lsp/src/hover_provider.rs` (create) - get_hover_info(), FIELD_DOCS
- `crates/agnix-lsp/src/lib.rs` (modify) - add module declaration

### Step 10: Implement textDocument/hover handler
**Goal**: Show field documentation on hover

**Files**:
- `crates/agnix-lsp/src/backend.rs` (modify) - add hover method, register capability

### Step 11: Add serde_json dependency
**Goal**: Enable JSON serialization for diagnostic data

**Files**:
- `crates/agnix-lsp/Cargo.toml` (modify) - add serde_json dependency

### Step 12-16: Tests
- Unit tests for position utilities (position.rs)
- Unit tests for code_actions module (code_actions.rs)
- Unit tests for hover_provider module (hover_provider.rs)
- Integration tests for LSP features (tests/lsp_integration.rs)
- Backend tests for new functionality (backend.rs)

## Critical Considerations

### High Risk
- `position.rs` - Byte to UTF-8 position conversion must handle multi-byte characters

### Needs Review
- `diagnostic_mapper.rs` - diagnostic.data field format affects code_action handler
- `hover_provider.rs` - YAML frontmatter detection and field extraction

### Performance
- did_change runs full validation - may need debouncing for large files
- Code action deserializes fixes on every request

## File Summary

| File | Action | Purpose |
|------|--------|---------|
| `crates/agnix-lsp/src/position.rs` | Create | Byte-to-position utilities |
| `crates/agnix-lsp/src/code_actions.rs` | Create | Fix to CodeAction conversion |
| `crates/agnix-lsp/src/hover_provider.rs` | Create | Hover documentation |
| `crates/agnix-lsp/src/backend.rs` | Modify | Add handlers and document cache |
| `crates/agnix-lsp/src/diagnostic_mapper.rs` | Modify | Store fixes in diagnostic.data |
| `crates/agnix-lsp/src/lib.rs` | Modify | Module declarations |
| `crates/agnix-lsp/Cargo.toml` | Modify | Add serde_json |
| `crates/agnix-lsp/tests/lsp_integration.rs` | Modify | Integration tests |
