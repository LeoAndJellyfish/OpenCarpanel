use std::{
    collections::HashSet,
    env, fs,
    path::{Component, Path, PathBuf},
};

const MAX_VDF_BYTES: u64 = 1024 * 1024;
const MAX_VDF_DEPTH: usize = 32;
const MAX_VDF_TOKENS: usize = 65_536;

#[derive(Debug, Clone, Copy)]
struct GameDescriptor {
    app_id: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VdfValue {
    Text(String),
    Object(VdfObject),
}

type VdfObject = Vec<(String, VdfValue)>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum VdfToken {
    Text(String),
    Open,
    Close,
}

/// Finds Steam-managed installation roots for one supported SCS game.
///
/// Discovery is read-only and bounded. Invalid or unreadable Steam metadata is
/// ignored so the system directory picker remains a reliable fallback.
pub(crate) fn discover_game_directories(game: &str) -> Result<Vec<PathBuf>, String> {
    discover_game_directories_in_roots(game, &platform_steam_roots())
}

fn discover_game_directories_in_roots(
    game: &str,
    steam_roots: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let descriptor = game_descriptor(game)?;
    let libraries = discover_library_roots(steam_roots);
    let mut directories = Vec::new();
    let mut seen = HashSet::new();

    for library in libraries {
        let steamapps = library.join("steamapps");
        let manifest = steamapps.join(format!("appmanifest_{}.acf", descriptor.app_id));
        let Some(install_directory) = manifest_install_directory(&manifest, descriptor.app_id)
        else {
            continue;
        };
        let candidate = steamapps.join("common").join(install_directory);
        push_existing_directory(&mut directories, &mut seen, &candidate);
    }

    Ok(directories)
}

fn game_descriptor(game: &str) -> Result<GameDescriptor, String> {
    match game {
        "ets2" => Ok(GameDescriptor { app_id: "227300" }),
        "ats" => Ok(GameDescriptor { app_id: "270880" }),
        _ => Err("Steam 自动查找只支持 ets2 或 ats".to_owned()),
    }
}

fn discover_library_roots(steam_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    let mut seen = HashSet::new();

    for root in steam_roots {
        push_existing_directory(&mut libraries, &mut seen, root);
        let configuration = root.join("steamapps").join("libraryfolders.vdf");
        let Some(object) = read_vdf(&configuration) else {
            continue;
        };
        let entries = object_value(&object, "libraryfolders").unwrap_or(&object);
        for (index, value) in entries {
            if index.parse::<u32>().is_err() {
                continue;
            }
            let path = match value {
                VdfValue::Text(path) => Some(path.as_str()),
                VdfValue::Object(fields) => text_value(fields, "path"),
            };
            if let Some(path) = path.filter(|path| !path.trim().is_empty()) {
                push_existing_directory(&mut libraries, &mut seen, Path::new(path.trim()));
            }
        }
    }

    libraries
}

fn manifest_install_directory(path: &Path, expected_app_id: &str) -> Option<String> {
    let object = read_vdf(path)?;
    let app_state = object_value(&object, "AppState").unwrap_or(&object);
    let app_id = text_value(app_state, "appid")?;
    if app_id != expected_app_id {
        return None;
    }
    let install_directory = text_value(app_state, "installdir")?.trim();
    safe_install_directory(install_directory).then(|| install_directory.to_owned())
}

fn safe_install_directory(value: &str) -> bool {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
    {
        return false;
    }
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn read_vdf(path: &Path) -> Option<VdfObject> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_VDF_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let input = String::from_utf8(bytes).ok()?;
    parse_vdf(&input).ok()
}

fn parse_vdf(input: &str) -> Result<VdfObject, &'static str> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let tokens = tokenize_vdf(input)?;
    let mut cursor = 0;
    let object = parse_vdf_object(&tokens, &mut cursor, false, 0)?;
    if cursor == tokens.len() {
        Ok(object)
    } else {
        Err("VDF contains trailing tokens")
    }
}

fn tokenize_vdf(input: &str) -> Result<Vec<VdfToken>, &'static str> {
    let mut tokens = Vec::new();
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            character if character.is_whitespace() => {}
            '/' if characters.peek() == Some(&'/') => {
                characters.next();
                for character in characters.by_ref() {
                    if character == '\n' {
                        break;
                    }
                }
            }
            '{' => tokens.push(VdfToken::Open),
            '}' => tokens.push(VdfToken::Close),
            '"' => {
                let mut value = String::new();
                let mut closed = false;
                while let Some(character) = characters.next() {
                    match character {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => match characters.next() {
                            Some('\\') => value.push('\\'),
                            Some('"') => value.push('"'),
                            Some(escaped) => {
                                value.push('\\');
                                value.push(escaped);
                            }
                            None => return Err("VDF string ends with an escape"),
                        },
                        value_character => value.push(value_character),
                    }
                }
                if !closed {
                    return Err("VDF string is not terminated");
                }
                tokens.push(VdfToken::Text(value));
            }
            first => {
                let mut value = String::from(first);
                while let Some(next) = characters.peek() {
                    if next.is_whitespace() || matches!(next, '{' | '}') {
                        break;
                    }
                    if let Some(next) = characters.next() {
                        value.push(next);
                    }
                }
                tokens.push(VdfToken::Text(value));
            }
        }
        if tokens.len() > MAX_VDF_TOKENS {
            return Err("VDF contains too many tokens");
        }
    }

    Ok(tokens)
}

fn parse_vdf_object(
    tokens: &[VdfToken],
    cursor: &mut usize,
    expects_close: bool,
    depth: usize,
) -> Result<VdfObject, &'static str> {
    if depth > MAX_VDF_DEPTH {
        return Err("VDF nesting is too deep");
    }
    let mut object = Vec::new();

    while let Some(token) = tokens.get(*cursor) {
        if token == &VdfToken::Close {
            if !expects_close {
                return Err("VDF contains an unexpected closing brace");
            }
            *cursor += 1;
            return Ok(object);
        }
        let VdfToken::Text(key) = token else {
            return Err("VDF key is not text");
        };
        *cursor += 1;
        let Some(value) = tokens.get(*cursor) else {
            return Err("VDF key has no value");
        };
        match value {
            VdfToken::Text(value) => {
                object.push((key.clone(), VdfValue::Text(value.clone())));
                *cursor += 1;
            }
            VdfToken::Open => {
                *cursor += 1;
                let nested = parse_vdf_object(tokens, cursor, true, depth + 1)?;
                object.push((key.clone(), VdfValue::Object(nested)));
            }
            VdfToken::Close => return Err("VDF key has no value"),
        }
    }

    if expects_close {
        Err("VDF object is not terminated")
    } else {
        Ok(object)
    }
}

fn object_value<'a>(object: &'a VdfObject, key: &str) -> Option<&'a VdfObject> {
    object.iter().find_map(|(candidate, value)| {
        if candidate.eq_ignore_ascii_case(key) {
            match value {
                VdfValue::Object(value) => Some(value),
                VdfValue::Text(_) => None,
            }
        } else {
            None
        }
    })
}

fn text_value<'a>(object: &'a VdfObject, key: &str) -> Option<&'a str> {
    object.iter().find_map(|(candidate, value)| {
        if candidate.eq_ignore_ascii_case(key) {
            match value {
                VdfValue::Text(value) => Some(value.as_str()),
                VdfValue::Object(_) => None,
            }
        } else {
            None
        }
    })
}

fn push_existing_directory(
    directories: &mut Vec<PathBuf>,
    seen: &mut HashSet<String>,
    path: &Path,
) {
    let Ok(canonical) = path.canonicalize() else {
        return;
    };
    if !canonical.is_dir() {
        return;
    }
    let identity = path_identity(&canonical);
    if seen.insert(identity) {
        directories.push(canonical);
    }
}

#[cfg(target_os = "windows")]
fn path_identity(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

#[cfg(not(target_os = "windows"))]
fn path_identity(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(target_os = "windows")]
fn platform_steam_roots() -> Vec<PathBuf> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER, enums::HKEY_LOCAL_MACHINE};

    let mut roots = Vec::new();
    let registry_locations = [
        (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Valve\Steam",
            "InstallPath",
        ),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
    ];
    for (hive, key, value_name) in registry_locations {
        let Ok(key) = RegKey::predef(hive).open_subkey(key) else {
            continue;
        };
        if let Ok(value) = key.get_value::<String, _>(value_name) {
            roots.push(PathBuf::from(value));
        }
    }
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files).join("Steam"));
    }
    if let Some(system_drive) = env::var_os("SystemDrive") {
        let drive = PathBuf::from(system_drive);
        roots.push(drive.join("Program Files (x86)").join("Steam"));
        roots.push(drive.join("Program Files").join("Steam"));
    }
    roots
}

#[cfg(target_os = "macos")]
fn platform_steam_roots() -> Vec<PathBuf> {
    env::var_os("HOME").map_or_else(Vec::new, |home| {
        vec![
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Steam"),
        ]
    })
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_steam_roots() -> Vec<PathBuf> {
    env::var_os("HOME").map_or_else(Vec::new, |home| {
        let home = PathBuf::from(home);
        vec![
            home.join(".local").join("share").join("Steam"),
            home.join(".steam").join("steam"),
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join("data")
                .join("Steam"),
        ]
    })
}

#[cfg(test)]
mod tests {
    use std::{error::Error, io::Write as _};

    use super::*;

    #[test]
    fn finds_the_requested_app_in_a_secondary_library() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let steam = temp.path().join("Steam");
        let secondary = temp.path().join("Driving Games");
        fs::create_dir_all(steam.join("steamapps"))?;
        fs::create_dir_all(secondary.join("steamapps").join("common"))?;
        let configuration = format!(
            "\"libraryfolders\"\n{{\n  \"0\" {{ \"path\" \"{}\" }}\n  \"1\" {{ \"path\" \"{}\" \"apps\" {{ \"227300\" \"1\" }} }}\n}}",
            vdf_path(&steam),
            vdf_path(&secondary),
        );
        fs::write(
            steam.join("steamapps").join("libraryfolders.vdf"),
            configuration,
        )?;
        write_manifest(&secondary, "227300", "Euro Truck Simulator 2")?;
        let game = secondary
            .join("steamapps")
            .join("common")
            .join("Euro Truck Simulator 2");
        fs::create_dir_all(&game)?;

        let found = discover_game_directories_in_roots("ets2", &[steam])?;
        assert_eq!(found, vec![game.canonicalize()?]);
        Ok(())
    }

    #[test]
    fn supports_the_legacy_libraryfolders_shape() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let steam = temp.path().join("Steam");
        let secondary = temp.path().join("Legacy Library");
        fs::create_dir_all(steam.join("steamapps"))?;
        fs::create_dir_all(secondary.join("steamapps").join("common"))?;
        let configuration = format!(
            "\"LibraryFolders\" {{ \"1\" \"{}\" }}",
            vdf_path(&secondary),
        );
        fs::write(
            steam.join("steamapps").join("libraryfolders.vdf"),
            configuration,
        )?;
        write_manifest(&secondary, "270880", "American Truck Simulator")?;
        let game = secondary
            .join("steamapps")
            .join("common")
            .join("American Truck Simulator");
        fs::create_dir_all(&game)?;

        let found = discover_game_directories_in_roots("ats", &[steam])?;
        assert_eq!(found, vec![game.canonicalize()?]);
        Ok(())
    }

    #[test]
    fn rejects_a_manifest_with_the_wrong_app_id() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let steam = temp.path().join("Steam");
        fs::create_dir_all(steam.join("steamapps").join("common"))?;
        write_manifest(&steam, "270880", "Euro Truck Simulator 2")?;
        fs::rename(
            steam.join("steamapps").join("appmanifest_270880.acf"),
            steam.join("steamapps").join("appmanifest_227300.acf"),
        )?;

        let found = discover_game_directories_in_roots("ets2", &[steam])?;
        assert!(found.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_a_manifest_install_directory_with_path_components() -> Result<(), Box<dyn Error>> {
        let temp = tempfile::tempdir()?;
        let steam = temp.path().join("Steam");
        fs::create_dir_all(steam.join("steamapps").join("common"))?;
        write_manifest(&steam, "227300", "..\\escape")?;

        let found = discover_game_directories_in_roots("ets2", &[steam])?;
        assert!(found.is_empty());
        Ok(())
    }

    #[test]
    fn parses_comments_and_escaped_backslashes() -> Result<(), Box<dyn Error>> {
        let object = parse_vdf(
            "\u{feff}// Steam library\n\"libraryfolders\" { \"0\" { \"path\" \"C:\\\\Steam\" } }",
        )?;
        let folders = object_value(&object, "libraryfolders").ok_or("missing libraryfolders")?;
        let first = object_value(folders, "0").ok_or("missing first library")?;
        assert_eq!(text_value(first, "path"), Some(r"C:\Steam"));
        Ok(())
    }

    fn write_manifest(
        library: &Path,
        app_id: &str,
        install_directory: &str,
    ) -> Result<(), Box<dyn Error>> {
        let steamapps = library.join("steamapps");
        fs::create_dir_all(&steamapps)?;
        let path = steamapps.join(format!("appmanifest_{app_id}.acf"));
        let mut file = fs::File::create(path)?;
        write!(
            file,
            "\"AppState\" {{ \"appid\" \"{app_id}\" \"installdir\" \"{install_directory}\" }}"
        )?;
        Ok(())
    }

    fn vdf_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "\\\\")
    }
}
