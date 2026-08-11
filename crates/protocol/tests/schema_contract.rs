use std::{error::Error, fs, path::Path};

use opencarpanel_protocol::generate_schema_documents;

#[test]
fn generated_schema_bundle_has_stable_paths_and_valid_json() -> Result<(), Box<dyn Error>> {
    let documents = generate_schema_documents()?;
    let paths = documents
        .iter()
        .map(opencarpanel_protocol::SchemaDocument::relative_path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        [
            "protocol/v1/client-message.schema.json",
            "protocol/v1/server-message.schema.json",
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
fn committed_schemas_match_the_generator() -> Result<(), Box<dyn Error>> {
    let schema_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");

    for document in generate_schema_documents()? {
        let committed = fs::read_to_string(schema_root.join(document.relative_path()))?;
        assert_eq!(committed, document.json(), "schema drift detected");
    }

    Ok(())
}
