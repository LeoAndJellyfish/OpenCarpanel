#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = opencarpanel_desktop_lib::run() {
        eprintln!("OpenCarpanel desktop failed: {error}");
        std::process::exit(1);
    }
}
