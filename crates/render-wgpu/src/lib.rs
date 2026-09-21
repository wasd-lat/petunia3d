//! Renderer wgpu: malha sombreada + arestas + grid estilo Blender + quads de referência.
//! Buffers de malha/arestas/referência são revisionados: câmera atualiza só o
//! uniform; geometria só reconstrói quando o fingerprint da cena muda (Wave 1).

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use petunia_core::Camera;
use petunia_core::RefAxis;
use petunia_core::{FingerprintFlags, SceneFingerprint, fingerprint_scene};
use petunia_project::Project;
use petunia_render::Shading;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MeshVertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    pos: [f32; 3],
    color: [f32; 3],
}

/// Vértice da camada de seleção: posição + cor com alpha.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SelectionVertex {
    pos: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    /// xyz = direção da luz (normalizada), w = livre.
    light_dir: [f32; 4],
    /// x = ambiente, y = difusa, z/w = livres.
    light_params: [f32; 4],
    /// x = alpha do X-Ray, y/z/w = livres.
    xray: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RefVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RefUniform {
    opacity: f32,
    _pad: [f32; 3],
}

struct RefGpu {
    width: u32,
    height: u32,
    len: usize,
    hash: std::cell::Cell<u64>,
    texture: wgpu::Texture,
    /// Mantida viva junto ao bind group (wgpu é ref-counted, mas explícito é mais seguro).
    #[allow(dead_code)]
    view: wgpu::TextureView,
    params: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

/// Slot de textura do canvas de um asset (paridade com o backend GL:
/// recria em resize, re-upload só com conteúdo novo via hash).
struct AssetTexGpu {
    asset_id: uuid::Uuid,
    width: u32,
    height: u32,
    hash: std::cell::Cell<u64>,
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
}

/// Faixa de vértices de um asset no VB único + textura (paridade GL).
struct MeshRange {
    start: u32,
    count: u32,
    asset_id: Option<uuid::Uuid>,
}

pub struct Renderer {
    depth_format: wgpu::TextureFormat,
    depth_view: Option<wgpu::TextureView>,
    depth_size: (u32, u32),
    mesh_pipeline: wgpu::RenderPipeline,
    mesh_xray_pipeline: wgpu::RenderPipeline,
    mesh_tex_pipeline: wgpu::RenderPipeline,
    mesh_tex_xray_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    line_xray_pipeline: wgpu::RenderPipeline,
    xray: bool,
    xray_opacity: f32,
    ref_pipeline: wgpu::RenderPipeline,
    ref_xray_pipeline: wgpu::RenderPipeline,
    cam_buffer: wgpu::Buffer,
    cam_bind_group: wgpu::BindGroup,
    ref_tex_layout: wgpu::BindGroupLayout,
    mesh_tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    mesh_vb: Option<wgpu::Buffer>,
    mesh_count: u32,
    mesh_ranges: Vec<MeshRange>,
    asset_tex: Vec<AssetTexGpu>,
    line_vb: Option<wgpu::Buffer>,
    line_count: u32,
    selection_tri_pipeline: wgpu::RenderPipeline,
    selection_line_pipeline: wgpu::RenderPipeline,
    /// Preenchimento translúcido das faces selecionadas (depth test, sem write).
    selection_tri_vb: Option<wgpu::Buffer>,
    selection_tri_count: u32,
    /// Contorno e marcadores da seleção (depth test).
    selection_line_vb: Option<wgpu::Buffer>,
    selection_line_count: u32,
    grid_vb: wgpu::Buffer,
    grid_count: u32,
    ref_vb: Option<wgpu::Buffer>,
    ref_count: u32,
    ref_gpu: Vec<RefGpu>,
    pub show_overlays: bool,
    pub show_grid: bool,
    last_fingerprint: Option<SceneFingerprint>,
    last_domain: Option<petunia_core::SelectionDomain>,
    mesh_rebuilds: u64,
    skipped_frames: u64,
}

/// Seleção: cor chapada, sem iluminação, com alpha. A seleção precisa ser
/// legível sobre qualquer shading e nunca depender da luz da cena.
const SELECTION_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: In) -> Out {
    var out: Out;
    out.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

const MESH_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) wpos: vec3<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.color = in.color;
    o.wpos = in.pos;
    return o;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    if (length(in.normal) < 0.1) {
        return vec4<f32>(in.color, 1.0);
    }
    let light = normalize(cam.light_dir.xyz);
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let c = in.color * (cam.light_params.x + cam.light_params.y * diff);
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_xray(in: Out) -> @location(0) vec4<f32> {
    if (length(in.normal) < 0.1) {
        return vec4<f32>(in.color, cam.xray.x);
    }
    let light = normalize(vec3<f32>(LIGHT_X, LIGHT_Y, LIGHT_Z));
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let amb = LIGHT_AMB;
    let c = in.color * (amb + LIGHT_DIF * diff);
    return vec4<f32>(c, cam.xray.x);
}
"#;

/// Variante texturizada da malha (paridade com o backend GL): multiplica a
/// cor do vértice pelo texel do canvas do asset. Fora do modo texturizado
/// (ou sem canvas) o range usa o pipeline de cor sólida.
const MESH_TEX_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) wpos: vec3<f32>,
    @location(3) uv: vec2<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.color = in.color;
    o.wpos = in.pos;
    o.uv = in.uv;
    return o;
}
"#;

/// Monta o shader texturizado com as constantes de luz compartilhadas.
fn mesh_tex_wgsl() -> String {
    const FRAG: &str = r#"
@fragment
fn fs_tex(in: Out) -> @location(0) vec4<f32> {
    let t = textureSample(tex, smp, in.uv);
    let base = in.color * t.rgb;
    if (length(in.normal) < 0.1) {
        return vec4<f32>(base, 1.0);
    }
    let light = normalize(cam.light_dir.xyz);
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let c = base * (cam.light_params.x + cam.light_params.y * diff);
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_tex_xray(in: Out) -> @location(0) vec4<f32> {
    let t = textureSample(tex, smp, in.uv);
    let base = in.color * t.rgb;
    if (length(in.normal) < 0.1) {
        return vec4<f32>(base, cam.xray.x);
    }
    let light = normalize(vec3<f32>(LIGHT_X, LIGHT_Y, LIGHT_Z));
    let n = normalize(in.normal);
    let diff = max(dot(n, light), 0.0);
    let amb = LIGHT_AMB;
    let c = base * (amb + LIGHT_DIF * diff);
    return vec4<f32>(c, cam.xray.x);
}
"#;
    (MESH_TEX_WGSL.to_string() + FRAG)
        .replace("LIGHT_X", &petunia_render::scene::LIGHT_DIR[0].to_string())
        .replace("LIGHT_Y", &petunia_render::scene::LIGHT_DIR[1].to_string())
        .replace("LIGHT_Z", &petunia_render::scene::LIGHT_DIR[2].to_string())
        .replace(
            "LIGHT_AMB",
            &petunia_render::scene::LIGHT_AMBIENT.to_string(),
        )
        .replace(
            "LIGHT_DIF",
            &petunia_render::scene::LIGHT_DIFFUSE.to_string(),
        )
}

/// Monta o shader da malha com as constantes de luz compartilhadas
/// (`render::scene`), mantendo uma fonte só.
fn mesh_wgsl() -> String {
    MESH_WGSL
        .replace("LIGHT_X", &petunia_render::scene::LIGHT_DIR[0].to_string())
        .replace("LIGHT_Y", &petunia_render::scene::LIGHT_DIR[1].to_string())
        .replace("LIGHT_Z", &petunia_render::scene::LIGHT_DIR[2].to_string())
        .replace(
            "LIGHT_AMB",
            &petunia_render::scene::LIGHT_AMBIENT.to_string(),
        )
        .replace(
            "LIGHT_DIF",
            &petunia_render::scene::LIGHT_DIFFUSE.to_string(),
        )
}

const LINE_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.color = in.color;
    return o;
}
@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

const REF_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    light_params: vec4<f32>,
    xray: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
struct Params { opacity: f32, _p0: f32, _p1: f32, _p2: f32 };
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var tex: texture_2d<f32>;
@group(1) @binding(2) var smp: sampler;

struct In {
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
};
struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex
fn vs_main(in: In) -> Out {
    var o: Out;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.uv = in.uv;
    return o;
}
@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, in.uv);
    return vec4<f32>(c.rgb, c.a * params.opacity);
}
"#;

fn grid_lines() -> Vec<LineVertex> {
    petunia_render::scene::grid_lines()
        .into_iter()
        .flat_map(|(a, b, c)| {
            [
                LineVertex { pos: a, color: c },
                LineVertex { pos: b, color: c },
            ]
        })
        .collect()
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let cam_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simple3d-cam"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cam_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("simple3d-cam-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                // O fragment lê a luz e a opacidade de X-Ray deste uniform.
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let cam_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simple3d-cam-bg"),
            layout: &cam_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buffer.as_entire_binding(),
            }],
        });

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-mesh"),
            source: wgpu::ShaderSource::Wgsl(mesh_wgsl().into()),
        });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-line"),
            source: wgpu::ShaderSource::Wgsl(LINE_WGSL.into()),
        });
        let ref_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-ref"),
            source: wgpu::ShaderSource::Wgsl(REF_WGSL.into()),
        });

        let mesh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-mesh-layout"),
            bind_group_layouts: &[Some(&cam_layout)],
            immediate_size: 0,
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-mesh-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-line-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let mesh_xray_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-mesh-xray-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_xray"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let line_xray_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-line-xray-pipe"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // Camada de seleção: um pipeline para o preenchimento translúcido das
        // faces e outro para contorno/marcadores. Ambos com depth test e sem
        // depth write, para não ocluir a geometria nem se sobrepor a si mesmos.
        let selection_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-selection-shader"),
            source: wgpu::ShaderSource::Wgsl(SELECTION_WGSL.into()),
        });
        let selection_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-selection-layout"),
            bind_group_layouts: &[Some(&cam_layout)],
            immediate_size: 0,
        });
        let selection_attrs = [Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SelectionVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
        })];
        let selection_tri_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("simple3d-selection-tri-pipe"),
                layout: Some(&selection_layout),
                vertex: wgpu::VertexState {
                    module: &selection_shader,
                    entry_point: Some("vs_main"),
                    buffers: &selection_attrs,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &selection_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let selection_line_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("simple3d-selection-line-pipe"),
                layout: Some(&selection_layout),
                vertex: wgpu::VertexState {
                    module: &selection_shader,
                    entry_point: Some("vs_main"),
                    buffers: &selection_attrs,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &selection_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });

        // refs: layout do grupo 1 (params + textura + sampler)
        let ref_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("simple3d-ref-tex-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let ref_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-ref-layout"),
            bind_group_layouts: &[Some(&cam_layout), Some(&ref_tex_layout)],
            immediate_size: 0,
        });
        let ref_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-ref-pipe"),
            layout: Some(&ref_layout),
            vertex: wgpu::VertexState {
                module: &ref_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RefVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ref_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let ref_xray_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-ref-xray-pipeline"),
            layout: Some(&ref_layout),
            vertex: wgpu::VertexState {
                module: &ref_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RefVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ref_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("simple3d-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Malha texturizada (paridade GL): grupo 1 = textura + sampler.
        let mesh_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("simple3d-mesh-tex-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let mesh_tex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("simple3d-mesh-tex"),
            source: wgpu::ShaderSource::Wgsl(mesh_tex_wgsl().into()),
        });
        let mesh_tex_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("simple3d-mesh-tex-layout"),
            bind_group_layouts: &[Some(&cam_layout), Some(&mesh_tex_layout)],
            immediate_size: 0,
        });
        let tex_vb_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x2],
        };
        let mesh_tex_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("simple3d-mesh-tex-pipe"),
            layout: Some(&mesh_tex_pipe_layout),
            vertex: wgpu::VertexState {
                module: &mesh_tex_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(tex_vb_layout.clone())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_tex_shader,
                entry_point: Some("fs_tex"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let mesh_tex_xray_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("simple3d-mesh-tex-xray-pipe"),
                layout: Some(&mesh_tex_pipe_layout),
                vertex: wgpu::VertexState {
                    module: &mesh_tex_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(tex_vb_layout)],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mesh_tex_shader,
                    entry_point: Some("fs_tex_xray"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });

        let grid = grid_lines();
        let grid_count = grid.len() as u32;
        let grid_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("simple3d-grid"),
            contents: bytemuck::cast_slice(&grid),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            depth_format: wgpu::TextureFormat::Depth24Plus,
            depth_view: None,
            depth_size: (0, 0),
            mesh_pipeline,
            mesh_xray_pipeline,
            mesh_tex_pipeline,
            mesh_tex_xray_pipeline,
            line_pipeline,
            line_xray_pipeline,
            xray: false,
            xray_opacity: 0.42,
            ref_pipeline,
            ref_xray_pipeline,
            cam_buffer,
            cam_bind_group,
            ref_tex_layout,
            mesh_tex_layout,
            sampler,
            mesh_vb: None,
            mesh_count: 0,
            mesh_ranges: Vec::new(),
            asset_tex: Vec::new(),
            line_vb: None,
            line_count: 0,
            selection_tri_pipeline,
            selection_line_pipeline,
            selection_tri_vb: None,
            selection_tri_count: 0,
            selection_line_vb: None,
            selection_line_count: 0,
            grid_vb,
            grid_count,
            ref_vb: None,
            ref_count: 0,
            ref_gpu: Vec::new(),
            show_overlays: true,
            show_grid: true,
            last_fingerprint: None,
            last_domain: None,
            mesh_rebuilds: 0,
            skipped_frames: 0,
        }
    }

    /// Quantas vezes os buffers de geometria foram reconstruídos (telemetria Wave 1).
    pub fn mesh_rebuilds(&self) -> u64 {
        self.mesh_rebuilds
    }

    /// Quantos `update()` pularam reconstrução por fingerprint idêntico.
    pub fn skipped_frames(&self) -> u64 {
        self.skipped_frames
    }

    /// Invalida o cache manualmente (ex. após troca de backend ou teste).
    pub fn invalidate_cache(&mut self) {
        self.last_fingerprint = None;
    }

    /// Opacidade da geometria em X-Ray, aplicada no uniform do shader.
    pub fn set_xray_opacity(&mut self, opacity: f32) {
        self.xray_opacity = opacity.clamp(0.1, 0.9);
    }

    pub fn set_overlays(&mut self, show_overlays: bool, show_grid: bool) {
        self.show_overlays = show_overlays;
        self.show_grid = show_grid;
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.depth_size == (width, height) && self.depth_view.is_some() {
            return;
        }
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("simple3d-depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_view = Some(tex.create_view(&Default::default()));
        self.depth_size = (width, height);
    }

    /// Atualiza uniforms de câmera todo frame; reconstrói buffers de geometria
    /// somente quando o fingerprint da cena muda (Wave 1 — P0-A).
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Project,
        refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        shading: Shading,
        xray: bool,
        show_triangulation: bool,
        textured: bool,
        show_wireframe_overlay: bool,
        edit_domain: petunia_core::SelectionDomain,
    ) {
        puffin::profile_function!();
        self.xray = xray;
        // Trocar de domínio muda a camada de seleção, não só a malha.
        if self.last_domain != Some(edit_domain) {
            self.last_domain = Some(edit_domain);
            self.last_fingerprint = None;
        }
        // Fonte de luz por modo: Solid/Material usam o estúdio fixo da viewport,
        // Rendered usa a primeira luz habilitada da cena. Sem luz na cena o
        // Rendered cai no estúdio em vez de renderizar preto.
        let (light_dir, ambient, diffuse) = match (shading.uses_scene_light(), scene.active_light())
        {
            (true, Some(light)) => {
                let [x, y, z] = light.normalized_direction();
                let intensity = light.intensity.clamp(0.0, 8.0);
                (
                    [x, y, z, 0.0],
                    petunia_render::scene::LIGHT_AMBIENT * 0.35,
                    petunia_render::scene::LIGHT_DIFFUSE * intensity,
                )
            }
            _ => (
                [
                    petunia_render::scene::LIGHT_DIR[0],
                    petunia_render::scene::LIGHT_DIR[1],
                    petunia_render::scene::LIGHT_DIR[2],
                    0.0,
                ],
                petunia_render::scene::LIGHT_AMBIENT,
                petunia_render::scene::LIGHT_DIFFUSE,
            ),
        };
        queue.write_buffer(
            &self.cam_buffer,
            0,
            bytemuck::cast_slice(&[CameraUniform {
                view_proj: camera.view_proj().to_cols_array_2d(),
                light_dir,
                light_params: [ambient, diffuse, 0.0, 0.0],
                xray: [self.xray_opacity, 0.0, 0.0, 0.0],
            }]),
        );

        let fp = fingerprint_scene(
            scene,
            refs,
            FingerprintFlags {
                shading,
                xray,
                show_triangulation,
                textured,
                edit_mode_is_edit: false,
                show_wireframe_overlay,
            },
        );
        let mesh_changed = self.last_fingerprint.map(|f| f.mesh) != Some(fp.mesh);
        let refs_changed = self.last_fingerprint.map(|f| f.refs_layout) != Some(fp.refs_layout);
        if !mesh_changed && !refs_changed {
            self.skipped_frames += 1;
            return;
        }
        self.last_fingerprint = Some(fp);
        // Qualquer mudança em geometria OU layout de refs reconstrói os buffers
        // de cena; câmera sozinha retorna cedo acima sem triangulação nem alloc.
        self.mesh_rebuilds += 1;

        // malha
        let mut mv: Vec<MeshVertex> = Vec::new();
        let mut lv: Vec<LineVertex> = Vec::new();
        let mut mesh_ranges: Vec<MeshRange> = Vec::new();
        // Wireframe não preenche; os outros três modos preenchem e diferem no
        // que amostram: cor do objeto, textura do material, ou material sob a
        // luz da cena. `smooth` é ortogonal: normais suavizadas por vértice.
        let is_wire = !shading.fills_faces();
        let unlit = false;
        let smooth = false;
        for obj in &scene.assets {
            if !obj.visible {
                continue;
            }
            let range_start = mv.len() as u32;
            let mesh = obj.evaluated_mesh();
            if !is_wire {
                let (mat_profile, mat_color, has_emission, emission_color) =
                    if let Some(mat) = obj.material(scene) {
                        (
                            mat.profile,
                            [mat.base_color[0], mat.base_color[1], mat.base_color[2]],
                            mat.emission_strength > 0.0,
                            [
                                mat.emission_color[0] * mat.emission_strength,
                                mat.emission_color[1] * mat.emission_strength,
                                mat.emission_color[2] * mat.emission_strength,
                            ],
                        )
                    } else {
                        (
                            petunia_project::ShaderProfile::Pbr,
                            obj.base_color,
                            false,
                            [0.0, 0.0, 0.0],
                        )
                    };

                let obj_unlit = unlit
                    || mat_profile == petunia_project::ShaderProfile::Unlit
                    || mat_profile == petunia_project::ShaderProfile::Emissive;

                let tris = if obj_unlit {
                    mesh.to_triangles_unlit()
                } else {
                    mesh.to_triangles_smooth(smooth)
                };
                for (pos, n, mut col, uv) in tris {
                    if (col[0] - 0.72).abs() < 0.02
                        && (col[1] - 0.73).abs() < 0.02
                        && (col[2] - 0.78).abs() < 0.02
                    {
                        col = mat_color;
                    }
                    if has_emission {
                        col = [
                            (col[0] + emission_color[0]).min(1.0),
                            (col[1] + emission_color[1]).min(1.0),
                            (col[2] + emission_color[2]).min(1.0),
                        ];
                    }
                    mv.push(MeshVertex {
                        pos,
                        normal: n,
                        color: col,
                        uv,
                    });
                }
            }
            let range_count = mv.len() as u32 - range_start;
            if range_count > 0 {
                let tex_canvas = obj
                    .texture
                    .as_ref()
                    .or_else(|| obj.material(scene).and_then(|m| m.albedo_texture.as_ref()));
                mesh_ranges.push(MeshRange {
                    start: range_start,
                    count: range_count,
                    // Material Preview e Rendered sempre amostram o material; nos
                    // outros modos a textura é opt-in pelo toggle `textured`.
                    asset_id: if (textured || shading.samples_material()) && tex_canvas.is_some() {
                        Some(obj.id)
                    } else {
                        None
                    },
                });
            }

            if is_wire {
                // Wireframe mostra topologia, não seleção: arestas neutras para
                // a camada de seleção continuar sendo o único destaque.
                for (a, b, sel) in mesh.to_edges() {
                    let c = if sel {
                        [1.0, 0.62, 0.20]
                    } else {
                        [0.62, 0.66, 0.74]
                    };
                    lv.push(LineVertex { pos: a, color: c });
                    lv.push(LineVertex { pos: b, color: c });
                }
            } else if show_wireframe_overlay {
                // Overlay de wireframe é opt-in: sem ele, Solid/Material/Rendered
                // mostram faces limpas e só a camada de seleção destaca arestas.
                for (a, b, _sel) in mesh.to_edges() {
                    let c = [0.05, 0.05, 0.06];
                    lv.push(LineVertex {
                        pos: [a[0], a[1] + 0.001, a[2]],
                        color: c,
                    });
                    lv.push(LineVertex {
                        pos: [b[0], b[1] + 0.001, b[2]],
                        color: c,
                    });
                }
            }
            if show_triangulation {
                let diag_c = [0.3, 0.65, 0.95];
                let lift = if is_wire { 0.0 } else { 0.0012 };
                for (a, b) in mesh.triangulation_wireframe() {
                    lv.push(LineVertex {
                        pos: [a[0], a[1] + lift, a[2]],
                        color: diag_c,
                    });
                    lv.push(LineVertex {
                        pos: [b[0], b[1] + lift, b[2]],
                        color: diag_c,
                    });
                }
            }
        }
        // Camada de seleção: geometria própria, com depth test no render. Só o
        // ativo contribui, e só o domínio atual — um vértice selecionado não
        // pode virar face pintada, que era a contaminação antiga.
        let mut sel_tri: Vec<SelectionVertex> = Vec::new();
        let mut sel_line: Vec<SelectionVertex> = Vec::new();
        if let Some(asset) = scene.assets.get(scene.active) {
            let mesh = asset.evaluated_mesh();
            let domain = edit_domain;
            // Seleção: laranja quente com alpha, como Blender/C4D. Legível
            // sobre qualquer shading porque o shader não aplica luz.
            let face_color = [1.0f32, 0.55, 0.15, 0.32];
            let edge_color = [1.0f32, 0.62, 0.20, 1.0];
            let point_color = [1.0f32, 0.78, 0.35, 1.0];
            let marker = 0.035f32 * scene.assets.len().max(1) as f32;

            if domain == petunia_core::SelectionDomain::Face {
                for face in mesh.faces.iter().filter(|face| face.selected) {
                    if face.verts.len() < 3 {
                        continue;
                    }
                    let p0 = mesh.verts[face.verts[0] as usize].vec();
                    for i in 1..face.verts.len() - 1 {
                        let p1 = mesh.verts[face.verts[i] as usize].vec();
                        let p2 = mesh.verts[face.verts[i + 1] as usize].vec();
                        for point in [p0, p1, p2] {
                            sel_tri.push(SelectionVertex {
                                pos: point.to_array(),
                                color: face_color,
                            });
                        }
                    }
                }
            }

            if domain == petunia_core::SelectionDomain::Edge {
                for &(a, b) in &mesh.selected_edges {
                    let (Some(va), Some(vb)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    else {
                        continue;
                    };
                    sel_line.push(SelectionVertex {
                        pos: va.pos,
                        color: edge_color,
                    });
                    sel_line.push(SelectionVertex {
                        pos: vb.pos,
                        color: edge_color,
                    });
                }
            }

            if domain == petunia_core::SelectionDomain::Vertex {
                for vertex in mesh.verts.iter().filter(|vertex| vertex.selected) {
                    // Cruz 3D: três segmentos curtos, legíveis de qualquer ângulo.
                    for axis in [glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z] {
                        let point = vertex.vec();
                        sel_line.push(SelectionVertex {
                            pos: (point - axis * marker).to_array(),
                            color: point_color,
                        });
                        sel_line.push(SelectionVertex {
                            pos: (point + axis * marker).to_array(),
                            color: point_color,
                        });
                    }
                }
            }
        }
        self.selection_tri_count = sel_tri.len() as u32;
        self.selection_tri_vb = if sel_tri.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-selection-tri"),
                    contents: bytemuck::cast_slice(&sel_tri),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        self.selection_line_count = sel_line.len() as u32;
        self.selection_line_vb = if sel_line.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-selection-line"),
                    contents: bytemuck::cast_slice(&sel_line),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };

        self.mesh_count = mv.len() as u32;
        self.mesh_ranges = mesh_ranges;
        self.sync_asset_textures(device, queue, scene, textured);
        self.mesh_vb = if mv.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-mesh-vb"),
                    contents: bytemuck::cast_slice(&mv),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        self.line_count = lv.len() as u32;
        self.line_vb = if lv.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-edge-vb"),
                    contents: bytemuck::cast_slice(&lv),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };

        // referências: garante texturas e monta quads (matemática em render::scene)
        self.ensure_ref_textures(device, refs);
        let mut rv: Vec<RefVertex> = Vec::new();
        for r in refs.iter().filter(|r| r.visible) {
            let plane = match r.axis {
                RefAxis::Front => petunia_render::scene::RefPlane::Front,
                RefAxis::Back => petunia_render::scene::RefPlane::Back,
                RefAxis::Left => petunia_render::scene::RefPlane::Left,
                RefAxis::Right | RefAxis::Side => petunia_render::scene::RefPlane::Right,
                RefAxis::Top => petunia_render::scene::RefPlane::Top,
                RefAxis::Bottom => petunia_render::scene::RefPlane::Bottom,
            };
            let aspect = r.width as f32 / r.height.max(1) as f32;
            let quad = petunia_render::scene::ref_quad_with_rot(
                plane, r.offset, r.size, aspect, r.rotation,
            );
            for (p, uv) in quad.iter().zip(petunia_render::scene::QUAD_UVS_TOP_LEFT) {
                rv.push(RefVertex { pos: *p, uv });
            }
        }
        // expande quads (4 verts) p/ 2 tris (6 verts)
        let mut tris: Vec<RefVertex> = Vec::new();
        for q in rv.chunks(4) {
            if q.len() == 4 {
                tris.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
            }
        }
        self.ref_count = tris.len() as u32;
        self.ref_vb = if tris.is_empty() {
            None
        } else {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("simple3d-ref-vb"),
                    contents: bytemuck::cast_slice(&tris),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        };
        let _ = queue;
    }

    fn ensure_ref_textures(
        &mut self,
        device: &wgpu::Device,
        refs: &[petunia_core::ReferenceImage],
    ) {
        // Reconstrói slots cujo tamanho/conteúdo mudou; mantém os demais (evita re-upload).
        while self.ref_gpu.len() < refs.len() {
            self.ref_gpu
                .push(self.make_ref_slot(device, &refs[self.ref_gpu.len()]));
        }
        self.ref_gpu.truncate(refs.len());
        for (i, r) in refs.iter().enumerate() {
            let slot = &self.ref_gpu[i];
            if slot.width != r.width || slot.height != r.height || slot.len != r.rgba.len() {
                self.ref_gpu[i] = self.make_ref_slot(device, r);
            }
        }
    }

    fn make_ref_slot(&self, device: &wgpu::Device, r: &petunia_core::ReferenceImage) -> RefGpu {
        let (w, h) = (r.width.max(1), r.height.max(1));
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("simple3d-ref"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simple3d-ref-params-slot"),
            size: std::mem::size_of::<RefUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view = tex.create_view(&Default::default());
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simple3d-ref-bg"),
            layout: &self.ref_tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        RefGpu {
            width: r.width,
            height: r.height,
            len: r.rgba.len(),
            hash: std::cell::Cell::new(0),
            texture: tex,
            view,
            params,
            bind_group: bg,
        }
    }

    /// Garante slots de textura dos assets com canvas (paridade GL):
    /// recria em resize, re-upload só com conteúdo novo (hash), poda saídos.
    fn sync_asset_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Project,
        textured: bool,
    ) {
        use petunia_project::Canvas;
        let wanted: Vec<(uuid::Uuid, u32, u32, Vec<u8>)> = if !textured {
            Vec::new()
        } else {
            scene
                .assets
                .iter()
                .filter(|o| o.visible)
                .filter_map(|o| {
                    let cv: &Canvas = o
                        .texture
                        .as_ref()
                        .or_else(|| o.material(scene).and_then(|m| m.albedo_texture.as_ref()))?;
                    Some((o.id, cv.w, cv.h, cv.pixels.clone()))
                })
                .collect()
        };
        // Poda slots sem canvas/asset correspondente.
        self.asset_tex.retain(|slot| {
            wanted
                .iter()
                .any(|(id, w, h, _)| *id == slot.asset_id && *w == slot.width && *h == slot.height)
        });
        for (id, w, h, pixels) in wanted {
            let hash = fnv1a_hash(&pixels);
            if let Some(slot) = self.asset_tex.iter().find(|s| s.asset_id == id) {
                if slot.hash.get() != hash {
                    slot.hash.set(hash);
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &slot.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &pixels,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * w),
                            rows_per_image: Some(h),
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                }
                continue;
            }
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("simple3d-asset-tex"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("simple3d-asset-tex-bg"),
                layout: &self.mesh_tex_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            self.asset_tex.push(AssetTexGpu {
                asset_id: id,
                width: w,
                height: h,
                hash: std::cell::Cell::new(hash),
                texture,
                view,
                bind_group,
            });
        }
    }

    /// Upload dos pixels + opacidade por ref (chamado todo frame; wgpu ignora se igual via hash cache).
    pub fn upload_ref_pixels(&self, queue: &wgpu::Queue, refs: &[petunia_core::ReferenceImage]) {
        for (i, r) in refs.iter().enumerate() {
            let Some(slot) = self.ref_gpu.get(i) else {
                continue;
            };
            let h = fnv1a_hash(&r.rgba);
            if !r.rgba.is_empty()
                && (r.width, r.height) == (slot.width, slot.height)
                && r.rgba.len() == slot.len
                && slot.hash.get() != h
            {
                slot.hash.set(h);
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &slot.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &r.rgba,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * r.width),
                        rows_per_image: Some(r.height),
                    },
                    wgpu::Extent3d {
                        width: r.width,
                        height: r.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            queue.write_buffer(
                &slot.params,
                0,
                bytemuck::cast_slice(&[RefUniform {
                    opacity: if r.visible { r.opacity } else { 0.0 },
                    _pad: [0.0; 3],
                }]),
            );
        }
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>, refs: &[petunia_core::ReferenceImage]) {
        puffin::profile_function!();
        pass.set_bind_group(0, &self.cam_bind_group, &[]);

        // Grid 3D condicional aos overlays
        if self.show_overlays && self.show_grid {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.grid_vb.slice(..));
            pass.draw(0..self.grid_count, 0..1);
        }

        // Referências padrão (não X-Ray): desenhadas ANTES da geometria sólida com depth test
        if self.show_overlays
            && let Some(vb) = &self.ref_vb
        {
            pass.set_pipeline(&self.ref_pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            let mut start = 0u32;
            for (i, r) in refs.iter().enumerate().filter(|(_, r)| r.visible) {
                if !r.xray
                    && let Some(slot) = self.ref_gpu.get(i)
                {
                    pass.set_bind_group(1, &slot.bind_group, &[]);
                    pass.draw(start..start + 6, 0..1);
                }
                start += 6;
            }
        }

        // malha sólida (ou raio-x com transparência); ranges com canvas usam
        // o pipeline texturizado (paridade com o backend GL).
        if let Some(vb) = &self.mesh_vb {
            pass.set_vertex_buffer(0, vb.slice(..));
            if self.mesh_ranges.is_empty() {
                if self.xray {
                    pass.set_pipeline(&self.mesh_xray_pipeline);
                } else {
                    pass.set_pipeline(&self.mesh_pipeline);
                }
                pass.draw(0..self.mesh_count, 0..1);
            } else {
                for range in &self.mesh_ranges {
                    let tex_bg = range
                        .asset_id
                        .and_then(|id| self.asset_tex.iter().find(|s| s.asset_id == id));
                    match (tex_bg, self.xray) {
                        (Some(slot), false) => {
                            pass.set_pipeline(&self.mesh_tex_pipeline);
                            pass.set_bind_group(1, &slot.bind_group, &[]);
                        }
                        (Some(slot), true) => {
                            pass.set_pipeline(&self.mesh_tex_xray_pipeline);
                            pass.set_bind_group(1, &slot.bind_group, &[]);
                        }
                        (None, true) => {
                            pass.set_pipeline(&self.mesh_xray_pipeline);
                        }
                        (None, false) => {
                            pass.set_pipeline(&self.mesh_pipeline);
                        }
                    }
                    pass.draw(range.start..range.start + range.count, 0..1);
                }
            }
        }
        // Seleção: preenchimento translúcido e contorno/marcadores, ambos com
        // depth test. Fica depois da geometria e antes das arestas para que o
        // wireframe permaneça legível por cima da seleção.
        if let Some(vb) = &self.selection_tri_vb {
            pass.set_pipeline(&self.selection_tri_pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.selection_tri_count, 0..1);
        }
        if let Some(vb) = &self.selection_line_vb {
            pass.set_pipeline(&self.selection_line_pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.selection_line_count, 0..1);
        }

        // arestas
        if let Some(vb) = &self.line_vb {
            if self.xray {
                pass.set_pipeline(&self.line_xray_pipeline);
            } else {
                pass.set_pipeline(&self.line_pipeline);
            }
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.line_count, 0..1);
        }

        // Referências X-Ray: overlay pass desenhado APÓS a geometria com depth test bypass
        if self.show_overlays
            && let Some(vb) = &self.ref_vb
        {
            pass.set_pipeline(&self.ref_xray_pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            let mut start = 0u32;
            for (i, r) in refs.iter().enumerate().filter(|(_, r)| r.visible) {
                if r.xray
                    && let Some(slot) = self.ref_gpu.get(i)
                {
                    pass.set_bind_group(1, &slot.bind_group, &[]);
                    pass.draw(start..start + 6, 0..1);
                }
                start += 6;
            }
        }

        let _ = Vec3::ZERO;
    }

    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_view.as_ref()
    }
}
