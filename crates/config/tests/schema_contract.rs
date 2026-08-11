use std::{error::Error, fs, path::Path};

use opencarpanel_config::generate_layout_schema;

#[test]
fn committed_layout_schema_matches_rust_types() -> Result<(), Box<dyn Error>> {
    let generated = generate_layout_schema()?;
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/layout/v1/layout-document.schema.json");
    let committed = fs::read_to_string(path)?;

    assert_eq!(committed, generated);
    let value: serde_json::Value = serde_json::from_str(&generated)?;
    assert_eq!(
        value["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    Ok(())
}
