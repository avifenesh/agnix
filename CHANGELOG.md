# Changelog

All notable changes to agnix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Parallel file validation using rayon for improved performance on large projects
- Deterministic diagnostic output with sorting by severity and file path
- Comprehensive tests for parallel validation edge cases

### Changed
- `validate_project()` now processes files in parallel while maintaining deterministic output
- Directory walking remains sequential, only validation is parallelized

### Performance
- Significant speed improvements on projects with many files
- Maintains correctness with deterministic sorting of results

