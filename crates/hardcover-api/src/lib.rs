//! Typed client for the Hardcover.app API.
mod client;
mod collections;
mod shows;
mod error;
pub mod model;
mod queries;
mod scalars;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL};
pub use collections::Page;
pub use error::{Error, Result};
