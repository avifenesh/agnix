# Implementation Plan: Task #113 - Harden File Reads

## Overview
Create a centralized safe file reading module (file_utils.rs) that:
- Rejects symlinks using `symlink_metadata()` (not `metadata()` which follows symlinks)
- Enforces max file size limits (1 MiB default)
- Provides clear error handling via new `LintError` variants

## Steps

### Step 1: Add new LintError variants in diagnostics.rs
- Add `FileSymlink { path: PathBuf }` for rejected symlinks
- Add `FileTooBig { path, size, limit }` for files exceeding size limit

### Step 2: Create file_utils.rs module
- `DEFAULT_MAX_FILE_SIZE = 1_048_576` (1 MiB)
- `safe_read_file(path) -> LintResult<String>`
- Uses `fs::symlink_metadata()` to detect symlinks without following them
- Checks size before reading

### Step 3: Export from lib.rs and update validate_file_with_registry
- Add `pub mod file_utils;`
- Replace `std::fs::read_to_string(path)` with `file_utils::safe_read_file(path)?`

### Step 4: Update imports.rs
- Replace `std::fs::read_to_string(file_path).ok()?` with safe reader

### Step 5: Update fixes.rs
- Replace `std::fs::read_to_string(&path)` with safe reader

### Step 6: Update config.rs
- Replace `std::fs::read_to_string(path)?` with safe reader

### Step 7: Remove local safe_read_file from claude_md.rs
- Delete the buggy local function (uses `metadata()` which follows symlinks)
- Use centralized `file_utils::safe_read_file().ok()`

### Step 8: Add unit tests for file_utils
- Normal file reading succeeds
- Symlink is rejected (platform-dependent)
- Oversized file is rejected
- Non-existent file returns error

### Step 9: Add integration tests
- validate_file rejects symlink pointing to valid SKILL.md
- validate_project skips symlinks gracefully
- Behavior with oversized files

## Key Technical Decision
Use `fs::symlink_metadata()` NOT `fs::metadata()`. The latter follows symlinks, which is the bug in the existing `safe_read_file()` in claude_md.rs.

## Files Changed
| File | Action |
|------|--------|
| diagnostics.rs | Modify - add error variants |
| file_utils.rs | Create - new module |
| lib.rs | Modify - export module, update validation |
| imports.rs | Modify - use safe reader |
| fixes.rs | Modify - use safe reader |
| config.rs | Modify - use safe reader |
| claude_md.rs | Modify - remove local function |
