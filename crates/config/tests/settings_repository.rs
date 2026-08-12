use std::{error::Error, fs};

use opencarpanel_config::{AppSettings, SettingsRepository};

#[test]
fn settings_are_created_saved_and_loaded_atomically() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = SettingsRepository::new(temp.path());
    let loaded = repository.load()?;
    assert_eq!(loaded.settings, AppSettings::default());
    assert!(!loaded.recovered);

    let mut changed = loaded.settings;
    changed.host.http_bind = "127.0.0.1:31000".into();
    changed.desktop.launch_at_login = true;
    repository.save(&changed)?;

    assert_eq!(repository.load()?.settings, changed);
    Ok(())
}

#[test]
fn invalid_primary_recovers_the_latest_valid_backup() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = SettingsRepository::new(temp.path());
    let mut first = AppSettings::default();
    first.host.snapshot_hz = 30;
    repository.save(&first)?;
    let mut latest = first;
    latest.host.snapshot_hz = 20;
    repository.save(&latest)?;
    fs::write(repository.settings_path(), b"{broken")?;

    let loaded = repository.load()?;
    assert!(loaded.recovered);
    assert!(!loaded.reset_to_defaults);
    assert_eq!(loaded.settings, latest);
    assert!(loaded.quarantined_path.is_some_and(|path| path.exists()));
    Ok(())
}

#[test]
fn invalid_primary_without_backup_installs_safe_defaults() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = SettingsRepository::new(temp.path());
    fs::write(repository.settings_path(), b"[]")?;

    let loaded = repository.load()?;
    assert!(loaded.reset_to_defaults);
    assert_eq!(loaded.settings, AppSettings::default());
    assert_eq!(repository.load()?.settings, AppSettings::default());
    Ok(())
}

#[test]
fn invalid_settings_never_replace_the_primary() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = SettingsRepository::new(temp.path());
    let original = repository.load()?.settings;
    let before = fs::read(repository.settings_path())?;
    let mut invalid = original;
    invalid.host.udp_bind = "not-a-socket".into();

    assert!(repository.save(&invalid).is_err());
    assert_eq!(fs::read(repository.settings_path())?, before);
    Ok(())
}
