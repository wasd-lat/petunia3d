//! Apply validated Lua intents through the Application command spine.

use petunia_core::{AppState, CommandError, CommandIntent};

use crate::host::ValidatedIntent;

/// Dispatches recorded plugin commands through `AppState::dispatch_intent`.
pub fn apply_intents(state: &mut AppState, intent: &ValidatedIntent) -> Result<(), CommandError> {
    for cmd in &intent.commands {
        let application = CommandIntent {
            command: cmd.command.clone(),
            asset_id: None,
            args: cmd.args.clone(),
        };
        state.dispatch_intent(&application)?;
    }
    Ok(())
}

/// Answers a recorded plugin query with a real JSON payload (never `"{}"`).
pub fn answer_query(state: &AppState, name: &str) -> Result<String, CommandError> {
    match name {
        "scene.list" | "scene.hierarchy" => serde_json::to_string(&state.query_scene_hierarchy())
            .map_err(|e| CommandError::Execution(e.to_string())),
        "scene.selection" => serde_json::to_string(&state.query_selection_details())
            .map_err(|e| CommandError::Execution(e.to_string())),
        "scene.tools" => serde_json::to_string(&state.query_tool_status())
            .map_err(|e| CommandError::Execution(e.to_string())),
        other => Err(CommandError::UnknownCommand(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Host, PLUGIN_API_VERSION, PluginCapabilities, PluginId};
    use petunia_core::ProjectService;

    #[test]
    fn lua_intent_reaches_application_commands() {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        let before = state.project.assets.len();
        let host = Host::new();
        let caps = PluginCapabilities {
            api_version: PLUGIN_API_VERSION,
            commands: vec!["model.add_primitive".to_string()],
            queries: vec!["scene.list".to_string()],
            filesystem_roots: vec![],
            allow_network: false,
            allow_destructive: false,
        };
        let id = PluginId::new("test.add").unwrap();
        let intent = host
            .execute(&id, &caps, r#"petunia.command("model.add_primitive")"#)
            .unwrap();
        apply_intents(&mut state, &intent).unwrap();
        assert_eq!(state.project.assets.len(), before + 1);
        let json = answer_query(&state, "scene.list").unwrap();
        assert!(json.contains("Cube"));
        assert_ne!(json, "{}");
    }
}
