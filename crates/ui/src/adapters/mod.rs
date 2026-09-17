//! # Adapters — a única camada que conhece crates auxiliares
//!
//! Regra §22.1 da diretiva Egui Ecosystem Final Push:
//!
//! > Somente adapters/foundation conhecem detalhes das crates. Product code
//! > depende de contratos `Petunia*`, nunca de uma crate específica.
//!
//! Proibido:
//!
//! ```text
//! properties_panel.rs → egui_dnd
//! settings_modal.rs   → twill
//! outliner.rs         → egui_ltreeview
//! ```
//!
//! Permitido:
//!
//! ```text
//! properties_panel.rs → crate::adapters::drag_drop::PetuniaDragList
//! settings_modal.rs   → crate::foundation::theme::token
//! outliner.rs         → crate::adapters::tree::TreeView
//! ```
//!
//! O confinamento não é convenção — é verificável:
//!
//! ```bash
//! cargo run -p xtask -- ui-guard --strict
//! ```
//!
//! ## Mapa
//!
//! | Módulo | Crate | Papel |
//! | --- | --- | --- |
//! | [`taffy_layout`] | `egui_taffy` | layout responsivo (flex/block/grid/wrap) |
//! | [`tree`] | `egui_ltreeview` | árvore Parts/Scene |
//! | [`twill_tokens`] | `twill` | tokens tipados → tokens Petunia |
//! | [`inbox`] | `egui_inbox` | resultado de worker → UI |
//! | [`file_dialog`] | `egui-file-dialog` | browse/save de arquivos |
//! | [`gizmo`] | `transform-gizmo` | gizmo 3D de transformação |
//! | [`icons`] | `iconflow` | packs de ícones genéricos |
//! | [`toolbar`] | `egui_taffy` | barra responsiva por faixas (§46) |
//! | [`top_bar`] | `egui_taffy` | header de três zonas com centro geométrico (§25) |
//! | [`tile_layout`] | `egui_tiles` | macro-layout do shell (§31, §47) |
//! | [`tool_grid`] | `egui_taffy` | grade da paleta lateral, colunas derivadas (§48) |
//! | [`form`] | `egui_taffy` + `egui_form` | campos rotulados de Settings e validação por campo (§48, §51) |
//! | [`drag_drop`] | `egui_dnd` | reordenação por arrasto em listas ordenáveis (§49) |
//! | [`popup`] | `egui` (`Window`) | superfícies flutuantes com perda de foco (§34) |
//!
//! Ainda **não** existentes (dependência não instalada — ver
//! `docs/dependencies/ui-ecosystem-lock.md`): `table` (`egui_table`),
//! `virtual_collection` (`egui_virtual_list`), `suspense` (`egui_suspense`).
//! Nenhum arquivo vazio é criado: um adapter só nasce quando a crate entra e o
//! gate de compatibilidade (§18) passa.
//!
//! [`form`] acumulou dois motores no mesmo contrato: o **arranjo** é do
//! `egui_taffy` (Wave 6, `label_min` + `control_min` decidem o empilhamento) e a
//! **validação** é do `egui_form` (Wave 7/9, erro por campo e estado de
//! revelação). Settings não foi reescrito para receber a segunda camada.

pub mod drag_drop;
pub mod file_dialog;
pub mod form;
pub mod gizmo;
pub mod icons;
pub mod inbox;
pub mod popup;
pub mod taffy_layout;
pub mod tile_layout;
pub mod tool_grid;
pub mod toolbar;
pub mod top_bar;
pub mod tree;
pub mod twill_tokens;

pub use drag_drop::{PetuniaDragList, PetuniaDragRow, PetuniaDragSpec};
pub use form::{PetuniaForm, PetuniaFormPlacement, PetuniaFormRow, PetuniaFormSpec};
pub use popup::{PetuniaPopup, PetuniaPopupLabels, PetuniaPopupMode, PetuniaPopupStyle};
pub use taffy_layout::{
    PetuniaAlign, PetuniaCentering, PetuniaGap, PetuniaJustify, PetuniaLayoutMode,
};
pub use tile_layout::{PetuniaLayoutAdapter, PetuniaPane, PetuniaShellLayout};
pub use tool_grid::{PetuniaToolCell, PetuniaToolGridSpec};
pub use toolbar::{
    PetuniaResponsiveToolbar, PetuniaToolbarCluster, PetuniaToolbarId, PetuniaToolbarPlan,
    PetuniaToolbarPriority, PetuniaToolbarSlot, PetuniaToolbarSpec,
};
pub use top_bar::{PetuniaTopBar, PetuniaTopBarPlan, PetuniaTopBarSlot, PetuniaTopBarSpec};
