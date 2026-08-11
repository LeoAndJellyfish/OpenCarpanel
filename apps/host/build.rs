use std::{
    env,
    error::Error,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=../../web/dashboard/dist");
    println!("cargo:rerun-if-changed=src/dashboard-unavailable.html");

    let manifest_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let output_directory = PathBuf::from(env::var("OUT_DIR")?);
    let dashboard_directory = manifest_directory.join("../../web/dashboard/dist");
    let index_path = dashboard_directory.join("index.html");
    let profile = env::var("PROFILE")?;

    let assets = if index_path.is_file() {
        collect_assets(&dashboard_directory)?
    } else {
        if profile == "release" {
            return Err(io::Error::other(
                "release builds require dashboard assets; run `npm ci` and `npm run build:web` first",
            )
            .into());
        }
        println!(
            "cargo:warning=dashboard assets are absent; the development Host will serve a build instruction page"
        );
        vec![(
            "index.html".to_owned(),
            manifest_directory.join("src/dashboard-unavailable.html"),
        )]
    };

    let mut generated = String::from("pub(crate) static EMBEDDED_ASSETS: &[EmbeddedAsset] = &[\n");
    for (relative_path, source_path) in assets {
        let source_literal = source_path.to_string_lossy();
        writeln!(
            generated,
            "    EmbeddedAsset {{ path: {relative_path:?}, bytes: include_bytes!({source_literal:?}), content_type: {:?} }},",
            content_type(&relative_path)
        )?;
    }
    generated.push_str("];\n");
    fs::write(output_directory.join("embedded_dashboard.rs"), generated)?;
    Ok(())
}

fn collect_assets(root: &Path) -> Result<Vec<(String, PathBuf)>, Box<dyn Error>> {
    let mut pending = vec![root.to_path_buf()];
    let mut assets = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                assets.push((relative, path));
            }
        }
    }
    assets.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(assets)
}

fn content_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json" | "webmanifest") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}
