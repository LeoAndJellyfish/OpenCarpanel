use schemars::schema_for;

use crate::LayoutDocument;

/// Generates the deterministic v1 layout JSON Schema.
///
/// # Errors
///
/// Returns a JSON serialization error if the schema cannot be encoded.
pub fn generate_layout_schema() -> Result<String, serde_json::Error> {
    let mut json = serde_json::to_string_pretty(&schema_for!(LayoutDocument))?;
    json.push('\n');
    Ok(json)
}
