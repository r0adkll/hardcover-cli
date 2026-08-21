//! Typed client for the Hardcover.app API.
mod client;
mod collections;
mod error;
pub mod model;
mod queries;
mod scalars;
mod shows;

pub use client::{Client, ClientBuilder, RetryPolicy, DEFAULT_BASE_URL};
pub use collections::Page;
pub use error::{Error, Result};
