use std::{error::Error, path::Path};

use opencarpanel_protocol::write_schema_documents;

fn main() -> Result<(), Box<dyn Error>> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema_root = workspace_root.join("schemas");

    for path in write_schema_documents(&schema_root)? {
        println!("{}", path.display());
    }

    Ok(())
}
