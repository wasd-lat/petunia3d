use petunia_core::AppState;

use super::Tool;

/// Bevel simples (§8.8): chanfro em arestas manifold selecionadas.
#[derive(Default)]
pub struct BevelTool;

impl Tool for BevelTool {
    fn id(&self) -> &'static str {
        "bevel"
    }
    fn label_key(&self) -> &'static str {
        "tools.bevel"
    }
    fn hint_key(&self) -> &'static str {
        "hints.bevel"
    }
    fn icon(&self) -> &'static str {
        "◐"
    }
    fn shortcut(&self) -> &'static str {
        "Ctrl+B"
    }
    fn on_activate(&self, state: &mut AppState) {
        state.pending_modal = Some(petunia_core::ModalKind::Bevel);
        state.mark_dirty();
    }
}

impl BevelTool {
    pub fn apply(state: &mut AppState) {
        let amt = state.bevel_amount;
        let segs = state.bevel_segments.clamp(1, 4);
        let clamp = state.tools.bevel_clamp_overlap;
        if let Err(err) = state.dispatch(&petunia_core::BevelCmd {
            amount: amt,
            segments: segs,
            clamp_overlap: clamp,
        }) {
            state.set_status(format!("bevel: {err}"));
        }
    }
}
