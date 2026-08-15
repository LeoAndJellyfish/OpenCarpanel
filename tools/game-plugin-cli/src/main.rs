use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use opensimdash_game_plugin_api::{
    GAME_PLUGIN_ABI_VERSION, GAME_PLUGIN_PACKAGE_VERSION, GamePluginPackage,
    MAX_PLUGIN_MODULE_BYTES, PluginRuntime, parse_manifest,
};
use opensimdash_game_plugin_runtime::{
    MAX_PLUGIN_PACKAGE_BYTES, ensure_plugin_package_extension, verify_package,
};
use sha2::{Digest as _, Sha256};

const MAX_MANIFEST_BYTES: usize = 64 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("plugin command failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, manifest, module, output] if command == "pack" => {
            pack(Path::new(manifest), Path::new(module), Path::new(output))
        }
        [command, package] if command == "validate" => validate(Path::new(package)),
        _ => Err(
            "usage: opensimdash-plugin pack <manifest.json> <decoder.wasm> <output.osd-plugin>\n       opensimdash-plugin validate <plugin.osd-plugin>"
                .into(),
        ),
    }
}

fn pack(manifest_path: &Path, module_path: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    ensure_plugin_package_extension(output)?;
    let manifest_bytes = read_bounded(manifest_path, MAX_MANIFEST_BYTES, "manifest")?;
    let mut manifest = parse_manifest(&manifest_bytes)?;
    let module = read_bounded(module_path, MAX_PLUGIN_MODULE_BYTES, "decoder module")?;
    if module.is_empty() {
        return Err("decoder module is empty".into());
    }
    let module_name = module_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("decoder module filename is not UTF-8")?
        .to_owned();
    manifest.runtime = PluginRuntime::Wasm {
        abi_version: GAME_PLUGIN_ABI_VERSION,
        module: module_name,
        sha256: format!("{:x}", Sha256::digest(&module)),
    };
    manifest.validate()?;
    let package = GamePluginPackage {
        package_version: GAME_PLUGIN_PACKAGE_VERSION,
        manifest,
        module_base64: STANDARD.encode(module),
    };
    let bytes = serde_json::to_vec_pretty(&package)?;
    verify_package(&bytes)?;
    let mut terminated = bytes;
    terminated.push(b'\n');
    fs::write(output, terminated)?;
    println!("created {}", display_path(output));
    Ok(())
}

fn validate(package: &Path) -> Result<(), Box<dyn Error>> {
    ensure_plugin_package_extension(package)?;
    let maximum = usize::try_from(MAX_PLUGIN_PACKAGE_BYTES).unwrap_or(usize::MAX);
    let bytes = read_bounded(package, maximum, "plugin package")?;
    let verified = verify_package(&bytes)?;
    opensimdash_game_plugin_runtime::WasmGameAdapter::from_bytes(
        &verified.manifest,
        &verified.module,
    )?;
    println!(
        "valid {} {} by {}",
        verified.manifest.id, verified.manifest.version, verified.manifest.publisher
    );
    Ok(())
}

fn read_bounded(path: &Path, maximum: usize, kind: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > u64::try_from(maximum).unwrap_or(u64::MAX) {
        return Err(format!("{kind} must be a regular file no larger than {maximum} bytes").into());
    }
    let bytes = fs::read(path)?;
    if bytes.len() > maximum {
        return Err(format!("{kind} grew beyond {maximum} bytes while being read").into());
    }
    Ok(bytes)
}

fn display_path(path: &Path) -> String {
    PathBuf::from(path).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_and_validates_the_example_manifest() -> Result<(), Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let module_path = temporary.path().join("decoder.wasm");
        let package_path = temporary.path().join("example-sim.osd-plugin");
        let module = wat::parse_str(
            r#"(module
              (memory (export "memory") 6 64)
              (func (export "osd_plugin_abi_version") (result i32) i32.const 1)
              (func (export "osd_input_ptr") (result i32) i32.const 0)
              (func (export "osd_input_capacity") (result i32) i32.const 65536)
              (func (export "osd_output_ptr") (result i32) i32.const 65536)
              (func (export "osd_output_capacity") (result i32) i32.const 262144)
              (func (export "osd_decode") (param i32 i64) (result i32) i32.const 0)
            )"#,
        )?;
        fs::write(&module_path, module)?;
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/game-plugin-rust/manifest.json");
        pack(&manifest_path, &module_path, &package_path)?;
        validate(&package_path)?;

        let verified = verify_package(&fs::read(package_path)?)?;
        assert_eq!(verified.manifest.id, "example-sim");
        assert!(matches!(
            verified.manifest.runtime,
            PluginRuntime::Wasm { abi_version: 1, .. }
        ));
        Ok(())
    }

    #[test]
    fn pack_rejects_a_noncanonical_output_extension() -> Result<(), Box<dyn Error>> {
        let temporary = tempfile::tempdir()?;
        let module_path = temporary.path().join("decoder.wasm");
        fs::write(&module_path, b"not-needed")?;
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/game-plugin-rust/manifest.json");

        assert!(
            pack(
                &manifest_path,
                &module_path,
                &temporary.path().join("example.plugin"),
            )
            .is_err()
        );
        Ok(())
    }
}
