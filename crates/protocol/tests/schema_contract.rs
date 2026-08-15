use std::{error::Error, fs, path::Path};

use opensimdash_protocol::generate_schema_documents;

#[test]
fn generated_schema_bundle_has_stable_paths_and_valid_json() -> Result<(), Box<dyn Error>> {
    let documents = generate_schema_documents()?;
    let paths = documents
        .iter()
        .map(opensimdash_protocol::SchemaDocument::relative_path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        [
            "protocol/v1/client-message.schema.json",
            "protocol/v1/server-message.schema.json",
            "game-plugin/v1/manifest.schema.json",
            "game-plugin/v1/package.schema.json",
            "telemetry/v1/telemetry-event.schema.json",
            "telemetry/v1/telemetry-snapshot.schema.json",
            "telemetry/v1/telemetry-update.schema.json",
        ]
    );

    for document in documents {
        let value: serde_json::Value = serde_json::from_str(document.json())?;
        assert_eq!(
            value["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(document.json().ends_with('\n'));
    }

    Ok(())
}

#[test]
fn wire_schemas_pin_the_supported_protocol_version() -> Result<(), Box<dyn Error>> {
    for document in generate_schema_documents()?
        .into_iter()
        .filter(|document| document.relative_path().starts_with("protocol/"))
    {
        let value: serde_json::Value = serde_json::from_str(document.json())?;
        assert_eq!(value["properties"]["v"]["const"], 1);
    }

    Ok(())
}

#[test]
fn committed_schemas_match_the_generator() -> Result<(), Box<dyn Error>> {
    let schema_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");

    for document in generate_schema_documents()? {
        let committed = fs::read_to_string(schema_root.join(document.relative_path()))?;
        assert_eq!(committed, document.json(), "schema drift detected");
    }

    Ok(())
}
