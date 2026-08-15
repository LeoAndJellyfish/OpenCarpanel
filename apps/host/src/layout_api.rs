use std::{collections::BTreeMap, sync::Arc};

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use opensimdash_config::{
    BreakpointName, ComponentType, ConfigError, GridPlacement, InstanceId, LayoutDocument,
    LayoutId, LayoutRepository, MAX_LAYOUT_BYTES, ThemeSettings, ValidationError, WidgetInstance,
};
use opensimdash_game_plugin_api::{GamePluginMetadata, PluginLayoutPreset};
use serde::Serialize;
use serde_json::{Value, json};

use crate::http::HttpState;

const DEFAULT_LAYOUT_ID: &str = "default";
const LEGACY_DEFAULT_LAYOUT_NAME: &str = "F1 24 Default";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LayoutEnvelope {
    document: LayoutDocument,
    recovered: bool,
}

#[derive(Debug)]
pub(crate) enum LayoutApiError {
    Unauthorized,
    InvalidRequest(&'static str),
    InvalidLayout(String),
    NotFound,
    Conflict(Box<LayoutDocument>),
    Internal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    current: Option<LayoutEnvelope>,
}

impl IntoResponse for LayoutApiError {
    fn into_response(self) -> Response {
        let (status, code, message, current) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "device_session_required",
                "a valid paired-device session is required".to_owned(),
                None,
            ),
            Self::InvalidRequest(message) => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                message.to_owned(),
                None,
            ),
            Self::InvalidLayout(message) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_layout",
                message,
                None,
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "layout_not_found",
                "the requested layout does not exist".to_owned(),
                None,
            ),
            Self::Conflict(document) => (
                StatusCode::CONFLICT,
                "revision_conflict",
                "the layout changed on another device".to_owned(),
                Some(LayoutEnvelope {
                    document: *document,
                    recovered: false,
                }),
            ),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "the Host could not complete the layout operation".to_owned(),
                None,
            ),
        };
        (
            status,
            Json(ErrorBody {
                code,
                message,
                current,
            }),
        )
            .into_response()
    }
}

pub(crate) async fn get_layout(
    State(state): State<HttpState>,
    Path(raw_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<LayoutEnvelope>, LayoutApiError> {
    authorize(&state, &headers).await?;
    let id =
        LayoutId::new(raw_id).map_err(|_| LayoutApiError::InvalidRequest("invalid layout id"))?;
    let repository = Arc::clone(&state.layouts);
    let plugins = state
        .host
        .supported_adapters()
        .iter()
        .map(|adapter| adapter.metadata().clone())
        .collect::<Vec<_>>();
    let loaded = tokio::task::spawn_blocking(move || load_or_create(&repository, &id, &plugins))
        .await
        .map_err(|_| LayoutApiError::Internal)?
        .map_err(map_config_error)?;
    loaded.map(Json).ok_or(LayoutApiError::NotFound)
}

pub(crate) async fn put_layout(
    State(state): State<HttpState>,
    Path(raw_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<LayoutEnvelope>, LayoutApiError> {
    authorize(&state, &headers).await?;
    require_json_content_type(&headers)?;
    if body.len() > MAX_LAYOUT_BYTES {
        return Err(LayoutApiError::InvalidRequest(
            "layout document is too large",
        ));
    }
    let id =
        LayoutId::new(raw_id).map_err(|_| LayoutApiError::InvalidRequest("invalid layout id"))?;
    let document: LayoutDocument = serde_json::from_slice(&body)
        .map_err(|_| LayoutApiError::InvalidRequest("layout body is not valid versioned JSON"))?;
    document
        .validate()
        .map_err(|error| LayoutApiError::InvalidLayout(error.to_string()))?;
    if document.id() != &id {
        return Err(LayoutApiError::InvalidRequest(
            "layout path and document id do not match",
        ));
    }
    validate_builtin_widgets(&document)?;

    let expected_revision = document.revision();
    let repository = Arc::clone(&state.layouts);
    let outcome =
        tokio::task::spawn_blocking(
            move || match repository.save(&document, expected_revision) {
                Ok(saved) => Ok(SaveOutcome::Saved(saved)),
                Err(ConfigError::Conflict { .. }) => repository
                    .load_required(&id)
                    .map(|loaded| SaveOutcome::Conflict(loaded.document)),
                Err(error) => Err(error),
            },
        )
        .await
        .map_err(|_| LayoutApiError::Internal)?
        .map_err(map_config_error)?;

    match outcome {
        SaveOutcome::Saved(document) => Ok(Json(LayoutEnvelope {
            document,
            recovered: false,
        })),
        SaveOutcome::Conflict(current) => Err(LayoutApiError::Conflict(Box::new(current))),
    }
}

enum SaveOutcome {
    Saved(LayoutDocument),
    Conflict(LayoutDocument),
}

async fn authorize(state: &HttpState, headers: &HeaderMap) -> Result<(), LayoutApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or(LayoutApiError::Unauthorized)?;
    state
        .pairing
        .authorize_device_session(value)
        .await
        .map_err(|_| LayoutApiError::Unauthorized)
}

fn require_json_content_type(headers: &HeaderMap) -> Result<(), LayoutApiError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
    {
        Ok(())
    } else {
        Err(LayoutApiError::InvalidRequest(
            "layout requests require application/json",
        ))
    }
}

fn load_or_create(
    repository: &LayoutRepository,
    id: &LayoutId,
    plugins: &[GamePluginMetadata],
) -> Result<Option<LayoutEnvelope>, ConfigError> {
    if let Some(loaded) = repository.load(id)? {
        let document = upgrade_previous_builtin_layout(repository, id, loaded.document, plugins)?;
        return Ok(Some(LayoutEnvelope {
            document,
            recovered: loaded.recovered,
        }));
    }
    let Some(default) = layout_for_new_id(repository, id, plugins)? else {
        return Ok(None);
    };

    match repository.save(&default, 0) {
        Ok(document) => Ok(Some(LayoutEnvelope {
            document,
            recovered: false,
        })),
        Err(ConfigError::Conflict { .. }) => repository.load(id).map(|loaded| {
            loaded.map(|loaded| LayoutEnvelope {
                document: loaded.document,
                recovered: loaded.recovered,
            })
        }),
        Err(error) => Err(error),
    }
}

fn upgrade_previous_builtin_layout(
    repository: &LayoutRepository,
    id: &LayoutId,
    document: LayoutDocument,
    plugins: &[GamePluginMetadata],
) -> Result<LayoutDocument, ConfigError> {
    if !is_v0_3_builtin_layout(id, &document) {
        return Ok(document);
    }
    let Some(current) = plugin_layout(id, plugins).map_err(ConfigError::Validation)? else {
        return Ok(document);
    };
    match repository.save(&current, document.revision()) {
        Ok(saved) => Ok(saved),
        Err(ConfigError::Conflict { .. }) => {
            repository.load_required(id).map(|loaded| loaded.document)
        }
        Err(error) => Err(error),
    }
}

fn is_v0_3_builtin_layout(id: &LayoutId, document: &LayoutDocument) -> bool {
    let Some(spec) = legacy_builtin_layout_spec(id) else {
        return false;
    };
    if document.revision() != 1
        || document.id() != id
        || document.name() != spec.name
        || document.widgets().len() != 4
        || document.theme().background != spec.background
        || document.theme().foreground != spec.foreground
        || document.theme().accent != spec.accent
        || document.theme().warning != spec.warning
    {
        return false;
    }

    let expected = [
        ("tachometer", "core.tachometer"),
        ("gear", "core.gear"),
        ("speed", "core.speed"),
        ("status", "core.status"),
    ];
    document
        .widgets()
        .iter()
        .zip(expected)
        .all(|(widget, (instance_id, component_type))| {
            widget.instance_id.as_str() == instance_id
                && widget.component_type.as_str() == component_type
                && match component_type {
                    "core.tachometer" => {
                        widget.settings == json!({"fallbackRpmMax": spec.fallback_rpm_max})
                    }
                    "core.speed" => widget.settings == json!({"unit": "km/h"}),
                    _ => widget.settings == json!({}),
                }
        })
}

fn layout_for_new_id(
    repository: &LayoutRepository,
    id: &LayoutId,
    plugins: &[GamePluginMetadata],
) -> Result<Option<LayoutDocument>, ConfigError> {
    let built_in = plugin_layout(id, plugins).map_err(ConfigError::Validation)?;
    if id.as_str() == DEFAULT_LAYOUT_ID || built_in.is_none() {
        return Ok(built_in);
    }

    let legacy_id = LayoutId::new(DEFAULT_LAYOUT_ID).map_err(ConfigError::Validation)?;
    let legacy = repository.load(&legacy_id)?.map(|loaded| loaded.document);
    if let Some(document) = legacy.filter(is_legacy_default_layout) {
        return clone_layout_with_id(&document, id)
            .map(Some)
            .map_err(ConfigError::Validation);
    }
    Ok(built_in)
}

fn is_legacy_default_layout(document: &LayoutDocument) -> bool {
    document.name() == LEGACY_DEFAULT_LAYOUT_NAME || document.revision() > 1
}

fn clone_layout_with_id(
    source: &LayoutDocument,
    id: &LayoutId,
) -> Result<LayoutDocument, ValidationError> {
    let mut migrated = LayoutDocument::empty(id.clone(), source.name())?;
    migrated.set_theme(source.theme().clone());
    migrated.set_widgets(source.widgets().to_vec());
    migrated.validate()?;
    Ok(migrated)
}

#[derive(Clone, Copy)]
struct LegacyBuiltInLayoutSpec {
    name: &'static str,
    background: &'static str,
    foreground: &'static str,
    accent: &'static str,
    warning: &'static str,
    fallback_rpm_max: u64,
}

fn legacy_builtin_layout_spec(id: &LayoutId) -> Option<LegacyBuiltInLayoutSpec> {
    let spec = match id.as_str() {
        DEFAULT_LAYOUT_ID => LegacyBuiltInLayoutSpec {
            name: "OpenSimDash Default",
            background: "#07090c",
            foreground: "#f2f0e9",
            accent: "#d9ff43",
            warning: "#ff4b3e",
            fallback_rpm_max: 12_000,
        },
        "game-f1-24" => LegacyBuiltInLayoutSpec {
            name: "F1 24 Trackside",
            background: "#07090c",
            foreground: "#f2f0e9",
            accent: "#d9ff43",
            warning: "#ff4b3e",
            fallback_rpm_max: 12_000,
        },
        "game-f1-25" => LegacyBuiltInLayoutSpec {
            name: "F1 25 Electric Grid",
            background: "#061015",
            foreground: "#eefcff",
            accent: "#42e8ff",
            warning: "#ff5e6c",
            fallback_rpm_max: 12_000,
        },
        "game-ets2" => LegacyBuiltInLayoutSpec {
            name: "ETS2 Long Haul",
            background: "#0e0b08",
            foreground: "#fff5e5",
            accent: "#ffbd45",
            warning: "#ff4b3e",
            fallback_rpm_max: 2_500,
        },
        "game-ats" => LegacyBuiltInLayoutSpec {
            name: "ATS Interstate",
            background: "#080d10",
            foreground: "#f5f0e6",
            accent: "#ff6a3d",
            warning: "#ffcf54",
            fallback_rpm_max: 2_500,
        },
        _ => return None,
    };
    Some(spec)
}

fn plugin_layout(
    id: &LayoutId,
    plugins: &[GamePluginMetadata],
) -> Result<Option<LayoutDocument>, ValidationError> {
    let plugin = if id.as_str() == DEFAULT_LAYOUT_ID {
        plugins
            .iter()
            .find(|plugin| plugin.presentation.layout_preset == PluginLayoutPreset::Formula)
            .or_else(|| plugins.first())
    } else {
        id.as_str()
            .strip_prefix("game-")
            .and_then(|plugin_id| plugins.iter().find(|plugin| plugin.id == plugin_id))
    };
    let Some(plugin) = plugin else {
        return Ok(None);
    };

    let name = if id.as_str() == DEFAULT_LAYOUT_ID {
        "OpenSimDash Default".to_owned()
    } else {
        format!("{} Default", plugin.presentation.short_name)
    };
    let mut document = LayoutDocument::empty(LayoutId::new(id.as_str())?, name)?;
    document.set_theme(ThemeSettings {
        background: plugin.presentation.theme.background.clone(),
        foreground: plugin.presentation.theme.foreground.clone(),
        accent: plugin.presentation.theme.accent.clone(),
        warning: plugin.presentation.theme.warning.clone(),
    });
    let mut widgets = match plugin.presentation.layout_preset {
        PluginLayoutPreset::Formula | PluginLayoutPreset::Generic => {
            formula_widgets(u64::from(plugin.presentation.fallback_rpm_max))?
        }
        PluginLayoutPreset::Truck => {
            truck_widgets(u64::from(plugin.presentation.fallback_rpm_max))?
        }
    };
    widgets.retain(|widget| {
        plugin
            .presentation
            .widgets
            .iter()
            .any(|component| component == widget.component_type.as_str())
    });
    document.set_widgets(widgets);
    document.validate()?;
    Ok(Some(document))
}

fn formula_widgets(fallback_rpm_max: u64) -> Result<Vec<WidgetInstance>, ValidationError> {
    Ok(vec![
        widget(
            "tachometer",
            "core.tachometer",
            [
                (BreakpointName::PhonePortrait, placement(0, 0, 12, 3)),
                (BreakpointName::PhoneLandscape, placement(0, 0, 12, 3)),
                (BreakpointName::Tablet, placement(0, 0, 12, 3)),
                (BreakpointName::Desktop, placement(0, 0, 12, 3)),
            ],
            json!({"fallbackRpmMax": fallback_rpm_max}),
        )?,
        widget(
            "gear",
            "core.gear",
            [
                (BreakpointName::PhonePortrait, placement(2, 3, 8, 9)),
                (BreakpointName::PhoneLandscape, placement(4, 3, 4, 5)),
                (BreakpointName::Tablet, placement(4, 3, 4, 6)),
                (BreakpointName::Desktop, placement(4, 3, 4, 6)),
            ],
            json!({}),
        )?,
        widget(
            "speed",
            "core.speed",
            [
                (BreakpointName::PhonePortrait, placement(0, 12, 5, 3)),
                (BreakpointName::PhoneLandscape, placement(0, 3, 4, 5)),
                (BreakpointName::Tablet, placement(0, 3, 4, 6)),
                (BreakpointName::Desktop, placement(0, 3, 4, 6)),
            ],
            json!({"unit": "km/h"}),
        )?,
        widget(
            "status",
            "core.status",
            [
                (BreakpointName::PhonePortrait, placement(6, 12, 6, 3)),
                (BreakpointName::PhoneLandscape, placement(8, 3, 4, 5)),
                (BreakpointName::Tablet, placement(8, 3, 4, 6)),
                (BreakpointName::Desktop, placement(8, 3, 4, 6)),
            ],
            json!({}),
        )?,
        widget(
            "race",
            "core.race",
            [
                (BreakpointName::PhonePortrait, placement(0, 15, 7, 3)),
                (BreakpointName::PhoneLandscape, placement(0, 8, 7, 2)),
                (BreakpointName::Tablet, placement(0, 9, 7, 3)),
                (BreakpointName::Desktop, placement(0, 9, 7, 3)),
            ],
            json!({}),
        )?,
        widget(
            "tyres",
            "core.tyres",
            [
                (BreakpointName::PhonePortrait, placement(7, 15, 5, 3)),
                (BreakpointName::PhoneLandscape, placement(7, 8, 5, 2)),
                (BreakpointName::Tablet, placement(7, 9, 5, 3)),
                (BreakpointName::Desktop, placement(7, 9, 5, 3)),
            ],
            json!({}),
        )?,
    ])
}

fn truck_widgets(fallback_rpm_max: u64) -> Result<Vec<WidgetInstance>, ValidationError> {
    Ok(vec![
        widget(
            "tachometer",
            "core.tachometer",
            [
                (BreakpointName::PhonePortrait, placement(5, 7, 7, 3)),
                (BreakpointName::PhoneLandscape, placement(0, 7, 7, 3)),
                (BreakpointName::Tablet, placement(0, 8, 7, 4)),
                (BreakpointName::Desktop, placement(0, 8, 7, 4)),
            ],
            json!({"fallbackRpmMax": fallback_rpm_max}),
        )?,
        widget(
            "gear",
            "core.gear",
            [
                (BreakpointName::PhonePortrait, placement(0, 7, 5, 5)),
                (BreakpointName::PhoneLandscape, placement(7, 0, 5, 4)),
                (BreakpointName::Tablet, placement(7, 0, 5, 4)),
                (BreakpointName::Desktop, placement(7, 0, 5, 4)),
            ],
            json!({}),
        )?,
        widget(
            "speed",
            "core.speed",
            [
                (BreakpointName::PhonePortrait, placement(0, 0, 12, 7)),
                (BreakpointName::PhoneLandscape, placement(0, 0, 7, 7)),
                (BreakpointName::Tablet, placement(0, 0, 7, 8)),
                (BreakpointName::Desktop, placement(0, 0, 7, 8)),
            ],
            json!({"unit": "km/h"}),
        )?,
        widget(
            "status",
            "core.status",
            [
                (BreakpointName::PhonePortrait, placement(5, 10, 7, 2)),
                (BreakpointName::PhoneLandscape, placement(7, 4, 5, 2)),
                (BreakpointName::Tablet, placement(7, 4, 5, 2)),
                (BreakpointName::Desktop, placement(7, 4, 5, 2)),
            ],
            json!({}),
        )?,
        widget(
            "route",
            "core.route",
            [
                (BreakpointName::PhonePortrait, placement(0, 12, 12, 6)),
                (BreakpointName::PhoneLandscape, placement(7, 6, 5, 4)),
                (BreakpointName::Tablet, placement(7, 6, 5, 6)),
                (BreakpointName::Desktop, placement(7, 6, 5, 6)),
            ],
            json!({}),
        )?,
    ])
}

fn widget(
    instance_id: &str,
    component_type: &str,
    placements: [(BreakpointName, GridPlacement); 4],
    settings: Value,
) -> Result<WidgetInstance, ValidationError> {
    Ok(WidgetInstance {
        instance_id: InstanceId::new(instance_id)?,
        component_type: ComponentType::new(component_type)?,
        placements: BTreeMap::from(placements),
        settings,
    })
}

const fn placement(x: u16, y: u16, width: u16, height: u16) -> GridPlacement {
    GridPlacement {
        x,
        y,
        width,
        height,
    }
}

fn validate_builtin_widgets(document: &LayoutDocument) -> Result<(), LayoutApiError> {
    for widget in document.widgets() {
        let settings = widget.settings.as_object().ok_or_else(|| {
            LayoutApiError::InvalidLayout("widget settings must be a JSON object".into())
        })?;
        let valid = match widget.component_type.as_str() {
            "core.gear" | "core.race" | "core.route" | "core.status" | "core.tyres" => {
                settings.is_empty()
            }
            "core.speed" => {
                settings.keys().all(|key| key == "unit")
                    && settings
                        .get("unit")
                        .is_none_or(|value| value.as_str() == Some("km/h"))
            }
            "core.tachometer" => {
                settings.keys().all(|key| key == "fallbackRpmMax")
                    && settings.get("fallbackRpmMax").is_none_or(|value| {
                        value
                            .as_u64()
                            .is_some_and(|rpm| (1_000..=30_000).contains(&rpm))
                    })
            }
            _ => false,
        };
        if !valid {
            return Err(LayoutApiError::InvalidLayout(format!(
                "unsupported component or settings for {}",
                widget.component_type.as_str()
            )));
        }
    }
    Ok(())
}

fn map_config_error(error: ConfigError) -> LayoutApiError {
    match error {
        ConfigError::Validation(error) => LayoutApiError::InvalidLayout(error.to_string()),
        ConfigError::DocumentTooLarge { .. } => {
            LayoutApiError::InvalidRequest("layout document is too large")
        }
        ConfigError::Json(_) | ConfigError::InvalidSchemaVersion => {
            LayoutApiError::InvalidRequest("layout body is invalid")
        }
        ConfigError::UnsupportedSchema { .. } | ConfigError::Migration(_) => {
            LayoutApiError::InvalidLayout("layout schema is not supported".into())
        }
        _ => LayoutApiError::Internal,
    }
}
