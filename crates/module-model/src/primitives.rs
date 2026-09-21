use petunia_core::AppState;

use super::Tool;

#[derive(Default)]
pub struct PrimitivesTool;

impl Tool for PrimitivesTool {
    fn id(&self) -> &'static str {
        "primitives"
    }
    fn label_key(&self) -> &'static str {
        "tools.primitives"
    }
    fn hint_key(&self) -> &'static str {
        "hints.primitives"
    }
    fn icon(&self) -> &'static str {
        "⬢"
    }
    fn shortcut(&self) -> &'static str {
        "A"
    }
}

impl PrimitivesTool {
    /// Caminho canônico único (§82): abre a sessão de criação (transação
    /// única + cartão Last Operation) em vez de inserir a malha diretamente.
    ///
    /// Nomes legados mantidos para CLI/FFI/testes (`Cylinder8`, …).
    /// UI/legacy entry: unknown names are rejected (never silently Cone).
    pub fn add_primitive(state: &mut AppState, name: &str) {
        if let Err(err) = Self::try_add_primitive(state, name) {
            state.set_status(format!("add primitive: {err}"));
        }
    }

    pub fn try_add_primitive(
        state: &mut AppState,
        name: &str,
    ) -> Result<(), petunia_core::CommandError> {
        let kind = petunia_core::PrimitiveKind::parse(name)?;
        state.dispatch(&petunia_core::AddPrimitiveCmd {
            kind,
            name: Some(name.to_string()),
            at_cursor: true,
        })
    }
}
