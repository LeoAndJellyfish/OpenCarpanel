use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
};

use opencarpanel_game_plugin_api::{GamePluginManifest, GamePluginPackage};
use opencarpanel_telemetry_core::{TelemetryEvent, TelemetrySnapshot, TelemetryUpdate};
use schemars::{Schema, schema_for};

use crate::{ClientMessage, ServerMessage};

/// One deterministic JSON Schema file generated from the Rust wire types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDocument {
    relative_path: &'static str,
    json: String,
}

impl SchemaDocument {
    /// Returns the path relative to the repository's `schemas` directory.
    #[must_use]
    pub const fn relative_path(&self) -> &'static str {
        self.relative_path
    }

    /// Returns pretty-printed JSON terminated by one newline.
    #[must_use]
    pub fn json(&self) -> &str {
        &self.json
    }
}

/// Builds every committed schema in a deterministic path order.
///
/// # Errors
///
/// Returns a serialization error if a generated schema cannot be represented
/// as JSON.
pub fn generate_schema_documents() -> Result<Vec<SchemaDocument>, serde_json::Error> {
    [
        (
            "protocol/v1/client-message.schema.json",
            schema_for!(ClientMessage),
        ),
        (
            "protocol/v1/server-message.schema.json",
            schema_for!(ServerMessage),
        ),
        (
            "game-plugin/v1/manifest.schema.json",
            schema_for!(GamePluginManifest),
        ),
        (
            "game-plugin/v1/package.schema.json",
            schema_for!(GamePluginPackage),
        ),
        (
            "telemetry/v1/telemetry-event.schema.json",
            schema_for!(TelemetryEvent),
        ),
        (
            "telemetry/v1/telemetry-snapshot.schema.json",
            schema_for!(TelemetrySnapshot),
        ),
        (
            "telemetry/v1/telemetry-update.schema.json",
            schema_for!(TelemetryUpdate),
        ),
    ]
    .into_iter()
    .map(|(relative_path, schema)| schema_document(relative_path, &schema))
    .collect()
}

fn schema_document(
    relative_path: &'static str,
    schema: &Schema,
) -> Result<SchemaDocument, serde_json::Error> {
    let mut json = serde_json::to_string_pretty(schema)?;
    json.push('\n');
    Ok(SchemaDocument {
        relative_path,
        json,
    })
}

/// Failure while generating or writing the schema bundle.
#[derive(Debug)]
#[non_exhaustive]
pub enum SchemaExportError {
    /// A generated schema could not be serialized.
    Generate(serde_json::Error),
    /// A schema directory or file could not be written.
    Io {
        /// Destination involved in the failed operation.
        path: PathBuf,
        /// Underlying operating-system error.
        source: io::Error,
    },
}

impl Display for SchemaExportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generate(error) => write!(formatter, "failed to generate JSON Schema: {error}"),
            Self::Io { path, source } => write!(
                formatter,
                "failed to write schema path {}: {source}",
                path.display()
            ),
        }
    }
}

impl Error for SchemaExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Generate(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

/// Writes the deterministic schema bundle below `schema_root`.
///
/// # Errors
///
/// Returns [`SchemaExportError`] when generation, directory creation, or file
/// writing fails.
pub fn write_schema_documents(schema_root: &Path) -> Result<Vec<PathBuf>, SchemaExportError> {
    let documents = generate_schema_documents().map_err(SchemaExportError::Generate)?;
    let mut written = Vec::with_capacity(documents.len());

    for document in documents {
        let destination = schema_root.join(document.relative_path());
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|source| SchemaExportError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&destination, document.json()).map_err(|source| SchemaExportError::Io {
            path: destination.clone(),
            source,
        })?;
        written.push(destination);
    }

    Ok(written)
}
