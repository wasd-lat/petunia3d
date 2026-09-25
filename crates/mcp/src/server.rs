//! Tools-only MCP server over stdio.
//!
//! MCP is an adapter: requests validate, then call the Application API
//! (`AppState::dispatch_intent`). It does not own a parallel Document/Undo.

use std::sync::Arc;

use petunia_core::{
    AddPrimitiveCmd, AppState, CommandIntent, PrimitiveKind, ProjectService, SceneItemContract,
    UndoCmd,
};
use rmcp::ErrorData as McpError;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{ServiceExt, tool, tool_router, transport::stdio};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Shared Application session behind the tool boundary.
struct McpSession {
    state: AppState,
}

impl McpSession {
    fn new() -> Self {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        Self { state }
    }
}

/// Parameters for [`PetuniaMcp::add_primitive`].
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AddPrimitiveParams {
    /// Primitive kind: `"cube"`, `"plane"`, `"sphere"`, …
    kind: String,
    /// Edge size in world units (0.01..=100). Reserved; V1 uses default mesh.
    size: f64,
}

/// Parameters for [`PetuniaMcp::export_obj`].
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ExportObjParams {
    /// Asset index in scene order.
    index: usize,
}

/// Parameters for [`PetuniaMcp::validate_intent`].
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ValidateIntentParams {
    /// Command id, e.g. `"model.extrude"`.
    command: String,
    /// Optional target asset UUID.
    #[serde(default)]
    asset_id: Option<String>,
    /// Numeric args in command-defined order.
    #[serde(default)]
    args: Vec<f64>,
}

/// Parameters for [`PetuniaMcp::execute_intent`].
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ExecuteIntentParams {
    /// Command id, e.g. `"model.extrude"`.
    command: String,
    /// Optional target asset UUID.
    #[serde(default)]
    asset_id: Option<String>,
    /// Numeric args in command-defined order.
    #[serde(default)]
    args: Vec<f64>,
}

/// Validation report returned by [`PetuniaMcp::validate_intent`].
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct IntentReport {
    /// Whether the intent is well-formed and registered.
    valid: bool,
    /// Stable reason code (`"ok"`, `"bad_shape"`, `"not_allowlisted"`).
    reason: String,
}

/// Asset summary DTO returned by scene tools.
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct AssetSummaryDto {
    /// Display name.
    name: String,
    /// Always `"object"` on this boundary.
    kind: String,
    /// Visibility flag.
    visible: bool,
}

/// Petunia MCP server. Clone shares the same Application session.
#[derive(Clone)]
pub struct PetuniaMcp {
    session: Arc<Mutex<McpSession>>,
}

impl std::fmt::Debug for PetuniaMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PetuniaMcp").finish_non_exhaustive()
    }
}

impl PetuniaMcp {
    /// Creates a server with a fresh Application session.
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(McpSession::new())),
        }
    }

    fn asset_summaries(state: &AppState) -> Vec<AssetSummaryDto> {
        state
            .query_scene_hierarchy()
            .assets
            .into_iter()
            .map(|asset| AssetSummaryDto {
                name: asset.name,
                kind: "object".to_string(),
                visible: asset.visible,
            })
            .collect()
    }

    fn intent_well_formed(command: &str, asset_id: Option<&str>, args: &[f64]) -> bool {
        let asset_ok = asset_id
            .is_none_or(|id| uuid::Uuid::parse_str(id).is_ok() || PrimitiveKind::parse(id).is_ok());
        !command.is_empty()
            && command.len() <= 64
            && command
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
            && args.iter().all(|a| a.is_finite())
            && asset_ok
    }
}

impl Default for PetuniaMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(server_handler)]
impl PetuniaMcp {
    /// Lists scene assets as `{name, kind, visible}` DTOs.
    #[tool(description = "List scene assets (name, kind, visible).")]
    async fn list_assets(&self) -> Result<Json<Vec<AssetSummaryDto>>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(Self::asset_summaries(&session.state)))
    }

    /// Adds a primitive through the Application command spine.
    #[tool(description = "Add a primitive to the scene (undoable).")]
    async fn add_primitive(
        &self,
        Parameters(params): Parameters<AddPrimitiveParams>,
    ) -> Result<Json<AssetSummaryDto>, McpError> {
        if !params.size.is_finite() || !(0.01..=100.0).contains(&params.size) {
            return Err(McpError::invalid_params(
                "size must be finite within 0.01..=100",
                None,
            ));
        }
        let kind = PrimitiveKind::parse(&params.kind).map_err(|e| {
            McpError::invalid_params(format!("unknown primitive '{}': {e}", params.kind), None)
        })?;
        let mut session = self.session.lock().await;
        session
            .state
            .dispatch(&AddPrimitiveCmd {
                kind,
                name: Some(format!("MCP {}", params.kind)),
                at_cursor: true,
            })
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let asset = session
            .state
            .project
            .active()
            .ok_or_else(|| McpError::internal_error("no active asset after add", None))?;
        Ok(Json(AssetSummaryDto {
            name: asset.name.clone(),
            kind: "object".to_string(),
            visible: asset.visible,
        }))
    }

    /// Undoes the last Application mutation.
    #[tool(description = "Undo the last scene mutation.")]
    async fn undo(&self) -> Result<Json<bool>, McpError> {
        let mut session = self.session.lock().await;
        Ok(Json(session.state.dispatch(&UndoCmd).is_ok()))
    }

    /// Exports one asset as OBJ text (bounded by scene content).
    #[tool(description = "Export one scene asset as OBJ text by index.")]
    async fn export_obj(
        &self,
        Parameters(params): Parameters<ExportObjParams>,
    ) -> Result<String, McpError> {
        let session = self.session.lock().await;
        let asset = session
            .state
            .project
            .assets
            .get(params.index)
            .ok_or_else(|| {
                McpError::invalid_params(format!("asset index {} out of range", params.index), None)
            })?;
        Ok(petunia_project::export::export_obj(asset))
    }

    /// Validates a command intent against the Application registry without executing it.
    #[tool(description = "Validate a command intent (id, target, args) without executing.")]
    async fn validate_intent(
        &self,
        Parameters(params): Parameters<ValidateIntentParams>,
    ) -> Result<Json<IntentReport>, McpError> {
        let session = self.session.lock().await;
        let well_formed =
            Self::intent_well_formed(&params.command, params.asset_id.as_deref(), &params.args);
        let registered = session.state.commands.contains(&params.command);
        let (valid, reason) = if !well_formed {
            (false, "bad_shape")
        } else if !registered {
            (false, "not_allowlisted")
        } else {
            (true, "ok")
        };
        Ok(Json(IntentReport {
            valid,
            reason: reason.to_string(),
        }))
    }

    /// Executes a validated command intent through the Application API.
    #[tool(description = "Execute a command intent through the Application command spine.")]
    async fn execute_intent(
        &self,
        Parameters(params): Parameters<ExecuteIntentParams>,
    ) -> Result<Json<IntentReport>, McpError> {
        let well_formed =
            Self::intent_well_formed(&params.command, params.asset_id.as_deref(), &params.args);
        if !well_formed {
            return Ok(Json(IntentReport {
                valid: false,
                reason: "bad_shape".to_string(),
            }));
        }
        let mut session = self.session.lock().await;
        if !session.state.commands.contains(&params.command)
            && !matches!(
                params.command.as_str(),
                "model.add_primitive" | "model.add_cube" | "model.scale" | "model.scale_selection"
            )
        {
            return Ok(Json(IntentReport {
                valid: false,
                reason: "not_allowlisted".to_string(),
            }));
        }
        let intent = CommandIntent {
            command: params.command,
            asset_id: params.asset_id,
            args: params.args,
        };
        match session.state.dispatch_intent(&intent) {
            Ok(()) => Ok(Json(IntentReport {
                valid: true,
                reason: "ok".to_string(),
            })),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    /// Exposes the scene as query DTOs (mirrors Application Queries shape).
    #[tool(description = "Scene items as query DTOs (name, kind, visible).")]
    async fn scene_items(&self) -> Result<Json<Vec<SceneItemContract>>, McpError> {
        let session = self.session.lock().await;
        Ok(Json(
            session
                .state
                .query_scene_hierarchy()
                .assets
                .into_iter()
                .map(|asset| SceneItemContract {
                    name: asset.name,
                    kind: "object".to_string(),
                    visible: asset.visible,
                })
                .collect(),
        ))
    }
}

/// Serves the Petunia MCP server over stdio. The Tokio runtime stays inside
/// this call; cancel by closing the transport (client disconnect).
pub async fn serve_stdio() -> Result<(), McpError> {
    let service = PetuniaMcp::new()
        .serve(stdio())
        .await
        .map_err(|e| McpError::internal_error(format!("serve: {e}"), None))?;
    service
        .waiting()
        .await
        .map_err(|e| McpError::internal_error(format!("waiting: {e}"), None))?;
    Ok(())
}

/// Blocking stdio entry for embedding binaries: builds a current-thread
/// Tokio runtime internally, so hosts never name a Tokio type.
pub fn serve_stdio_blocking() -> Result<(), McpError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| McpError::internal_error(format!("runtime: {e}"), None))?
        .block_on(serve_stdio())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_starts_with_starter_scene() {
        let server = PetuniaMcp::new();
        let items = server.list_assets().await.unwrap().0;
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i.kind == "object"));
    }

    #[tokio::test]
    async fn add_undo_roundtrip() {
        let server = PetuniaMcp::new();
        let before = server.list_assets().await.unwrap().0.len();
        let added = server
            .add_primitive(Parameters(AddPrimitiveParams {
                kind: "cube".to_string(),
                size: 2.0,
            }))
            .await
            .unwrap()
            .0;
        assert_eq!(added.name, "MCP cube");
        assert_eq!(server.list_assets().await.unwrap().0.len(), before + 1);
        assert!(server.undo().await.unwrap().0);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before);
        assert!(!server.undo().await.unwrap().0);
    }

    #[tokio::test]
    async fn invalid_primitive_params_are_rejected() {
        let server = PetuniaMcp::new();
        assert!(
            server
                .add_primitive(Parameters(AddPrimitiveParams {
                    kind: "not-a-prim".to_string(),
                    size: 1.0,
                }))
                .await
                .is_err()
        );
        assert!(
            server
                .add_primitive(Parameters(AddPrimitiveParams {
                    kind: "cube".to_string(),
                    size: f64::NAN,
                }))
                .await
                .is_err()
        );
        assert!(
            server
                .export_obj(Parameters(ExportObjParams { index: 9999 }))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn export_and_validate_paths() {
        let server = PetuniaMcp::new();
        let obj = server
            .export_obj(Parameters(ExportObjParams { index: 0 }))
            .await
            .unwrap();
        assert!(obj.contains('v'));

        let ok = server
            .validate_intent(Parameters(ValidateIntentParams {
                command: "model.extrude".to_string(),
                asset_id: None,
                args: vec![0.5],
            }))
            .await
            .unwrap()
            .0;
        assert!(ok.valid);
        assert_eq!(ok.reason, "ok");

        let unknown = server
            .validate_intent(Parameters(ValidateIntentParams {
                command: "model.frobnicator".to_string(),
                asset_id: None,
                args: vec![],
            }))
            .await
            .unwrap()
            .0;
        assert!(!unknown.valid);
        assert_eq!(unknown.reason, "not_allowlisted");

        let bad = server
            .validate_intent(Parameters(ValidateIntentParams {
                command: "rm -rf".to_string(),
                asset_id: None,
                args: vec![],
            }))
            .await
            .unwrap()
            .0;
        assert!(!bad.valid);
        assert_eq!(bad.reason, "bad_shape");
    }

    #[tokio::test]
    async fn execute_intent_uses_application_commands() {
        let server = PetuniaMcp::new();
        let before = server.list_assets().await.unwrap().0.len();
        let report = server
            .execute_intent(Parameters(ExecuteIntentParams {
                command: "model.add_primitive".to_string(),
                asset_id: Some("Plane".to_string()),
                args: vec![],
            }))
            .await
            .unwrap()
            .0;
        assert!(report.valid);
        assert_eq!(server.list_assets().await.unwrap().0.len(), before + 1);
    }
}
