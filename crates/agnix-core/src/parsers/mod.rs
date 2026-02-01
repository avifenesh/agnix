//! File parsers for different config formats

pub mod frontmatter;
pub mod json;
pub mod markdown;

pub use frontmatter::{parse_frontmatter, split_frontmatter, FrontmatterParts};
pub use json::parse_json_config;
