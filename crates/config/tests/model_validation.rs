use std::{collections::BTreeMap, error::Error};

use opensimdash_config::{
    BreakpointName, ComponentType, GridPlacement, HostSettings, InstanceId, LayoutDocument,
    LayoutId, ThemeSettings, ValidationError, WidgetInstance,
};
use serde_json::json;

fn widget(number: usize) -> Result<WidgetInstance, ValidationError> {
    Ok(WidgetInstance {
        instance_id: InstanceId::new(format!("widget-{number}"))?,
        component_type: ComponentType::new("core.tachometer")?,
        placements: BTreeMap::from([(
            BreakpointName::PhoneLandscape,
            GridPlacement {
                x: 0,
                y: 0,
                width: 6,
                height: 4,
            },
        )]),
        settings: json!({"accent": "#3ee6a8"}),
    })
}

#[test]
fn identifiers_reject_path_traversal_but_component_types_allow_dots() {
    assert!(LayoutId::new("../escape").is_err());
    assert!(InstanceId::new("widget--one").is_err());
    assert!(ComponentType::new("core.tachometer").is_ok());
    assert!(ComponentType::new("core..tachometer").is_err());
}

#[test]
fn widget_count_grid_and_settings_depth_are_bounded() -> Result<(), Box<dyn Error>> {
    let mut layout = LayoutDocument::empty(LayoutId::new("bounded")?, "Bounded")?;
    let widgets = (0..65).map(widget).collect::<Result<Vec<_>, _>>()?;
    layout.set_widgets(widgets);
    assert!(matches!(
        layout.validate(),
        Err(ValidationError::TooManyWidgets { .. })
    ));

    let mut invalid_grid = widget(1)?;
    invalid_grid.placements.insert(
        BreakpointName::PhoneLandscape,
        GridPlacement {
            x: 23,
            y: 0,
            width: 2,
            height: 1,
        },
    );
    layout.set_widgets(vec![invalid_grid]);
    assert_eq!(
        layout.validate(),
        Err(ValidationError::InvalidGridPlacement)
    );

    let mut nested = json!(true);
    for _ in 0..10 {
        nested = json!({"next": nested});
    }
    let mut deep = widget(2)?;
    deep.settings = nested;
    layout.set_widgets(vec![deep]);
    assert!(matches!(
        layout.validate(),
        Err(ValidationError::SettingsTooDeep { .. })
    ));
    Ok(())
}

#[test]
fn host_settings_accept_only_publication_presets() {
    let mut settings = HostSettings::default();
    assert!(settings.validate().is_ok());
    settings.snapshot_hz = 59;
    assert_eq!(
        settings.validate(),
        Err(ValidationError::UnsupportedSnapshotRate(59))
    );

    let theme = ThemeSettings::default();
    assert!(!theme.background.is_empty());
}

#[test]
fn theme_tokens_reject_executable_css_strings() -> Result<(), Box<dyn Error>> {
    let mut layout = LayoutDocument::empty(LayoutId::new("theme-safe")?, "Safe theme")?;
    let theme = ThemeSettings {
        accent: "url(javascript:alert(1))".into(),
        ..ThemeSettings::default()
    };
    layout.set_theme(theme);

    assert!(matches!(
        layout.validate(),
        Err(ValidationError::InvalidColor { .. })
    ));
    Ok(())
}
