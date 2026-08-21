//! `--format table`: aligned text tables for humans, derived from the same serialized
//! JSON the `json` format emits, so the two can never disagree about field names.
use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
use serde_json::Value;

const MAX_CELL: usize = 60;

/// A collection: one row per item, columns from the union of top-level keys (first-seen
/// order), minus columns that are empty everywhere.
pub fn render_rows(items: &[Value]) -> String {
    if items.is_empty() {
        return "(no rows)".into();
    }
    let mut columns: Vec<String> = Vec::new();
    for item in items {
        if let Some(obj) = item.as_object() {
            for k in obj.keys() {
                if !columns.contains(k) {
                    columns.push(k.clone());
                }
            }
        }
    }
    let mut table = base_table();
    table.set_header(&columns);
    for item in items {
        table.add_row(
            columns
                .iter()
                .map(|c| cell(item.get(c).unwrap_or(&Value::Null))),
        );
    }
    table.to_string()
}

/// A single entity: `field | value` rows.
pub fn render_entity(value: &Value) -> String {
    let mut table = base_table();
    table.set_header(["field", "value"]);
    match value.as_object() {
        Some(obj) => {
            for (k, v) in obj {
                table.add_row([k.clone(), cell(v)]);
            }
        }
        None => {
            table.add_row(["value".to_string(), cell(value)]);
        }
    }
    table.to_string()
}

fn base_table() -> Table {
    let mut t = Table::new();
    t.load_preset(UTF8_FULL_CONDENSED);
    t.set_content_arrangement(ContentArrangement::Dynamic);
    t
}

/// Scalars print as-is; objects collapse to their most identifying field; arrays join their
/// collapsed elements. Long text is truncated — `json` exists for the full thing.
fn cell(v: &Value) -> String {
    let s = match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(items) => items.iter().map(summarize).collect::<Vec<_>>().join(", "),
        Value::Object(_) => summarize(v),
    };
    truncate(&s.replace('\n', " "))
}

fn summarize(v: &Value) -> String {
    match v {
        Value::Object(o) => [
            "title", "name", "label", "username", "question", "slug", "id",
        ]
        .iter()
        .find_map(|k| o.get(*k))
        .map(|x| match x {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| format!("{{{} fields}}", o.len())),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn truncate(s: &str) -> String {
    if s.chars().count() <= MAX_CELL {
        s.to_string()
    } else {
        let cut: String = s.chars().take(MAX_CELL - 1).collect();
        format!("{cut}…")
    }
}
