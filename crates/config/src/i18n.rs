//! i18n via TOML — melhor encaixe para Rust:
//! - `serde + toml` é idiomático, tipado e com comentários (#),
//! - menos verboso que JSON, sem as armadilhas do YAML (spec gigante),
//! - um arquivo por idioma em `locales/*.toml`, fácil de traduzir.
//!
//! Formato: tabelas viram prefixo com ponto. Ex.:
//! ```toml
//! [ui]
//! title = "Simple3D"
//! [tools]
//! select = "Selecionar"
//! ```
//! vira chaves `ui.title`, `tools.select` via `t()`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};

static AVAILABLE_LOCALES: LazyLock<RwLock<Vec<String>>> =
    LazyLock::new(|| RwLock::new(discover_available_locales()));

pub struct I18n {
    pub lang: String,
    map: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl I18n {
    /// Lista de idiomas já descoberta em cache. O filesystem não é tocado no
    /// hot path de menus egui; o watcher chama [`Self::refresh_available`].
    pub fn available() -> Vec<String> {
        match AVAILABLE_LOCALES.read() {
            Ok(locales) => locales.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Reescaneia os arquivos de locale. Deve ser chamado somente em eventos de
    /// mudança do watcher ou por ações explícitas de atualização.
    pub fn refresh_available() {
        let locales = discover_available_locales();
        match AVAILABLE_LOCALES.write() {
            Ok(mut cached) => *cached = locales,
            Err(poisoned) => *poisoned.into_inner() = locales,
        }
    }

    pub fn load(lang: &str) -> Self {
        let fallback = load_file("en");
        let map = if lang == "en" {
            fallback.clone()
        } else {
            let m = load_file(lang);
            if m.is_empty() { fallback.clone() } else { m }
        };
        Self {
            lang: lang.to_string(),
            map,
            fallback,
        }
    }

    pub fn set_lang(&mut self, lang: &str) {
        *self = Self::load(lang);
    }

    /// Traduz `chave`. Cai para inglês e depois para a própria chave.
    /// Retorna `String` (não `&str`) de propósito: evita borrow persistente
    /// de `app` dentro dos closures do egui.
    pub fn t(&self, key: &str) -> String {
        self.map
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Traduz um [`TextId`] tipado (Wave 7).
    pub fn t_id(&self, id: TextId) -> String {
        self.t(id.key())
    }

    /// Constrói de um texto TOML (testes e pseudo-locale, sem disco).
    pub fn parse(text: &str) -> HashMap<String, String> {
        let v: toml::Value = toml::from_str(text).unwrap_or(toml::Value::Table(Default::default()));
        let mut map = HashMap::new();
        flatten(&v, String::new(), &mut map);
        map
    }

    /// Pseudo-locale de teste (Wave 7 — §12.4): expande ~40% e acentua para
    /// expor clipping, larguras fixas e overflow de shelf/dropdowns.
    /// Uso exclusivo em testes; nunca embarca.
    pub fn pseudo_from(base: &Self) -> Self {
        let map = base
            .map
            .iter()
            .map(|(k, v)| (k.clone(), pseudo_transform(v)))
            .collect();
        let fallback = base
            .fallback
            .iter()
            .map(|(k, v)| (k.clone(), pseudo_transform(v)))
            .collect();
        Self {
            lang: "pseudo".to_string(),
            map,
            fallback,
        }
    }

    /// Chaves do mapa principal (auditoria de paridade).
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.map.keys().cloned().collect();
        keys.sort();
        keys
    }
}

/// Expansão pseudo-locale: prefixo/sufixo visíveis + alongamento + acentos.
fn pseudo_transform(text: &str) -> String {
    const ACCENTS: &[char] = &['é', 'ñ', 'ü', 'ß', 'ç', 'ø'];
    let mut out = String::with_capacity(text.len() * 2 + 4);
    out.push('⟦');
    for (i, ch) in text.chars().enumerate() {
        out.push(ch);
        if ch.is_ascii_alphabetic() && i % 2 == 0 {
            out.push(ch);
        }
        if ch == ' ' {
            out.push(ACCENTS[i % ACCENTS.len()]);
        }
    }
    out.push('⟧');
    out
}

/// Identificador de tradução tipado (Wave 7 — §12.2).
///
/// Evita chaves stringly-typed espalhadas: o catálogo vive em [`text_id`] e o
/// teste de paridade garante resolução em todos os locales embutidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextId(pub &'static str);

impl TextId {
    pub const fn new(key: &'static str) -> Self {
        Self(key)
    }

    pub fn key(self) -> &'static str {
        self.0
    }
}

/// Catálogo de `TextId` das superfícies migradas (Waves 2–7).
///
/// Chaves pré-existentes em formato string continuam válidas (compatível);
/// superfícies novas usam estas constantes.
pub mod text_id {
    use super::TextId;

    pub const UI_OUTLINER: TextId = TextId::new("ui.outliner");
    pub const UI_PROPERTIES: TextId = TextId::new("ui.properties");
    pub const UI_COLLAPSE: TextId = TextId::new("ui.collapse");
    pub const UI_EXPAND: TextId = TextId::new("ui.expand");
    pub const UI_DOCK_SPLIT_HINT: TextId = TextId::new("ui.dock_split_hint");
    pub const UI_MORE: TextId = TextId::new("ui.more");
    pub const UI_TOOLS_MENU: TextId = TextId::new("ui.tools_menu");
    pub const UI_DUPLICATE: TextId = TextId::new("ui.duplicate");
    pub const UI_REFS: TextId = TextId::new("ui.refs");
    pub const UI_ASSETS: TextId = TextId::new("ui.assets");
    pub const UI_CLOSE: TextId = TextId::new("ui.close");
    pub const UI_FLOATING_INSPECTOR: TextId = TextId::new("ui.floating_inspector");
    pub const UI_REDOCK: TextId = TextId::new("ui.redock");
    pub const UI_AT_3D_CURSOR: TextId = TextId::new("ui.at_3d_cursor");

    // Chrome do shell Slint: títulos de painel, botões e dicas de status.
    pub const UI_PARTS: TextId = TextId::new("ui.parts");
    pub const UI_PROJECT_ASSET_LIBRARY: TextId = TextId::new("ui.project_asset_library");
    pub const UI_SAVE_ACTIVE_AS_ASSET: TextId = TextId::new("ui.save_active_as_asset");
    pub const UI_STATUS_HINT: TextId = TextId::new("ui.status_hint");
    pub const UI_UNWRAP_MESH: TextId = TextId::new("ui.unwrap_mesh");
    pub const UI_PACK_ISLANDS: TextId = TextId::new("ui.pack_islands");
    pub const UI_ACTIVE_BRUSH_COLOR: TextId = TextId::new("ui.active_brush_color");
    pub const UI_ALBEDO_BASE_COLOR: TextId = TextId::new("ui.albedo_base_color");
    pub const UI_THEME: TextId = TextId::new("ui.theme");
    pub const UI_PLACE_IN_SCENE: TextId = TextId::new("ui.place_in_scene");

    // Diálogo de recuperação de autosave (P3D-002).
    pub const UI_RECOVERY_TITLE: TextId = TextId::new("ui.recovery_title");
    pub const UI_RECOVERY_BODY: TextId = TextId::new("ui.recovery_body");
    pub const UI_RECOVERY_RECOVER: TextId = TextId::new("ui.recovery_recover");
    pub const UI_RECOVERY_KEEP: TextId = TextId::new("ui.recovery_keep");
    pub const UI_RECOVERY_DISCARD: TextId = TextId::new("ui.recovery_discard");
    pub const ACTIONS_APPLY: TextId = TextId::new("actions.apply");
    pub const ACTIONS_CANCEL: TextId = TextId::new("actions.cancel");
    pub const ACTIONS_DELETE: TextId = TextId::new("actions.delete");
    pub const ACTIONS_DUPLICATE: TextId = TextId::new("actions.duplicate");

    // Barra de menus do shell e itens que ela publica. As chaves de locale já
    // existiam; o que faltava era o vínculo tipado que o shell consome.
    pub const MENU_FILE: TextId = TextId::new("menu.file");
    pub const MENU_EDIT: TextId = TextId::new("menu.edit");
    pub const MENU_VIEW: TextId = TextId::new("menu.view");
    pub const MENU_WINDOW: TextId = TextId::new("menu.window");
    pub const MENU_COMMAND_PALETTE: TextId = TextId::new("menu.command_palette");
    pub const MENU_PREFERENCES: TextId = TextId::new("menu.preferences");
    pub const FILE_NEW: TextId = TextId::new("file.new");
    pub const FILE_OPEN_PROJECT: TextId = TextId::new("file.open_project");
    pub const FILE_SAVE: TextId = TextId::new("file.save");
    pub const FILE_SAVE_AS: TextId = TextId::new("file.save_as");
    pub const FILE_IMPORT_OBJ: TextId = TextId::new("file.import_obj");
    pub const EDIT_UNDO: TextId = TextId::new("edit.undo");
    pub const EDIT_REDO: TextId = TextId::new("edit.redo");
    pub const VIEW_FRAME: TextId = TextId::new("view.frame");
    pub const VIEW_FRAME_ALL: TextId = TextId::new("view.frame_all");
    pub const VIEW_TOGGLE_PROJECTION: TextId = TextId::new("camera.projection");
    pub const VIEW_RESET_CAMERA: TextId = TextId::new("camera.reset");
    pub const VIEW_TOGGLE_WIREFRAME: TextId = TextId::new("shading.wire");

    pub const UV_TITLE: TextId = TextId::new("uv.title");
    pub const UV_SELECTED: TextId = TextId::new("uv.selected");
    pub const UV_FACES: TextId = TextId::new("uv.faces");
    pub const UV_PREVIEW_3D: TextId = TextId::new("uv.preview_3d");
    pub const UV_HINT: TextId = TextId::new("uv.hint");
    pub const UV_ROTATE_90: TextId = TextId::new("uv.rotate_90");
    pub const UV_ROTATE_90_TIP: TextId = TextId::new("uv.rotate_90_tip");
    pub const UV_ROTATE_CCW: TextId = TextId::new("uv.rotate_ccw");
    pub const UV_ROTATE_CW: TextId = TextId::new("uv.rotate_cw");
    pub const UV_ROTATE_CCW_TIP: TextId = TextId::new("uv.rotate_ccw_tip");
    pub const UV_ROTATE_CW_TIP: TextId = TextId::new("uv.rotate_cw_tip");
    pub const UV_TOGGLE_SEAMS: TextId = TextId::new("uv.toggle_seams");
    pub const UV_TOGGLE_SEAMS_TIP: TextId = TextId::new("uv.toggle_seams_tip");
    pub const UV_CLEAR_SEAMS: TextId = TextId::new("uv.clear_seams");
    pub const UV_CLEAR_SEAMS_TIP: TextId = TextId::new("uv.clear_seams_tip");
    pub const UV_SEAMS_TOGGLED: TextId = TextId::new("uv.seams_toggled");
    pub const UV_SEAMS_NO_FACE: TextId = TextId::new("uv.seams_no_face");
    pub const UV_SEAMS_CLEARED: TextId = TextId::new("uv.seams_cleared");

    pub const BOOLEAN_TITLE: TextId = TextId::new("boolean.title");
    pub const BOOLEAN_OPERAND: TextId = TextId::new("boolean.operand");
    pub const BOOLEAN_SET_OPERAND: TextId = TextId::new("boolean.set_operand");
    pub const BOOLEAN_NO_OPERAND: TextId = TextId::new("boolean.no_operand");
    pub const BOOLEAN_KEEP_PARTS: TextId = TextId::new("boolean.keep_parts");
    pub const BOOLEAN_FUSE: TextId = TextId::new("boolean.fuse");
    pub const BOOLEAN_CUT: TextId = TextId::new("boolean.cut");
    pub const BOOLEAN_INTERSECT: TextId = TextId::new("boolean.intersect");
    pub const BOOLEAN_JOIN: TextId = TextId::new("boolean.join");
    pub const BOOLEAN_CLEAR_OPERAND: TextId = TextId::new("boolean.clear_operand");
    pub const BOOLEAN_OPERAND_SET: TextId = TextId::new("boolean.operand_set");
    pub const BOOLEAN_COMMAND_FAILED: TextId = TextId::new("boolean.command_failed");
    pub const BOOLEAN_FUSE_RESULT: TextId = TextId::new("boolean.fuse_result");
    pub const BOOLEAN_CUT_RESULT: TextId = TextId::new("boolean.cut_result");
    pub const BOOLEAN_INTERSECT_RESULT: TextId = TextId::new("boolean.intersect_result");
    pub const BOOLEAN_JOIN_RESULT: TextId = TextId::new("boolean.join_result");

    pub const PAINT_FILL_SCOPE: TextId = TextId::new("paint.fill_scope");
    pub const PAINT_FILL_CONNECTED_PIXELS: TextId = TextId::new("paint.fill_connected_pixels");
    pub const PAINT_FILL_FACE: TextId = TextId::new("paint.fill_face");
    pub const PAINT_FILL_SELECTED_FACES: TextId = TextId::new("paint.fill_selected_faces");
    pub const PAINT_FILL_UV_ISLAND: TextId = TextId::new("paint.fill_uv_island");
    pub const PAINT_FILL_OBJECT: TextId = TextId::new("paint.fill_object");
    pub const PAINT_VIEW_MODE: TextId = TextId::new("paint.view_mode");
    pub const PAINT_VIEW_3D: TextId = TextId::new("paint.view_3d");
    pub const PAINT_VIEW_2D: TextId = TextId::new("paint.view_2d");
    pub const PAINT_VIEW_SPLIT: TextId = TextId::new("paint.view_split");

    pub const ANIMATE_HUMANOID: TextId = TextId::new("animate.humanoid");
    pub const ANIMATE_AUTO_RIG: TextId = TextId::new("animate.auto_rig");
    pub const ANIMATE_PLAY: TextId = TextId::new("animate.play");
    pub const ANIMATE_PAUSE: TextId = TextId::new("animate.pause");
    pub const ANIMATE_FIRST_FRAME: TextId = TextId::new("animate.first_frame");
    pub const ANIMATE_LAST_FRAME: TextId = TextId::new("animate.last_frame");
    pub const ANIMATE_FRAME: TextId = TextId::new("animate.frame");
    pub const ANIMATE_TIP_FIRST: TextId = TextId::new("animate.tip_first");
    pub const ANIMATE_TIP_PREV: TextId = TextId::new("animate.tip_prev");
    pub const ANIMATE_TIP_PLAY: TextId = TextId::new("animate.tip_play");
    pub const ANIMATE_TIP_NEXT: TextId = TextId::new("animate.tip_next");
    pub const ANIMATE_TIP_LAST: TextId = TextId::new("animate.tip_last");

    pub const PAINT_RADIUS: TextId = TextId::new("paint.radius");
    pub const PAINT_COLOR: TextId = TextId::new("paint.color");

    pub const SETTINGS_INTERFACE: TextId = TextId::new("settings.interface");
    pub const SETTINGS_IMPORT_EXPORT: TextId = TextId::new("settings.import_export");
    pub const SETTINGS_SHOW_SHELF: TextId = TextId::new("settings.show_shelf");
    pub const SETTINGS_RESET_WORKSPACE: TextId = TextId::new("settings.reset_workspace");
    pub const SETTINGS_RESET_ALL: TextId = TextId::new("settings.reset_all_layouts");
    pub const SETTINGS_EXPORT_GLB: TextId = TextId::new("settings.export_glb");
    pub const SETTINGS_EXPORT_GLB_HINT: TextId = TextId::new("settings.export_glb_hint");

    pub const REFS_VISIBLE: TextId = TextId::new("refs.visible");
    pub const REFS_LOCK: TextId = TextId::new("refs.lock");
    pub const REFS_CLICK_TO_LOAD: TextId = TextId::new("refs.click_to_load");
    pub const REFS_NO_IMAGE: TextId = TextId::new("refs.no_image");
    pub const REFS_REPLACE: TextId = TextId::new("refs.replace");
    pub const REFS_REMOVE: TextId = TextId::new("refs.remove");
    pub const REFS_ALIGN_VIEW: TextId = TextId::new("refs.align_view");
    pub const REFS_RESET_DEFAULT: TextId = TextId::new("refs.reset_default");
    pub const REFS_FINE_TUNE: TextId = TextId::new("refs.fine_tune");
    pub const REFS_LOADED: TextId = TextId::new("refs.loaded");
    pub const REFS_OPACITY: TextId = TextId::new("refs.opacity");
    pub const REFS_SIZE: TextId = TextId::new("refs.size");
    pub const REFS_OFFSET: TextId = TextId::new("refs.offset");
    pub const REFS_ROTATION: TextId = TextId::new("refs.rotation");

    pub const PRIMS_CUBE: TextId = TextId::new("prims.cube");
    pub const PRIMS_PLANE: TextId = TextId::new("prims.plane");
    pub const PRIMS_CYLINDER: TextId = TextId::new("prims.cylinder");
    pub const PRIMS_SPHERE: TextId = TextId::new("prims.sphere");
    pub const PRIMS_CONE: TextId = TextId::new("prims.cone");
    pub const PRIMS_CAPSULE: TextId = TextId::new("prims.capsule");
    pub const PRIMS_SIZE: TextId = TextId::new("prims.size");
    pub const PRIMS_RADIUS: TextId = TextId::new("prims.radius");
    pub const PRIMS_SEGMENTS: TextId = TextId::new("prims.segments");
    pub const PRIMS_RINGS: TextId = TextId::new("prims.rings");
    pub const PRIMS_HEIGHT: TextId = TextId::new("prims.height");
    pub const PRIMS_SIDES: TextId = TextId::new("prims.sides");
    pub const PRIMS_WIDTH: TextId = TextId::new("prims.width");
    pub const PRIMS_CONFIRM: TextId = TextId::new("prims.confirm");
    pub const PRIMS_CANCEL: TextId = TextId::new("prims.cancel");
    pub const PRIMS_REOPEN: TextId = TextId::new("prims.reopen");
    pub const PRIMS_CONFIRM_HINT: TextId = TextId::new("prims.confirm_hint");
    pub const PRIMS_WEDGE: TextId = TextId::new("prims.wedge");
    pub const PRIMS_CIRCLE: TextId = TextId::new("prims.circle");
    pub const PRIMS_TORUS: TextId = TextId::new("prims.torus");
    pub const PRIMS_ICOSPHERE: TextId = TextId::new("prims.icosphere");
    pub const PRIMS_GROUP_BASIC: TextId = TextId::new("prims.group_basic");
    pub const PRIMS_GROUP_ROUND: TextId = TextId::new("prims.group_round");
    pub const PRIMS_GROUP_ORGANIC: TextId = TextId::new("prims.group_organic");
    pub const PRIMS_DEPTH: TextId = TextId::new("prims.depth");
    pub const PRIMS_TOP_RADIUS: TextId = TextId::new("prims.top_radius");
    pub const PRIMS_BOTTOM_RADIUS: TextId = TextId::new("prims.bottom_radius");
    pub const PRIMS_MAJOR_RADIUS: TextId = TextId::new("prims.major_radius");
    pub const PRIMS_MINOR_RADIUS: TextId = TextId::new("prims.minor_radius");
    pub const PRIMS_VERTICES: TextId = TextId::new("prims.vertices");
    pub const PRIMS_FILL: TextId = TextId::new("prims.fill");
    pub const PRIMS_CAP: TextId = TextId::new("prims.cap");
    pub const PRIMS_SUBDIV: TextId = TextId::new("prims.subdivision");
    pub const PRIMS_BODY_LENGTH: TextId = TextId::new("prims.body_length");
    pub const PRIMS_RESET: TextId = TextId::new("prims.reset");
    pub const PRIMS_TRIS: TextId = TextId::new("prims.tris");
    pub const PRIMS_CAP_BOTH: TextId = TextId::new("prims.cap_both");
    pub const PRIMS_CAP_TOP: TextId = TextId::new("prims.cap_top_only");
    pub const PRIMS_CAP_BOTTOM: TextId = TextId::new("prims.cap_bottom_only");
    pub const PRIMS_CAP_NONE: TextId = TextId::new("prims.cap_none");
    pub const PRIMS_FILL_NONE: TextId = TextId::new("prims.fill_none");
    pub const PRIMS_FILL_DISC: TextId = TextId::new("prims.fill_disc");
    pub const PRIMS_TIP_SIDES: TextId = TextId::new("prims.tip_sides");
    pub const PRIMS_TIP_TOP_RADIUS: TextId = TextId::new("prims.tip_top_radius");
    pub const PRIMS_TIP_SUBDIV: TextId = TextId::new("prims.tip_subdiv");
    pub const PRIMS_TIP_MAJOR_RADIUS: TextId = TextId::new("prims.tip_major_radius");
    pub const PRIMS_TIP_MINOR_RADIUS: TextId = TextId::new("prims.tip_minor_radius");
    pub const PRIMS_TIP_SEGMENTS: TextId = TextId::new("prims.tip_segments");
    pub const PRIMS_TIP_RINGS: TextId = TextId::new("prims.tip_rings");
    pub const PRIMS_TIP_VERTICES: TextId = TextId::new("prims.tip_vertices");
    pub const PRIMS_TIP_FILL: TextId = TextId::new("prims.tip_fill");
    pub const PRIMS_TIP_CAPS: TextId = TextId::new("prims.tip_caps");
    pub const PRIMS_TIP_BODY_LENGTH: TextId = TextId::new("prims.tip_body_length");

    /// Todos os ids do catálogo (cobertura de tradução).
    pub const ALL: &[TextId] = &[
        UI_OUTLINER,
        UI_PROPERTIES,
        UI_COLLAPSE,
        UI_EXPAND,
        UI_DOCK_SPLIT_HINT,
        UI_MORE,
        UI_TOOLS_MENU,
        UI_DUPLICATE,
        UI_REFS,
        UI_ASSETS,
        UI_CLOSE,
        UI_FLOATING_INSPECTOR,
        UI_REDOCK,
        UI_AT_3D_CURSOR,
        UI_PARTS,
        UI_PROJECT_ASSET_LIBRARY,
        UI_SAVE_ACTIVE_AS_ASSET,
        UI_STATUS_HINT,
        UI_UNWRAP_MESH,
        UI_PACK_ISLANDS,
        UI_ACTIVE_BRUSH_COLOR,
        UI_ALBEDO_BASE_COLOR,
        UI_THEME,
        UI_PLACE_IN_SCENE,
        UI_RECOVERY_TITLE,
        UI_RECOVERY_BODY,
        UI_RECOVERY_RECOVER,
        UI_RECOVERY_KEEP,
        UI_RECOVERY_DISCARD,
        ACTIONS_APPLY,
        ACTIONS_CANCEL,
        ACTIONS_DELETE,
        ACTIONS_DUPLICATE,
        MENU_FILE,
        MENU_EDIT,
        MENU_VIEW,
        MENU_WINDOW,
        MENU_COMMAND_PALETTE,
        MENU_PREFERENCES,
        FILE_NEW,
        FILE_OPEN_PROJECT,
        FILE_SAVE,
        FILE_SAVE_AS,
        FILE_IMPORT_OBJ,
        EDIT_UNDO,
        EDIT_REDO,
        VIEW_FRAME,
        VIEW_FRAME_ALL,
        VIEW_TOGGLE_PROJECTION,
        VIEW_RESET_CAMERA,
        VIEW_TOGGLE_WIREFRAME,
        UV_TITLE,
        UV_SELECTED,
        UV_FACES,
        UV_PREVIEW_3D,
        UV_HINT,
        UV_ROTATE_90,
        UV_ROTATE_90_TIP,
        UV_ROTATE_CCW,
        UV_ROTATE_CW,
        UV_ROTATE_CCW_TIP,
        UV_ROTATE_CW_TIP,
        UV_TOGGLE_SEAMS,
        UV_TOGGLE_SEAMS_TIP,
        UV_CLEAR_SEAMS,
        UV_CLEAR_SEAMS_TIP,
        UV_SEAMS_TOGGLED,
        UV_SEAMS_NO_FACE,
        UV_SEAMS_CLEARED,
        BOOLEAN_TITLE,
        BOOLEAN_OPERAND,
        BOOLEAN_SET_OPERAND,
        BOOLEAN_NO_OPERAND,
        BOOLEAN_KEEP_PARTS,
        BOOLEAN_FUSE,
        BOOLEAN_CUT,
        BOOLEAN_INTERSECT,
        BOOLEAN_JOIN,
        BOOLEAN_CLEAR_OPERAND,
        BOOLEAN_OPERAND_SET,
        BOOLEAN_COMMAND_FAILED,
        BOOLEAN_FUSE_RESULT,
        BOOLEAN_CUT_RESULT,
        BOOLEAN_INTERSECT_RESULT,
        BOOLEAN_JOIN_RESULT,
        ANIMATE_HUMANOID,
        ANIMATE_AUTO_RIG,
        ANIMATE_PLAY,
        ANIMATE_PAUSE,
        ANIMATE_FIRST_FRAME,
        ANIMATE_LAST_FRAME,
        ANIMATE_FRAME,
        ANIMATE_TIP_FIRST,
        ANIMATE_TIP_PREV,
        ANIMATE_TIP_PLAY,
        ANIMATE_TIP_NEXT,
        ANIMATE_TIP_LAST,
        PAINT_RADIUS,
        PAINT_COLOR,
        PAINT_FILL_SCOPE,
        PAINT_FILL_CONNECTED_PIXELS,
        PAINT_FILL_FACE,
        PAINT_FILL_SELECTED_FACES,
        PAINT_FILL_UV_ISLAND,
        PAINT_FILL_OBJECT,
        PAINT_VIEW_MODE,
        PAINT_VIEW_3D,
        PAINT_VIEW_2D,
        PAINT_VIEW_SPLIT,
        SETTINGS_INTERFACE,
        SETTINGS_IMPORT_EXPORT,
        SETTINGS_SHOW_SHELF,
        SETTINGS_RESET_WORKSPACE,
        SETTINGS_RESET_ALL,
        SETTINGS_EXPORT_GLB,
        SETTINGS_EXPORT_GLB_HINT,
        REFS_VISIBLE,
        REFS_LOCK,
        REFS_CLICK_TO_LOAD,
        REFS_NO_IMAGE,
        REFS_REPLACE,
        REFS_REMOVE,
        REFS_ALIGN_VIEW,
        REFS_RESET_DEFAULT,
        REFS_FINE_TUNE,
        REFS_LOADED,
        REFS_OPACITY,
        REFS_SIZE,
        REFS_OFFSET,
        REFS_ROTATION,
        PRIMS_SIZE,
        PRIMS_RADIUS,
        PRIMS_SEGMENTS,
        PRIMS_RINGS,
        PRIMS_HEIGHT,
        PRIMS_SIDES,
        PRIMS_WIDTH,
        PRIMS_CONFIRM,
        PRIMS_CANCEL,
        PRIMS_REOPEN,
        PRIMS_CONFIRM_HINT,
        PRIMS_CUBE,
        PRIMS_PLANE,
        PRIMS_CYLINDER,
        PRIMS_SPHERE,
        PRIMS_CONE,
        PRIMS_CAPSULE,
        PRIMS_WEDGE,
        PRIMS_CIRCLE,
        PRIMS_TORUS,
        PRIMS_ICOSPHERE,
        PRIMS_GROUP_BASIC,
        PRIMS_GROUP_ROUND,
        PRIMS_GROUP_ORGANIC,
        PRIMS_DEPTH,
        PRIMS_TOP_RADIUS,
        PRIMS_BOTTOM_RADIUS,
        PRIMS_MAJOR_RADIUS,
        PRIMS_MINOR_RADIUS,
        PRIMS_VERTICES,
        PRIMS_FILL,
        PRIMS_CAP,
        PRIMS_SUBDIV,
        PRIMS_BODY_LENGTH,
        PRIMS_RESET,
        PRIMS_TRIS,
        PRIMS_CAP_BOTH,
        PRIMS_CAP_TOP,
        PRIMS_CAP_BOTTOM,
        PRIMS_CAP_NONE,
        PRIMS_FILL_NONE,
        PRIMS_FILL_DISC,
        PRIMS_TIP_SIDES,
        PRIMS_TIP_TOP_RADIUS,
        PRIMS_TIP_SUBDIV,
        PRIMS_TIP_MAJOR_RADIUS,
        PRIMS_TIP_MINOR_RADIUS,
        PRIMS_TIP_SEGMENTS,
        PRIMS_TIP_RINGS,
        PRIMS_TIP_VERTICES,
        PRIMS_TIP_FILL,
        PRIMS_TIP_CAPS,
        PRIMS_TIP_BODY_LENGTH,
    ];
}

fn locales_dir() -> PathBuf {
    // 1. ./assets/locales (dev, executando da raiz)
    // 2. ao lado do executável (instalado)
    for cand in ["assets/locales", "locales"] {
        let p = PathBuf::from(cand);
        if p.exists() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for cand in ["assets/locales", "locales", "../../assets/locales"] {
            let p = dir.join(cand);
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from("assets/locales")
}

fn discover_available_locales() -> Vec<String> {
    let mut out = vec!["en".to_string()];
    if let Ok(entries) = fs::read_dir(locales_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml")
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
                && stem != "en"
                && !out.iter().any(|locale| locale == stem)
            {
                out.push(stem.to_string());
            }
        }
    }
    out.sort();
    out
}

/// Locales embutidos como reserva (Wave 7): binários instalados sem o diretório
/// de locales e testes executados fora da raiz nunca voltam a exibir chaves.
const EN_EMBEDDED: &str = include_str!("../../../assets/locales/en.toml");
const PT_BR_EMBEDDED: &str = include_str!("../../../assets/locales/pt-BR.toml");

fn embedded(lang: &str) -> &'static str {
    match lang {
        "pt-BR" => PT_BR_EMBEDDED,
        _ => EN_EMBEDDED,
    }
}

fn load_file(lang: &str) -> HashMap<String, String> {
    let path = locales_dir().join(format!("{lang}.toml"));
    let text = fs::read_to_string(&path).unwrap_or_default();
    if text.is_empty() {
        return I18n::parse(embedded(lang));
    }
    I18n::parse(&text)
}

fn flatten(v: &toml::Value, prefix: String, out: &mut HashMap<String, String>) {
    match v {
        toml::Value::Table(t) => {
            for (k, vv) in t {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(vv, key, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix, s.clone());
        }
        other => {
            out.insert(prefix, other.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::text_id::ALL;
    use super::{I18n, pseudo_transform};

    const EN_TOML: &str = include_str!("../../../assets/locales/en.toml");
    const PT_TOML: &str = include_str!("../../../assets/locales/pt-BR.toml");

    fn keys_of(toml_text: &str) -> Vec<String> {
        let mut keys: Vec<String> = I18n::parse(toml_text).keys().cloned().collect();
        keys.sort();
        keys
    }

    #[test]
    fn locale_parity_en_ptbr() {
        // Paridade exata: CI falha com chave faltante ou órfã (Wave 7 — §12.3).
        let en = keys_of(EN_TOML);
        let pt = keys_of(PT_TOML);
        let missing: Vec<&String> = en.iter().filter(|k| !pt.contains(k)).collect();
        let orphan: Vec<&String> = pt.iter().filter(|k| !en.contains(k)).collect();
        assert!(missing.is_empty(), "pt-BR sem tradução: {missing:?}");
        assert!(orphan.is_empty(), "pt-BR com chaves órfãs: {orphan:?}");
        assert!(!en.is_empty());
    }

    #[test]
    fn text_id_catalog_resolves_in_both_locales() {
        let en = I18n::parse(EN_TOML);
        let pt = I18n::parse(PT_TOML);
        assert_eq!(en.len(), pt.len());
        for id in ALL {
            assert!(en.contains_key(id.key()), "en sem {}", id.key());
            assert!(pt.contains_key(id.key()), "pt-BR sem {}", id.key());
            assert!(!en[id.key()].is_empty() && !pt[id.key()].is_empty());
        }
    }

    #[test]
    fn pseudo_expands_and_marks() {
        let out = pseudo_transform("Settings");
        assert!(out.starts_with('⟦') && out.ends_with('⟧'));
        assert!(out.len() > "Settings".len());
        // Texto vazio não quebra.
        assert_eq!(pseudo_transform(""), "⟦⟧");
    }

    #[test]
    fn pseudo_locale_covers_catalog() {
        let base = I18n {
            lang: "en".to_string(),
            map: I18n::parse(EN_TOML),
            fallback: I18n::parse(EN_TOML),
        };
        let pseudo = I18n::pseudo_from(&base);
        assert_eq!(pseudo.lang, "pseudo");
        for id in ALL {
            let s = pseudo.t_id(*id);
            assert!(s.starts_with('⟦'), "{} sem marca pseudo", id.key());
        }
    }
}
