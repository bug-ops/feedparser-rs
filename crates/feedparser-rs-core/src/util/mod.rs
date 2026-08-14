//! Utility functions for feed parsing
//!
//! This module provides helper functions for common feed parsing tasks.

pub mod base_url;
pub mod date;
pub mod encoding;
pub mod sanitize;
/// Shared SSRF validation logic, reused by `http::validation` and `base_url`
pub(crate) mod ssrf;
/// Text processing utilities
pub mod text;

// Re-export commonly used functions
pub use base_url::{BaseUrlContext, combine_bases, is_safe_url, resolve_url};
pub use date::parse_date;
