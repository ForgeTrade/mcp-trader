//! Configuration Management
//!
//! This module handles loading and managing configuration including API credentials.

pub mod credentials;

#[cfg(feature = "http-api")]
pub mod http;

// Re-export
pub use credentials::Credentials;

#[cfg(feature = "http-api")]
pub use http::HttpConfig;
