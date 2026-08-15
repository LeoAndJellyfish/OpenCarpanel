use std::{error::Error, fs};

use opensimdash_config::{
    ConfigError, LayoutDocument, LayoutId, LayoutRepository, migrate_layout_json,
};

fn document() -> Result<LayoutDocument, ConfigError> {
    Ok(LayoutDocument::empty(
        LayoutId::new("default")?,
        "Default dashboard",
    )?)
}

#[test]
fn saving_and_loading_preserves_schema_and_increments_revision() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = LayoutRepository::new(temp.path());

    let saved = repository.save(&document()?, 0)?;
    let loaded = repository.load_required(saved.id())?;

    assert_eq!(saved.schema_version(), 1);
    assert_eq!(saved.revision(), 1);
    assert_eq!(loaded.document, saved);
    assert!(!loaded.recovered);
    Ok(())
}

#[test]
fn stale_revision_returns_a_conflict() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = LayoutRepository::new(temp.path());
    let saved = repository.save(&document()?, 0)?;

    let error = repository
        .save(&saved, 0)
        .err()
        .ok_or("stale save unexpectedly succeeded")?;

    assert!(matches!(
        error,
        ConfigError::Conflict {
            current_revision: 1
        }
    ));
    Ok(())
}

#[test]
fn corrupt_primary_is_quarantined_and_last_known_good_is_restored() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = LayoutRepository::new(temp.path());
    let saved = repository.save(&document()?, 0)?;
    fs::write(repository.layout_path(saved.id()), b"{ definitely not JSON")?;

    let loaded = repository.load_required(saved.id())?;

    assert_eq!(loaded.document, saved);
    assert!(loaded.recovered);
    let quarantine = loaded
        .quarantined_path
        .ok_or("corrupt primary was not quarantined")?;
    assert!(quarantine.exists());
    assert!(repository.layout_path(saved.id()).exists());
    Ok(())
}

#[test]
fn failed_validation_does_not_modify_the_existing_file() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = LayoutRepository::new(temp.path());
    let saved = repository.save(&document()?, 0)?;
    let before = fs::read(repository.layout_path(saved.id()))?;

    let mut invalid = saved.clone();
    invalid.set_name("x".repeat(300));
    assert!(repository.save(&invalid, saved.revision()).is_err());

    let after = fs::read(repository.layout_path(saved.id()))?;
    assert_eq!(after, before);
    assert_eq!(repository.load_required(saved.id())?.document, saved);
    Ok(())
}

#[test]
fn v0_layout_migrates_deterministically_to_v1() -> Result<(), Box<dyn Error>> {
    let fixture = br#"{
        "id": "legacy",
        "name": "Legacy",
        "widgets": []
    }"#;

    let first = migrate_layout_json(fixture)?;
    let second = migrate_layout_json(fixture)?;

    assert_eq!(first, second);
    assert_eq!(first.schema_version(), 1);
    assert_eq!(first.revision(), 0);
    assert_eq!(first.id().as_str(), "legacy");
    assert!(first.widgets().is_empty());
    Ok(())
}

#[test]
fn backup_history_is_bounded() -> Result<(), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let repository = LayoutRepository::new(temp.path());
    let mut current = document()?;
    for revision in 0..5 {
        current.set_name(format!("Dashboard {revision}"));
        current = repository.save(&current, current.revision())?;
    }

    let backup_directory = temp.path().join("backups/default");
    let backup_count = fs::read_dir(backup_directory)?
        .collect::<Result<Vec<_>, _>>()?
        .len();
    assert_eq!(backup_count, 3);
    Ok(())
}
