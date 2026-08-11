use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current layout and settings schema version.
pub const CONFIG_SCHEMA_VERSION: u16 = 1;

/// Maximum serialized bytes accepted for one layout document.
pub const MAX_LAYOUT_BYTES: usize = 256 * 1024;

/// Maximum number of widgets accepted in one layout.
pub const MAX_WIDGETS: usize = 64;

const MAX_IDENTIFIER_LEN: usize = 96;
const MAX_STRING_LEN: usize = 256;
const MAX_SETTINGS_DEPTH: usize = 8;
const MAX_SETTINGS_ENTRIES: usize = 256;
const MAX_GRID_COLUMNS: u16 = 24;
const MAX_GRID_ROWS: u16 = 1_000;

macro_rules! identifier_type {
    ($name:ident, $kind:literal) => {
        #[doc = concat!("Validated ", $kind, " identifier.")]
        #[derive(
            Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
        )]
        #[serde(transparent)]
        #[schemars(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a validated ", $kind, " identifier.")]
            ///
            /// # Errors
            ///
            /// Returns [`ValidationError`] when the value is not a safe stable slug.
            pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
                let value = value.into();
                validate_identifier($kind, &value)?;
                Ok(Self(value))
            }

            /// Returns the stable identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier_type!(LayoutId, "layout");
identifier_type!(InstanceId, "widget instance");

/// Validated dotted built-in component type.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct ComponentType(String);

impl ComponentType {
    /// Creates a component type such as `core.tachometer`.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] when any dotted segment is not a safe slug.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        validate_component_type(&value)?;
        Ok(Self(value))
    }

    /// Returns the stable dotted component type.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Responsive dashboard breakpoint.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum BreakpointName {
    /// Narrow portrait phone layout.
    PhonePortrait,
    /// Wide phone layout.
    PhoneLandscape,
    /// Tablet-sized layout.
    Tablet,
    /// Desktop preview layout.
    Desktop,
}

/// Integer geometry for one widget at one breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GridPlacement {
    /// Zero-based grid column.
    pub x: u16,
    /// Zero-based grid row.
    pub y: u16,
    /// Width in grid columns.
    pub width: u16,
    /// Height in grid rows.
    pub height: u16,
}

/// One built-in widget instance in a layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WidgetInstance {
    /// Stable instance id used by editor operations.
    pub instance_id: InstanceId,
    /// Registered built-in component type, such as `core.tachometer`.
    pub component_type: ComponentType,
    /// Breakpoint-specific geometry.
    #[serde(default)]
    pub placements: BTreeMap<BreakpointName, GridPlacement>,
    /// Component settings validated again by the component-specific schema.
    #[serde(default)]
    pub settings: Value,
}

/// Safe theme tokens exposed to built-in widgets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ThemeSettings {
    /// Dashboard background CSS color token.
    pub background: String,
    /// Primary foreground CSS color token.
    pub foreground: String,
    /// Accent CSS color token.
    pub accent: String,
    /// Warning CSS color token.
    pub warning: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            background: "#07090f".into(),
            foreground: "#f5f7fb".into(),
            accent: "#3ee6a8".into(),
            warning: "#ff4d5e".into(),
        }
    }
}

/// Versioned responsive dashboard document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LayoutDocument {
    schema_version: u16,
    revision: u64,
    id: LayoutId,
    name: String,
    #[serde(default)]
    widgets: Vec<WidgetInstance>,
    #[serde(default)]
    theme: ThemeSettings,
}

impl LayoutDocument {
    /// Creates an empty v1 dashboard document.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] when the name or identifier is invalid.
    pub fn empty(id: LayoutId, name: impl Into<String>) -> Result<Self, ValidationError> {
        let document = Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            revision: 0,
            id,
            name: name.into(),
            widgets: Vec::new(),
            theme: ThemeSettings::default(),
        };
        document.validate()?;
        Ok(document)
    }

    /// Returns the document schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Returns the optimistic concurrency revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the stable layout id.
    #[must_use]
    pub const fn id(&self) -> &LayoutId {
        &self.id
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns widget instances in persisted order.
    #[must_use]
    pub fn widgets(&self) -> &[WidgetInstance] {
        &self.widgets
    }

    /// Returns the safe theme token set.
    #[must_use]
    pub const fn theme(&self) -> &ThemeSettings {
        &self.theme
    }

    /// Replaces the display name. Repository validation still runs before save.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Replaces the widget list. Repository validation still runs before save.
    pub fn set_widgets(&mut self, widgets: Vec<WidgetInstance>) {
        self.widgets = widgets;
    }

    /// Replaces the theme. Repository validation still runs before save.
    pub fn set_theme(&mut self, theme: ThemeSettings) {
        self.theme = theme;
    }

    /// Validates all import and persistence bounds.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] for unsafe identifiers, excessive size or
    /// nesting, duplicate widgets, or invalid grid geometry.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(ValidationError::UnsupportedSchema {
                actual: self.schema_version,
            });
        }
        validate_identifier("layout", self.id.as_str())?;
        validate_string("layout name", &self.name, false)?;
        if self.widgets.len() > MAX_WIDGETS {
            return Err(ValidationError::TooManyWidgets {
                actual: self.widgets.len(),
                maximum: MAX_WIDGETS,
            });
        }

        let mut instance_ids = BTreeSet::new();
        for widget in &self.widgets {
            validate_identifier("widget instance", widget.instance_id.as_str())?;
            validate_component_type(widget.component_type.as_str())?;
            if !instance_ids.insert(widget.instance_id.as_str()) {
                return Err(ValidationError::DuplicateWidgetInstance(
                    widget.instance_id.as_str().to_owned(),
                ));
            }
            for placement in widget.placements.values() {
                validate_placement(*placement)?;
            }
            validate_json_value(&widget.settings, 0)?;
        }

        validate_color("theme background", &self.theme.background)?;
        validate_color("theme foreground", &self.theme.foreground)?;
        validate_color("theme accent", &self.theme.accent)?;
        validate_color("theme warning", &self.theme.warning)?;
        Ok(())
    }

    pub(crate) fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }
}

/// Versioned Host network and publication settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostSettings {
    /// Settings schema version.
    pub schema_version: u16,
    /// F1 UDP listen address.
    pub udp_bind: String,
    /// Local HTTP listen address.
    pub http_bind: String,
    /// Maximum snapshot publication rate.
    pub snapshot_hz: u16,
}

impl Default for HostSettings {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            udp_bind: "0.0.0.0:20777".into(),
            http_bind: "0.0.0.0:20778".into(),
            snapshot_hz: 60,
        }
    }
}

impl HostSettings {
    /// Validates safe settings bounds before runtime use.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] for unsupported schema/rate or oversized text.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(ValidationError::UnsupportedSchema {
                actual: self.schema_version,
            });
        }
        validate_string("UDP bind address", &self.udp_bind, false)?;
        validate_string("HTTP bind address", &self.http_bind, false)?;
        if !matches!(self.snapshot_hz, 20 | 30 | 60) {
            return Err(ValidationError::UnsupportedSnapshotRate(self.snapshot_hz));
        }
        Ok(())
    }
}

/// Failure while validating imported or persisted configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    /// A persisted document has an unsupported schema version.
    UnsupportedSchema {
        /// Rejected schema version.
        actual: u16,
    },
    /// Stable id contains unsafe or ambiguous characters.
    InvalidIdentifier {
        /// Kind of identifier being validated.
        kind: &'static str,
        /// Rejected value.
        value: String,
    },
    /// A required string is empty.
    EmptyString {
        /// Field being validated.
        field: &'static str,
    },
    /// A string exceeds its configured bound.
    StringTooLong {
        /// Field being validated.
        field: &'static str,
        /// Observed UTF-8 byte length.
        actual: usize,
        /// Maximum byte length.
        maximum: usize,
    },
    /// A theme token is not a bounded hexadecimal color.
    InvalidColor {
        /// Theme field being validated.
        field: &'static str,
        /// Rejected value.
        value: String,
    },
    /// Layout contains more widget instances than allowed.
    TooManyWidgets {
        /// Observed widget count.
        actual: usize,
        /// Maximum widget count.
        maximum: usize,
    },
    /// Two widgets share the same stable instance id.
    DuplicateWidgetInstance(String),
    /// Grid position or size is outside the bounded canvas.
    InvalidGridPlacement,
    /// Settings JSON exceeds the maximum nesting depth.
    SettingsTooDeep {
        /// Observed depth.
        actual: usize,
        /// Maximum accepted depth.
        maximum: usize,
    },
    /// One settings object/array contains too many entries.
    TooManySettingsEntries {
        /// Observed entries.
        actual: usize,
        /// Maximum accepted entries.
        maximum: usize,
    },
    /// Snapshot publication rate is not an allowed preset.
    UnsupportedSnapshotRate(u16),
}

impl Display for ValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { actual } => {
                write!(
                    formatter,
                    "unsupported configuration schema version {actual}"
                )
            }
            Self::InvalidIdentifier { kind, value } => {
                write!(formatter, "invalid {kind} identifier {value:?}")
            }
            Self::EmptyString { field } => write!(formatter, "{field} cannot be empty"),
            Self::StringTooLong {
                field,
                actual,
                maximum,
            } => write!(formatter, "{field} is {actual} bytes; maximum is {maximum}"),
            Self::InvalidColor { field, value } => {
                write!(
                    formatter,
                    "{field} must be a hexadecimal color, got {value:?}"
                )
            }
            Self::TooManyWidgets { actual, maximum } => write!(
                formatter,
                "layout contains {actual} widgets; maximum is {maximum}"
            ),
            Self::DuplicateWidgetInstance(id) => {
                write!(formatter, "duplicate widget instance {id:?}")
            }
            Self::InvalidGridPlacement => formatter.write_str("invalid widget grid placement"),
            Self::SettingsTooDeep { actual, maximum } => write!(
                formatter,
                "settings nesting depth {actual} exceeds maximum {maximum}"
            ),
            Self::TooManySettingsEntries { actual, maximum } => write!(
                formatter,
                "settings collection has {actual} entries; maximum is {maximum}"
            ),
            Self::UnsupportedSnapshotRate(rate) => {
                write!(
                    formatter,
                    "snapshot rate {rate} Hz is not one of 20, 30, or 60"
                )
            }
        }
    }
}

impl Error for ValidationError {}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), ValidationError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && !value.contains("--");
    if valid {
        Ok(())
    } else {
        Err(ValidationError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        })
    }
}

fn validate_component_type(value: &str) -> Result<(), ValidationError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_LEN
        && value.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && segment
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        });
    if valid {
        Ok(())
    } else {
        Err(ValidationError::InvalidIdentifier {
            kind: "component type",
            value: value.to_owned(),
        })
    }
}

fn validate_string(
    field: &'static str,
    value: &str,
    allow_empty: bool,
) -> Result<(), ValidationError> {
    if !allow_empty && value.is_empty() {
        return Err(ValidationError::EmptyString { field });
    }
    if value.len() > MAX_STRING_LEN {
        return Err(ValidationError::StringTooLong {
            field,
            actual: value.len(),
            maximum: MAX_STRING_LEN,
        });
    }
    Ok(())
}

fn validate_color(field: &'static str, value: &str) -> Result<(), ValidationError> {
    validate_string(field, value, false)?;
    let valid = value.strip_prefix('#').is_some_and(|digits| {
        matches!(digits.len(), 3 | 4 | 6 | 8) && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    if valid {
        Ok(())
    } else {
        Err(ValidationError::InvalidColor {
            field,
            value: value.to_owned(),
        })
    }
}

fn validate_placement(placement: GridPlacement) -> Result<(), ValidationError> {
    let valid = placement.width > 0
        && placement.height > 0
        && placement
            .x
            .checked_add(placement.width)
            .is_some_and(|right| right <= MAX_GRID_COLUMNS)
        && placement
            .y
            .checked_add(placement.height)
            .is_some_and(|bottom| bottom <= MAX_GRID_ROWS);
    if valid {
        Ok(())
    } else {
        Err(ValidationError::InvalidGridPlacement)
    }
}

fn validate_json_value(value: &Value, depth: usize) -> Result<(), ValidationError> {
    if depth > MAX_SETTINGS_DEPTH {
        return Err(ValidationError::SettingsTooDeep {
            actual: depth,
            maximum: MAX_SETTINGS_DEPTH,
        });
    }
    match value {
        Value::String(value) => validate_string("widget setting", value, true),
        Value::Array(values) => {
            validate_collection_len(values.len())?;
            for value in values {
                validate_json_value(value, depth + 1)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            validate_collection_len(values.len())?;
            for (key, value) in values {
                validate_string("widget setting key", key, false)?;
                validate_json_value(value, depth + 1)?;
            }
            Ok(())
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
    }
}

fn validate_collection_len(actual: usize) -> Result<(), ValidationError> {
    if actual > MAX_SETTINGS_ENTRIES {
        Err(ValidationError::TooManySettingsEntries {
            actual,
            maximum: MAX_SETTINGS_ENTRIES,
        })
    } else {
        Ok(())
    }
}
