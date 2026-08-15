#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = opensimdash_desktop_lib::run() {
        eprintln!("OpenSimDash desktop failed: {error}");
        std::process::exit(1);
    }
}
