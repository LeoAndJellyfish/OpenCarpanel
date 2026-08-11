//! Versioned configuration persistence and migrations.

mod migration;
mod model;
mod repository;
mod schema;

pub use migration::migrate_layout_json;
pub use model::{
    BreakpointName, CONFIG_SCHEMA_VERSION, ComponentType, GridPlacement, HostSettings, InstanceId,
    LayoutDocument, LayoutId, MAX_LAYOUT_BYTES, MAX_WIDGETS, ThemeSettings, ValidationError,
    WidgetInstance,
};
pub use repository::{ConfigError, LayoutRepository, LoadedLayout};
pub use schema::generate_layout_schema;
