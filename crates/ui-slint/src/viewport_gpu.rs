//! Integração WGPU com o PetuniaViewport e Slint Image.

use std::sync::Arc;

use petunia_core::{Camera, SelectionDomain, Workspace};
use petunia_project::Project;
use petunia_render::Shading;
use petunia_render_wgpu::Renderer;

use crate::PetuniaViewport;

/// Viewport acelerado por WGPU que renderiza a cena 3D para uma textura
/// off-screen e converte em [`slint::Image`].
pub struct WgpuViewport {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub renderer: Renderer,
    pub width: u32,
    pub height: u32,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    target_texture: Option<wgpu::Texture>,
    target_view: Option<wgpu::TextureView>,
}

impl WgpuViewport {
    /// Tenta criar um novo `WgpuViewport` inicializando adaptador e dispositivo padrão.
    pub fn try_create_default(width: u32, height: u32) -> Result<Self, &'static str> {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(instance_desc);

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|_| "Nenhum adaptador WGPU disponível")?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Petunia Slint Viewport Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: Default::default(),
        }))
        .map_err(|_| "Falha ao criar dispositivo WGPU")?;

        Ok(Self::new(Arc::new(device), Arc::new(queue), width, height))
    }

    /// Cria um viewport usando `Device` e `Queue` existentes.
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Self {
        let renderer = Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let mut viewport = Self {
            device,
            queue,
            renderer,
            width: width.max(1),
            height: height.max(1),
            workspace: Workspace::Model,
            selection_domain: SelectionDomain::Object,
            target_texture: None,
            target_view: None,
        };
        viewport.recreate_target();
        viewport
    }

    pub fn recreate_target(&mut self) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Petunia Slint Viewport Target"),
            size: wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.target_texture = Some(texture);
        self.target_view = Some(view);
        self.renderer.resize(&self.device, self.width, self.height);
    }

    /// Renderiza a cena atual para a textura do viewport e exporta como [`slint::Image`].
    pub fn render_frame(
        &mut self,
        project: &Project,
        camera: &Camera,
    ) -> Result<slint::Image, &'static str> {
        if self.target_texture.is_none() {
            self.recreate_target();
        }

        let texture = self.target_texture.as_ref().ok_or("Textura indisponível")?;
        let view = self
            .target_view
            .as_ref()
            .ok_or("TextureView indisponível")?;

        self.renderer.update(
            &self.device,
            &self.queue,
            project,
            &[],
            camera,
            Shading::Solid,
            false,
            false,
            false,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Slint Viewport Encoder"),
            });

        {
            let depth_view = self.renderer.depth_view();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slint Viewport Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.082,
                            g: 0.086,
                            b: 0.098,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: depth_view.map(|dview| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: dview,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.renderer.render(&mut rpass, &[]);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        // Em vez de passar o wgpu::Texture bruto (que causa panic `unimplemented!()` nos renderers
        // FemtoVG OpenGL e Software do Slint), lemos os pixels para um SharedPixelBuffer universal.
        let unpadded_bytes_per_row = self.width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) & !(align - 1);
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Petunia Viewport Staging Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut readback_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Slint Viewport Readback Encoder"),
                });

        readback_encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue
            .submit(std::iter::once(readback_encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|_| "Falha ao sincronizar dispositivo WGPU")?;
        rx.recv()
            .map_err(|_| "Falha ao receber mapeamento de buffer")?
            .map_err(|_| "Erro ao mapear buffer de textura")?;

        let data = buffer_slice
            .get_mapped_range()
            .map_err(|_| "Falha ao obter dados mapeados da textura")?;
        let mut pixel_buffer =
            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(self.width, self.height);
        let pixels = pixel_buffer.make_mut_slice();

        for y in 0..self.height as usize {
            let src_start = y * padded_bytes_per_row as usize;
            let src_end = src_start + unpadded_bytes_per_row as usize;
            let dst_start = y * self.width as usize;
            let row_bytes = &data[src_start..src_end];
            for (x, &[r, g, b, a]) in row_bytes.as_chunks::<4>().0.iter().enumerate() {
                pixels[dst_start + x] = slint::Rgba8Pixel { r, g, b, a };
            }
        }
        drop(data);
        staging_buffer.unmap();

        Ok(slint::Image::from_rgba8(pixel_buffer))
    }
}

impl PetuniaViewport for WgpuViewport {
    fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width.max(1);
            self.height = height.max(1);
            self.recreate_target();
        }
    }

    fn update(&mut self, _dt_seconds: f32) {}

    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
    }

    fn render_frame(&mut self, project: &Project, camera: &Camera) -> Option<slint::Image> {
        WgpuViewport::render_frame(self, project, camera).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgpu_viewport_initializes_or_skips_when_no_gpu() {
        match WgpuViewport::try_create_default(640, 480) {
            Ok(mut viewport) => {
                assert_eq!(viewport.width, 640);
                assert_eq!(viewport.height, 480);
                viewport.resize(800, 600);
                assert_eq!(viewport.width, 800);
                assert_eq!(viewport.height, 600);
                viewport.set_workspace(Workspace::Paint);
                assert_eq!(viewport.workspace, Workspace::Paint);

                let project = Project::new();
                let camera = Camera::default();
                let img = viewport.render_frame(&project, &camera);
                assert!(img.is_ok());
            }
            Err(err) => {
                println!("WgpuViewport ignorado por falta de GPU física: {err}");
            }
        }
    }
}
