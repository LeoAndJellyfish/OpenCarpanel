use serde_json::{Map, Value};

use crate::{
    CONFIG_SCHEMA_VERSION, ConfigError, LayoutDocument, MAX_LAYOUT_BYTES, ValidationError,
};

/// Parses, migrates, and validates a layout document.
///
/// A missing `schemaVersion` is the deterministic v0 format.
///
/// # Errors
///
/// Returns [`ConfigError`] for oversized JSON, invalid syntax, unsupported
/// versions, migration failures, or invalid v1 values.
pub fn migrate_layout_json(bytes: &[u8]) -> Result<LayoutDocument, ConfigError> {
    if bytes.len() > MAX_LAYOUT_BYTES {
        return Err(ConfigError::DocumentTooLarge {
            actual: bytes.len(),
            maximum: MAX_LAYOUT_BYTES,
        });
    }

    let mut value: Value = serde_json::from_slice(bytes).map_err(ConfigError::Json)?;
    let version = schema_version(&value)?;
    match version {
        0 => migrate_v0_to_v1(&mut value)?,
        CONFIG_SCHEMA_VERSION => {}
        actual => return Err(ConfigError::UnsupportedSchema { actual }),
    }

    let document: LayoutDocument = serde_json::from_value(value).map_err(ConfigError::Json)?;
    document.validate().map_err(ConfigError::Validation)?;
    Ok(document)
}

fn schema_version(value: &Value) -> Result<u16, ConfigError> {
    let Some(raw) = value.get("schemaVersion") else {
        return Ok(0);
    };
    let number = raw.as_u64().ok_or(ConfigError::InvalidSchemaVersion)?;
    u16::try_from(number).map_err(|_| ConfigError::InvalidSchemaVersion)
}

fn migrate_v0_to_v1(value: &mut Value) -> Result<(), ConfigError> {
    let object = value
        .as_object_mut()
        .ok_or(ConfigError::Migration("v0 layout must be an object"))?;
    insert_number(object, "schemaVersion", u64::from(CONFIG_SCHEMA_VERSION));
    insert_number(object, "revision", 0);
    object
        .entry("widgets")
        .or_insert_with(|| Value::Array(Vec::new()));
    object.entry("theme").or_insert_with(|| {
        serde_json::json!({
            "background": "#07090f",
            "foreground": "#f5f7fb",
            "accent": "#3ee6a8",
            "warning": "#ff4d5e"
        })
    });
    Ok(())
}

fn insert_number(object: &mut Map<String, Value>, key: &str, value: u64) {
    object.insert(key.to_owned(), Value::Number(value.into()));
}

impl From<ValidationError> for ConfigError {
    fn from(error: ValidationError) -> Self {
        Self::Validation(error)
    }
}
