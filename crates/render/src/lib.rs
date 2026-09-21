//! Tipos compartilhados de render: shading, stats e capabilities (§29-30).
//! Somente `render-gl` / `render-wgpu` executam chamadas gráficas (§27).
//!
//! Matemática de cena (grid, quads de referência, luz) vive aqui para os
//! dois backends usarem a mesma fonte (sem duplicação).

pub mod scene;

/// Viewport shading. Defined in core (session state); re-exported here so
/// existing `petunia_render::Shading` call sites keep compiling.
pub use petunia_core::Shading;

/// Contadores do frame para overlay e benchmarks.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub tris: usize,
    pub draws: usize,
    pub tex_uploads: usize,
}

/// Capacidades detectadas na inicialização (§29).
#[derive(Debug, Clone)]
pub struct GpuCaps {
    pub gl_version: String,
    pub glsl_version: String,
    pub vendor: String,
    pub renderer: String,
    pub max_texture_size: i32,
}

impl Default for GpuCaps {
    fn default() -> Self {
        Self {
            gl_version: "desconhecido".into(),
            glsl_version: "desconhecido".into(),
            vendor: "desconhecido".into(),
            renderer: "desconhecido".into(),
            max_texture_size: 2048,
        }
    }
}
