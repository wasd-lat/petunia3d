//! Petunia3D — projeto autocontido: assets com UUID persistente,
//! serialização versionada (postcard) e exportação (OBJ, glTF/GLB).

use petunia_mesh::Mesh;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod animation;
pub mod autosave;
pub mod export;
pub mod format;
pub mod import_gltf;
pub mod import_obj;
pub mod io_atomic;
pub mod material;
pub mod model_library;
pub mod package;
pub mod paint_layers;
pub mod palette;
pub mod pipeline;
pub mod rig;
pub mod surface_recipe;

pub use animation::{
    AnimationAsset, AnimationClip, AnimationLibrary, BoneTrack, Interpolation, Keyframe,
    RetargetProfile, RigPreset, auto_fit_humanoid, compute_auto_skin_weights,
};
pub use autosave::{AutosaveConfig, AutosaveService, RecoveryInfo, SessionLockInfo};
pub use export::{ExportError, export_gltf, export_obj};
pub use import_gltf::{GlbMeshes, GltfImportError, GltfSummary, import_glb_bytes, parse_gltf_json};
pub use import_obj::{ObjImportError, import_obj_bytes};
pub use io_atomic::{AtomicIoError, TempScope, atomic_write};
pub use material::{AlphaMode, Material, ShaderProfile, TextureChannel};
pub use model_library::{AssetSummary, ModelLibraryQuery, ModelLibraryService, ModelLibrarySort};
pub use package::{
    Attachment, PACKAGE_VERSION, PackageError, PackageManifest, open_package, open_package_bytes,
    save_package, save_package_bytes,
};
pub use paint_layers::{
    DecalLayer, LayerBlendMode, LayerKind, PaintEffect, PaintLayer, PaintLayerStack, apply_effect,
    blend_pixels,
};
pub use palette::{export_gpl, export_hex, import_gpl, import_hex, preset_gameboy, preset_pico8};
pub use pipeline::{
    BatchExportReport, DeliveryPipeline, ExportOptions, ExportReport, FileFormat,
    FormatCapabilities, FormatExporter, FormatImporter, ImportOptions, ImportPayload,
    PipelineError,
};
pub use rig::{Bone, RigError, Skeleton, SkinData, Transform3D, VertexSkinWeight};
pub use surface_recipe::{
    NodeSpec, RECIPE_SCHEMA_VERSION, RecipeEdge, RecipeError, RecipeNode, RecipeOutputChannel,
    RecipeResult, SocketType, SocketValue, SurfaceRecipe,
};

/// Canvas de textura simples (albedo) por asset — workspace PAINT.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    /// RGBA8 row-major, origem em cima.
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(w: u32, h: u32, fill: [u8; 4]) -> Self {
        let w = w.clamp(1, 1024);
        let h = h.clamp(1, 1024);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for i in (0..pixels.len()).step_by(4) {
            pixels[i..i + 4].copy_from_slice(&fill);
        }
        Self { w, h, pixels }
    }

    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let i = ((y * self.w + x) * 4) as usize;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }

    pub fn set(&mut self, x: u32, y: u32, c: [u8; 4]) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        self.pixels[i..i + 4].copy_from_slice(&c);
    }

    pub fn fill(&mut self, c: [u8; 4]) {
        for i in (0..self.pixels.len()).step_by(4) {
            self.pixels[i..i + 4].copy_from_slice(&c);
        }
    }

    /// Redimensiona por vizinho mais próximo (operação explícita de resize;
    /// preserva o conteúdo proporcionalmente, sem filtros caros).
    pub fn resized(&self, w: u32, h: u32) -> Self {
        let w = w.clamp(1, 1024);
        let h = h.clamp(1, 1024);
        if w == self.w && h == self.h {
            return self.clone();
        }
        let mut out = Self::new(w, h, [0, 0, 0, 0]);
        for y in 0..h {
            for x in 0..w {
                let sx = ((x as f32 * self.w as f32) / w as f32) as u32;
                let sy = ((y as f32 * self.h as f32) / h as f32) as u32;
                if let Some(px) = self.get(sx.min(self.w - 1), sy.min(self.h - 1)) {
                    out.set(x, y, px);
                }
            }
        }
        out
    }

    /// Repara canvas vindo de arquivo (M4): dims 1..1024 + pixels exatos.
    pub fn validate(&mut self) {
        self.w = self.w.clamp(1, 1024);
        self.h = self.h.clamp(1, 1024);
        let want = (self.w * self.h * 4) as usize;
        self.pixels.resize(want, 0);
    }
}

/// Operação não destrutiva persistente avaliada sobre a malha-base do asset.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ModifierKind {
    Mirror {
        axis: usize,
        weld: f32,
    },
    Symmetry {
        axis: usize,
        positive_to_negative: bool,
        weld: f32,
    },
}

/// Instância ordenada de modifier. O UUID mantém identidade estável para UI,
/// reordenação e futuras animações/serialization migrations.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ModifierInstance {
    pub id: Uuid,
    pub enabled: bool,
    pub kind: ModifierKind,
}

impl ModifierInstance {
    pub fn mirror(axis: usize, weld: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            kind: ModifierKind::Mirror {
                axis: axis.min(2),
                weld: weld.max(0.0),
            },
        }
    }

    pub fn symmetry(axis: usize, positive_to_negative: bool, weld: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            kind: ModifierKind::Symmetry {
                axis: axis.min(2),
                positive_to_negative,
                weld: weld.max(0.0),
            },
        }
    }
}

/// Um asset do projeto. `id` nunca muda (rename seguro).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub name: String,
    pub mesh: Mesh,
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub collection: Option<String>,
    pub base_color: [f32; 3],
    pub texture: Option<Canvas>,
    /// Pilha de camadas de pintura (P3D-061). `None` = legado/sem camadas:
    /// a `texture` é a representação composta.
    #[serde(default)]
    pub paint_stack: Option<PaintLayerStack>,
    #[serde(default)]
    pub material_id: Option<Uuid>,
    #[serde(default)]
    pub skeleton_id: Option<Uuid>,
    #[serde(default)]
    pub skin_data: Option<SkinData>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub modifiers: Vec<ModifierInstance>,
    #[serde(skip)]
    eval_cache: Option<(u64, u64, Mesh)>,
}

impl Asset {
    pub fn new(name: &str, mesh: Mesh) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            mesh,
            visible: true,
            locked: false,
            collection: None,
            base_color: [0.75, 0.75, 0.78],
            texture: None,
            material_id: None,
            skeleton_id: None,
            skin_data: None,
            favorite: false,
            tags: Vec::new(),
            modifiers: Vec::new(),
            paint_stack: None,
            eval_cache: None,
        }
    }

    /// Obtém o material atribuído ao asset a partir do projeto.
    pub fn material<'a>(&self, project: &'a Project) -> Option<&'a Material> {
        self.material_id.and_then(|id| project.get_material(id))
    }

    /// Avalia a pilha de modifiers sem alterar a malha-base.
    /// Render, preview e export usam este resultado; edição continua operando
    /// sobre `mesh`, preservando a natureza não destrutiva da pilha.
    pub fn evaluated_mesh(&self) -> Mesh {
        let key = (
            self.mesh.verts.len() as u64 * 1_000_003 + self.mesh.faces.len() as u64,
            self.modifiers.len() as u64,
        );
        if let Some((k0, k1, cached)) = &self.eval_cache
            && (*k0, *k1) == key
        {
            return cached.clone();
        }
        let mut mesh = self.mesh.clone();
        for modifier in &self.modifiers {
            if !modifier.enabled {
                continue;
            }
            match modifier.kind {
                ModifierKind::Mirror { axis, weld } => mesh.mirror(axis, weld),
                ModifierKind::Symmetry {
                    axis,
                    positive_to_negative,
                    weld,
                } => {
                    mesh.symmetrize(axis, positive_to_negative, weld);
                }
            }
        }
        mesh
    }

    /// Cache-aware evaluation. Callers with `&mut Asset` reuse the last result.
    pub fn evaluated_mesh_cached(&mut self) -> &Mesh {
        let key = (
            self.mesh.verts.len() as u64 * 1_000_003 + self.mesh.faces.len() as u64,
            self.modifiers.len() as u64,
        );
        let miss = self
            .eval_cache
            .as_ref()
            .is_none_or(|(k0, k1, _)| (*k0, *k1) != key);
        if miss {
            let mesh = self.evaluated_mesh();
            self.eval_cache = Some((key.0, key.1, mesh));
        }
        &self.eval_cache.as_ref().unwrap().2
    }

    /// Duplicata com novo UUID.
    pub fn duplicate(&self) -> Self {
        let mut c = self.clone();
        c.id = Uuid::new_v4();
        c.name = format!("{} copy", self.name);
        c
    }

    /// Adiciona uma tag normalizada (minúscula, sem espaços extras).
    pub fn add_tag(&mut self, tag: &str) -> bool {
        let trimmed = tag.trim().to_lowercase();
        if trimmed.is_empty() || self.tags.iter().any(|t| t.to_lowercase() == trimmed) {
            return false;
        }
        self.tags.push(trimmed);
        true
    }

    /// Remove uma tag.
    pub fn remove_tag(&mut self, tag: &str) {
        let trimmed = tag.trim().to_lowercase();
        self.tags.retain(|t| t.to_lowercase() != trimmed);
    }

    /// Verifica se possui determinada tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        let trimmed = tag.trim().to_lowercase();
        self.tags.iter().any(|t| t.to_lowercase() == trimmed)
    }

    /// Alterna estado de favorito.
    pub fn toggle_favorite(&mut self) {
        self.favorite = !self.favorite;
    }
}

fn default_palette() -> Vec<[f32; 3]> {
    vec![
        [1.0, 0.2, 0.2],
        [1.0, 0.8, 0.2],
        [0.2, 0.8, 0.3],
        [0.3, 0.5, 1.0],
    ]
}

fn default_stroke_color() -> [f32; 4] {
    [0.0, 0.74, 0.83, 1.0] // Ciano característico do Blender
}

fn default_stroke_width() -> f32 {
    2.0
}

const fn default_true() -> bool {
    true
}

const fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

/// Traço de anotação livre em espaço 3D (ferramenta Annotate).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnnotationStroke {
    pub points: Vec<[f32; 3]>,
    #[serde(default = "default_stroke_color")]
    pub color: [f32; 4],
    #[serde(default = "default_stroke_width")]
    pub width: f32,
}

impl Default for AnnotationStroke {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            color: default_stroke_color(),
            width: default_stroke_width(),
        }
    }
}

/// Item de anotação pertencente à collection de Anotações do projeto.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnnotationItem {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub group: Option<String>,
    pub strokes: Vec<AnnotationStroke>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub translation: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3], // Graus de Euler XYZ
    #[serde(default = "default_scale")]
    pub scale: [f32; 3], // [1.0, 1.0, 1.0]
}

impl AnnotationItem {
    pub fn new(name: impl Into<String>, strokes: Vec<AnnotationStroke>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            group: None,
            strokes,
            visible: true,
            locked: false,
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn transform_matrix(&self) -> glam::Mat4 {
        let t = glam::Vec3::from(self.translation);
        let r = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            self.rotation[0].to_radians(),
            self.rotation[1].to_radians(),
            self.rotation[2].to_radians(),
        );
        let s = glam::Vec3::from(self.scale);
        glam::Mat4::from_scale_rotation_translation(s, r, t)
    }

    pub fn transform_point(&self, pt: [f32; 3]) -> [f32; 3] {
        let m = self.transform_matrix();
        let v = m.transform_point3(glam::Vec3::from(pt));
        [v.x, v.y, v.z]
    }

    pub fn center(&self) -> [f32; 3] {
        let mut sum = glam::Vec3::ZERO;
        let mut count = 0;
        let m = self.transform_matrix();
        for s in &self.strokes {
            for &pt in &s.points {
                sum += m.transform_point3(glam::Vec3::from(pt));
                count += 1;
            }
        }
        if count > 0 {
            let avg = sum / (count as f32);
            [avg.x, avg.y, avg.z]
        } else {
            self.translation
        }
    }
}

/// Item de medição tridimensional com distância euclidiana e deltas cartesianos.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MeasurementItem {
    pub id: Uuid,
    pub name: String,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub distance: f32,
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl MeasurementItem {
    pub fn new(name: impl Into<String>, start: [f32; 3], end: [f32; 3], distance: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            start,
            end,
            distance,
            visible: true,
        }
    }

    pub fn deltas(&self) -> [f32; 3] {
        [
            (self.end[0] - self.start[0]).abs(),
            (self.end[1] - self.start[1]).abs(),
            (self.end[2] - self.start[2]).abs(),
        ]
    }
}

fn default_project_name() -> String {
    "Untitled".to_string()
}

/// Projeto: metadados, lista de assets com UUID persistente, paleta, coleções, anotações e medições.
/// Tipo de luz de cena (P3D-134). V1 tem direcional; os demais entram depois.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LightKind {
    /// Direcional: só a direção importa, como o sol.
    #[default]
    Directional,
    /// Ponto: posição + alcance.
    Point,
}

impl LightKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Directional => "directional",
            Self::Point => "point",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "directional" => Some(Self::Directional),
            "point" => Some(Self::Point),
            _ => None,
        }
    }
}

/// Luz de cena usada pelo modo Rendered da viewport.
///
/// A direção é normalizada no renderer; `intensity` escala a contribuição
/// difusa e `color` tinge a luz. Sem luz habilitada o Rendered cai no estúdio
/// da viewport em vez de renderizar preto.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub id: Uuid,
    pub name: String,
    pub kind: LightKind,
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub enabled: bool,
}

impl Light {
    pub fn directional(name: impl Into<String>, direction: [f32; 3]) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind: LightKind::Directional,
            direction,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            enabled: true,
        }
    }

    /// Direção normalizada, caindo no eixo Y quando degenerada.
    pub fn normalized_direction(&self) -> [f32; 3] {
        let [x, y, z] = self.direction;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return [0.0, 1.0, 0.0];
        }
        let length = (x * x + y * y + z * z).sqrt();
        if length <= 1.0e-5 {
            return [0.0, 1.0, 0.0];
        }
        [x / length, y / length, z / length]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "default_project_name")]
    pub name: String,
    pub assets: Vec<Asset>,
    pub active: usize,
    /// Transient selection context carried by in-memory undo snapshots. The
    /// document file keeps selection in the editor session, not authored data.
    #[serde(skip)]
    pub history_selection: Vec<Uuid>,
    #[serde(default = "default_palette")]
    pub palette: Vec<[f32; 3]>,
    #[serde(default)]
    pub collections: Vec<String>,
    #[serde(default)]
    pub annotations: Vec<AnnotationItem>,
    #[serde(default)]
    pub annotation_groups: Vec<String>,
    #[serde(default)]
    pub measurements: Vec<MeasurementItem>,
    #[serde(default = "default_true")]
    pub annotations_visible: bool,
    #[serde(default)]
    pub annotations_locked: bool,
    #[serde(default = "default_true")]
    pub measurements_visible: bool,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub lights: Vec<Light>,
    #[serde(default)]
    pub skeletons: Vec<Skeleton>,
    #[serde(default)]
    pub animations: Vec<AnimationAsset>,
    /// Scene-level revision counters for GPU invalidation (not hashed content).
    #[serde(default)]
    pub topology_revision: u64,
    #[serde(default)]
    pub position_revision: u64,
    #[serde(default)]
    pub selection_revision: u64,
    #[serde(default)]
    pub material_revision: u64,
    #[serde(default)]
    pub texture_revision: u64,
    #[serde(default)]
    pub transform_revision: u64,
}

impl Project {
    /// Primeira luz habilitada da cena, se houver.
    pub fn active_light(&self) -> Option<&Light> {
        self.lights.iter().find(|light| light.enabled)
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: default_project_name(),
            assets: Vec::new(),
            active: 0,
            history_selection: Vec::new(),
            palette: default_palette(),
            collections: Vec::new(),
            annotations: Vec::new(),
            annotation_groups: Vec::new(),
            measurements: Vec::new(),
            annotations_visible: true,
            annotations_locked: false,
            measurements_visible: true,
            materials: vec![Material::new("Default Material")],
            // Uma direcional padrão: o modo Rendered precisa de luz real, e sem
            // nenhuma a cena renderizaria preta.
            lights: vec![Light::directional("Key Light", [0.4, 0.9, 0.6])],
            skeletons: Vec::new(),
            animations: Vec::new(),
            topology_revision: 0,
            position_revision: 0,
            selection_revision: 0,
            material_revision: 0,
            texture_revision: 0,
            transform_revision: 0,
        }
    }
}

impl Project {
    pub fn bump_topology(&mut self) {
        self.topology_revision = self.topology_revision.wrapping_add(1);
    }
    pub fn bump_positions(&mut self) {
        self.position_revision = self.position_revision.wrapping_add(1);
    }
    pub fn bump_selection(&mut self) {
        self.selection_revision = self.selection_revision.wrapping_add(1);
    }
    pub fn bump_materials(&mut self) {
        self.material_revision = self.material_revision.wrapping_add(1);
    }
    pub fn bump_textures(&mut self) {
        self.texture_revision = self.texture_revision.wrapping_add(1);
    }

    /// Approximate owned payload size for history eviction (meshes + textures).
    pub fn estimated_bytes(&self) -> usize {
        let mut n = std::mem::size_of::<Self>();
        for asset in &self.assets {
            n = n.saturating_add(asset.mesh.verts.len().saturating_mul(32));
            n = n.saturating_add(asset.mesh.faces.len().saturating_mul(48));
            if let Some(tex) = asset.texture.as_ref() {
                n = n.saturating_add(tex.pixels.len());
            }
            if let Some(stack) = asset.paint_stack.as_ref() {
                for layer in &stack.layers {
                    if let Some(cv) = layer.canvas() {
                        n = n.saturating_add(cv.pixels.len());
                    }
                }
            }
        }
        for mat in &self.materials {
            if let Some(tex) = mat.albedo_texture.as_ref() {
                n = n.saturating_add(tex.pixels.len());
            }
        }
        n.max(1)
    }

    pub fn new() -> Self {
        let def_mat = Material::new("Default Material");
        let def_mat_id = def_mat.id;
        let mut cube = Asset::new("Cube", Mesh::cube(2.0));
        cube.material_id = Some(def_mat_id);
        Self {
            id: Uuid::new_v4(),
            name: default_project_name(),
            assets: vec![cube],
            active: 0,
            history_selection: Vec::new(),
            palette: default_palette(),
            collections: Vec::new(),
            annotations: Vec::new(),
            annotation_groups: Vec::new(),
            measurements: Vec::new(),
            annotations_visible: true,
            annotations_locked: false,
            measurements_visible: true,
            materials: vec![def_mat],
            lights: vec![Light::directional("Key Light", [0.4, 0.9, 0.6])],
            skeletons: Vec::new(),
            animations: Vec::new(),
            topology_revision: 0,
            position_revision: 0,
            selection_revision: 0,
            material_revision: 0,
            texture_revision: 0,
            transform_revision: 0,
        }
    }

    pub fn get_skeleton(&self, id: Uuid) -> Option<&Skeleton> {
        self.skeletons.iter().find(|s| s.id == id)
    }

    pub fn get_skeleton_mut(&mut self, id: Uuid) -> Option<&mut Skeleton> {
        self.skeletons.iter_mut().find(|s| s.id == id)
    }

    pub fn add_skeleton(&mut self, skeleton: Skeleton) {
        self.skeletons.push(skeleton);
    }

    pub fn remove_skeleton(&mut self, id: Uuid) {
        self.skeletons.retain(|s| s.id != id);
        for a in &mut self.assets {
            if a.skeleton_id == Some(id) {
                a.skeleton_id = None;
                a.skin_data = None;
            }
        }
    }

    pub fn get_animation(&self, id: Uuid) -> Option<&AnimationAsset> {
        self.animations.iter().find(|a| a.id == id)
    }

    pub fn get_animation_mut(&mut self, id: Uuid) -> Option<&mut AnimationAsset> {
        self.animations.iter_mut().find(|a| a.id == id)
    }

    pub fn add_animation(&mut self, animation: AnimationAsset) {
        self.animations.push(animation);
    }

    pub fn remove_animation(&mut self, id: Uuid) {
        self.animations.retain(|a| a.id != id);
    }

    pub fn add_annotation(&mut self, item: AnnotationItem) {
        self.annotations.push(item);
    }

    pub fn remove_annotation(&mut self, id: Uuid) {
        self.annotations.retain(|a| a.id != id);
    }

    pub fn add_annotation_group(&mut self, name: &str) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() || self.annotation_groups.iter().any(|g| g == trimmed) {
            return false;
        }
        self.annotation_groups.push(trimmed.to_string());
        true
    }

    pub fn remove_annotation_group(&mut self, name: &str) {
        self.annotation_groups.retain(|g| g != name);
        for a in &mut self.annotations {
            if a.group.as_deref() == Some(name) {
                a.group = None;
            }
        }
    }

    pub fn add_measurement(&mut self, item: MeasurementItem) {
        self.measurements.push(item);
    }

    pub fn remove_measurement(&mut self, id: Uuid) {
        self.measurements.retain(|m| m.id != id);
    }

    pub fn add_collection(&mut self, name: &str) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() || self.collections.iter().any(|c| c == trimmed) {
            return false;
        }
        self.collections.push(trimmed.to_string());
        true
    }

    pub fn remove_collection(&mut self, name: &str) {
        self.collections.retain(|c| c != name);
        for a in &mut self.assets {
            if a.collection.as_deref() == Some(name) {
                a.collection = None;
            }
        }
    }

    pub fn active(&self) -> Option<&Asset> {
        self.assets.get(self.active)
    }
    pub fn active_mesh(&self) -> Option<&Mesh> {
        self.active().map(|o| &o.mesh)
    }

    pub fn active_mut(&mut self) -> Option<&mut Asset> {
        self.assets.get_mut(self.active)
    }
    pub fn active_mesh_mut(&mut self) -> Option<&mut Mesh> {
        self.active_mut().map(|o| &mut o.mesh)
    }

    pub fn add(&mut self, name: &str, mesh: Mesh) {
        self.assets.push(Asset::new(name, mesh));
        self.active = self.assets.len() - 1;
    }

    pub fn remove(&mut self, i: usize) {
        if i >= self.assets.len() {
            return;
        }
        let removed = self.assets.remove(i);
        self.history_selection.retain(|id| *id != removed.id);
        if self.assets.is_empty() || self.active == usize::MAX {
            self.active = usize::MAX;
        } else if i < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.assets.len() - 1);
        }
    }

    /// Reordena um asset da posição `from` para a posição `to`, mantendo o asset ativo selecionado.
    /// Reorders an asset from position `from` to position `to`, preserving the active asset selection.
    pub fn reorder_asset(&mut self, from: usize, to: usize) -> bool {
        if from >= self.assets.len() || to >= self.assets.len() || from == to {
            return false;
        }
        let active_id = self.assets.get(self.active).map(|a| a.id);
        let asset = self.assets.remove(from);
        self.assets.insert(to, asset);
        if let Some(id) = active_id
            && let Some(idx) = self.find(id)
        {
            self.active = idx;
        }
        true
    }

    pub fn totals(&self) -> (usize, usize) {
        let (mut v, mut f) = (0, 0);
        for o in &self.assets {
            v += o.mesh.vert_count();
            f += o.mesh.tri_count();
        }
        (v, f)
    }

    pub fn find(&self, id: Uuid) -> Option<usize> {
        self.assets.iter().position(|a| a.id == id)
    }

    /// Localiza asset por ID estável retornando índice e referência.
    pub fn find_by_id(&self, id: Uuid) -> Option<(usize, &Asset)> {
        self.assets.iter().enumerate().find(|(_, a)| a.id == id)
    }

    /// Localiza asset por ID estável retornando índice e referência mutável.
    pub fn find_by_id_mut(&mut self, id: Uuid) -> Option<(usize, &mut Asset)> {
        self.assets.iter_mut().enumerate().find(|(_, a)| a.id == id)
    }

    /// Remove asset por ID estável mantendo invariants de seleção.
    pub fn remove_by_id(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.find(id) {
            self.remove(pos);
            true
        } else {
            false
        }
    }

    /// Duplica asset por ID gerando novo UUID persistente e ativando-o.
    pub fn duplicate_by_id(&mut self, id: Uuid) -> Option<Uuid> {
        let dup = {
            let (_, asset) = self.find_by_id(id)?;
            asset.duplicate()
        };
        let new_id = dup.id;
        self.assets.push(dup);
        self.active = self.assets.len() - 1;
        Some(new_id)
    }

    // ---- Material Management (P3D-050) ----

    pub fn add_material(&mut self, mat: Material) -> Uuid {
        let id = mat.id;
        self.materials.push(mat);
        id
    }

    pub fn get_material(&self, id: Uuid) -> Option<&Material> {
        self.materials.iter().find(|m| m.id == id)
    }

    pub fn get_material_mut(&mut self, id: Uuid) -> Option<&mut Material> {
        self.materials.iter_mut().find(|m| m.id == id)
    }

    pub fn remove_material(&mut self, id: Uuid) -> bool {
        let before = self.materials.len();
        self.materials.retain(|m| m.id != id);
        let removed = self.materials.len() < before;
        if removed {
            for a in &mut self.assets {
                if a.material_id == Some(id) {
                    a.material_id = None;
                }
            }
        }
        removed
    }

    pub fn active_material(&self) -> Option<&Material> {
        let active_asset = self.active()?;
        active_asset
            .material(self)
            .or_else(|| self.materials.first())
    }

    pub fn active_material_mut(&mut self) -> Option<&mut Material> {
        let mat_id = self
            .active()
            .and_then(|a| a.material_id)
            .or_else(|| self.materials.first().map(|m| m.id))?;
        self.get_material_mut(mat_id)
    }

    /// Normaliza projeto vindo de arquivo (M2/M3): malhas válidas,
    /// no mínimo 1 asset, `active` dentro dos limites e materiais íntegros (P3D-050).
    pub fn validate(&mut self) {
        for mat in &mut self.materials {
            mat.validate();
        }
        if self.materials.is_empty() {
            self.materials.push(Material::new("Default Material"));
        }
        let fallback_mat_id = self.materials[0].id;

        for a in &mut self.assets {
            a.mesh.validate();
            if let Some(cv) = a.texture.as_mut() {
                cv.validate();
            }
            if !a.base_color.iter().all(|x| x.is_finite()) {
                a.base_color = [0.75, 0.75, 0.78];
            }
            if a.material_id.is_none()
                || !self.materials.iter().any(|m| Some(m.id) == a.material_id)
            {
                a.material_id = Some(fallback_mat_id);
            }
        }
        // An empty document and an explicitly cleared object selection are valid.
        if self.assets.is_empty() {
            self.active = usize::MAX;
        } else if self.active != usize::MAX {
            self.active = self.active.min(self.assets.len() - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_remove_shifts_active_index() {
        let mut p = Project::new();
        p.add("Asset 1", Mesh::cube(1.0));
        p.add("Asset 2", Mesh::cube(1.0));
        assert_eq!(p.assets.len(), 3);
        p.active = 2; // Asset 2 active

        // Remove Asset 0 (before active)
        p.remove(0);
        assert_eq!(p.assets.len(), 2);
        assert_eq!(p.active, 1);
        assert_eq!(p.assets[p.active].name, "Asset 2");

        // Remove active asset
        p.remove(1);
        assert_eq!(p.assets.len(), 1);
        assert_eq!(p.active, 0);
        assert_eq!(p.assets[p.active].name, "Asset 1");
    }

    #[test]
    fn test_annotation_item_transform() {
        let mut stroke = AnnotationStroke::default();
        stroke.points.push([1.0, 0.0, 0.0]);
        stroke.points.push([3.0, 0.0, 0.0]);

        let mut item = AnnotationItem::new("Note 1", vec![stroke]);
        item.translation = [10.0, 5.0, 2.0];
        item.scale = [2.0, 2.0, 2.0];

        let p0 = item.transform_point([1.0, 0.0, 0.0]);
        assert_eq!(p0, [12.0, 5.0, 2.0]);

        let center = item.center();
        assert_eq!(center, [14.0, 5.0, 2.0]);
    }

    #[test]
    fn test_project_annotations_and_measurements_management() {
        let mut p = Project::new();
        assert!(p.annotations.is_empty());
        assert!(p.measurements.is_empty());

        let a_item = AnnotationItem::new("A1", vec![AnnotationStroke::default()]);
        let a_id = a_item.id;
        p.add_annotation(a_item);
        assert_eq!(p.annotations.len(), 1);

        assert!(p.add_annotation_group("Rascunhos"));
        p.annotations[0].group = Some("Rascunhos".to_string());
        p.remove_annotation_group("Rascunhos");
        assert!(p.annotations[0].group.is_none());

        p.remove_annotation(a_id);
        assert!(p.annotations.is_empty());

        let m_item = MeasurementItem::new("M1", [0.0, 0.0, 0.0], [1.0, 2.0, 2.0], 3.0);
        let m_id = m_item.id;
        p.add_measurement(m_item);
        assert_eq!(p.measurements.len(), 1);
        assert_eq!(p.measurements[0].deltas(), [1.0, 2.0, 2.0]);

        p.remove_measurement(m_id);
        assert!(p.measurements.is_empty());
    }
}
