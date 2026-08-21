//! Type aliases for Hasura/Postgres custom scalars exposed by the Hardcover schema.
#![allow(non_camel_case_types, dead_code)]
pub type numeric = f64;
pub type float8 = f64;
pub type bigint = i64;
pub type smallint = i16;
pub type citext = String;
pub type date = String;
pub type timestamp = String;
pub type timestamptz = String;
pub type json = serde_json::Value;
pub type jsonb = serde_json::Value;
