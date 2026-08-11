use std::{error::Error, fs, path::Path};

use opencarpanel_config::generate_layout_schema;

fn main() -> Result<(), Box<dyn Error>> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let destination = workspace_root.join("schemas/layout/v1/layout-document.schema.json");
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&destination, generate_layout_schema()?)?;
    println!("{}", destination.display());
    Ok(())
}
