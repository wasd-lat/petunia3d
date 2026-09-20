//! Renderizador 3D em software puro (CPU Rasterizer) para fallback de alta fidelidade.
//!
//! Permite visualização 3D completa (Grid, Sombreado Difuso, Wireframe, Vértices,
//! Projeção Persp/Ortho e Câmera Interativa) em qualquer ambiente onde o WGPU não
//! esteja disponível ou encontre falhas de driver.

use glam::{Mat4, Vec3, Vec4};
use petunia_core::{Camera, SelectionDomain, Workspace};
use petunia_project::Project;

use crate::PetuniaViewport;

/// Viewport 3D rasterizado via CPU com Z-buffer e iluminação direcional.
pub struct Software3dViewport {
    pub width: u32,
    pub height: u32,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    pub depth_buffer: Vec<f32>,
    pub color_buffer: Vec<u8>,
}

impl Software3dViewport {
    pub fn new(width: u32, height: u32) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let count = (w * h) as usize;
        Self {
            width: w,
            height: h,
            workspace: Workspace::Model,
            selection_domain: SelectionDomain::Object,
            depth_buffer: vec![1.0; count],
            color_buffer: vec![0; count * 4],
        }
    }

    fn clear(&mut self, bg: [u8; 4]) {
        self.depth_buffer.fill(1.0);
        let count = (self.width * self.height) as usize;
        if self.color_buffer.len() != count * 4 {
            self.color_buffer.resize(count * 4, 0);
        }
        for chunk in self.color_buffer.as_chunks_mut::<4>().0 {
            chunk.copy_from_slice(&bg);
        }
    }

    #[inline]
    fn project_point(&self, p: Vec3, vp: &Mat4) -> Option<(f32, f32, f32)> {
        let clip = *vp * Vec4::new(p.x, p.y, p.z, 1.0);
        if clip.w <= 0.05 {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        let ndc_x = clip.x * inv_w;
        let ndc_y = clip.y * inv_w;
        let ndc_z = clip.z * inv_w;

        let screen_x = (ndc_x + 1.0) * 0.5 * (self.width as f32);
        let screen_y = (1.0 - ndc_y) * 0.5 * (self.height as f32);
        Some((screen_x, screen_y, ndc_z))
    }

    fn draw_line_2d(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
        let mut x0 = x0;
        let mut y0 = y0;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let w = self.width as i32;
        let h = self.height as i32;

        loop {
            if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
                let idx = ((y0 as u32 * self.width + x0 as u32) * 4) as usize;
                if idx + 4 <= self.color_buffer.len() {
                    self.color_buffer[idx..idx + 4].copy_from_slice(&color);
                }
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn draw_triangle_3d(
        &mut self,
        p0: (f32, f32, f32),
        p1: (f32, f32, f32),
        p2: (f32, f32, f32),
        color: [u8; 4],
    ) {
        let (x0, y0, z0) = p0;
        let (x1, y1, z1) = p1;
        let (x2, y2, z2) = p2;

        // Área 2D orientada
        let area = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
        if area.abs() < 1e-4 {
            return;
        }

        let min_x = (x0.min(x1).min(x2).floor() as i32).clamp(0, self.width as i32 - 1);
        let max_x = (x0.max(x1).max(x2).ceil() as i32).clamp(0, self.width as i32 - 1);
        let min_y = (y0.min(y1).min(y2).floor() as i32).clamp(0, self.height as i32 - 1);
        let max_y = (y0.max(y1).max(y2).ceil() as i32).clamp(0, self.height as i32 - 1);

        let inv_area = 1.0 / area;

        for y in min_y..=max_y {
            let fy = y as f32 + 0.5;
            for x in min_x..=max_x {
                let fx = x as f32 + 0.5;

                let w0 = ((x1 - fx) * (y2 - fy) - (x2 - fx) * (y1 - fy)) * inv_area;
                let w1 = ((x2 - fx) * (y0 - fy) - (x0 - fx) * (y2 - fy)) * inv_area;
                let w2 = 1.0 - w0 - w1;

                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let z = w0 * z0 + w1 * z1 + w2 * z2;
                    let pixel_idx = (y as u32 * self.width + x as u32) as usize;

                    if pixel_idx < self.depth_buffer.len() && z < self.depth_buffer[pixel_idx] {
                        self.depth_buffer[pixel_idx] = z;
                        let color_idx = pixel_idx * 4;
                        if color_idx + 4 <= self.color_buffer.len() {
                            self.color_buffer[color_idx..color_idx + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
    }

    fn draw_grid(&mut self, vp: &Mat4) {
        let grid_size = 6.0;
        let step = 1.0;
        let mut coord: f32 = -grid_size;
        let grid_color = [45, 48, 55, 255];
        let axis_x_color = [220, 60, 60, 255];
        let axis_z_color = [60, 100, 230, 255];

        while coord <= grid_size + 1e-4 {
            let is_x_axis = coord.abs() < 1e-4;
            let line_color = if is_x_axis { axis_x_color } else { grid_color };
            if let (Some(a), Some(b)) = (
                self.project_point(Vec3::new(-grid_size, 0.0, coord), vp),
                self.project_point(Vec3::new(grid_size, 0.0, coord), vp),
            ) {
                self.draw_line_2d(a.0 as i32, a.1 as i32, b.0 as i32, b.1 as i32, line_color);
            }

            let is_z_axis = coord.abs() < 1e-4;
            let line_color = if is_z_axis { axis_z_color } else { grid_color };
            if let (Some(a), Some(b)) = (
                self.project_point(Vec3::new(coord, 0.0, -grid_size), vp),
                self.project_point(Vec3::new(coord, 0.0, grid_size), vp),
            ) {
                self.draw_line_2d(a.0 as i32, a.1 as i32, b.0 as i32, b.1 as i32, line_color);
            }

            coord += step;
        }
    }
}

impl PetuniaViewport for Software3dViewport {
    fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        if self.width != w || self.height != h {
            self.width = w;
            self.height = h;
            let count = (w * h) as usize;
            self.depth_buffer.resize(count, 1.0);
            self.color_buffer.resize(count * 4, 0);
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
        let bg_color = [24, 25, 28, 255];
        self.clear(bg_color);

        let vp = camera.view_proj();
        self.draw_grid(&vp);

        let light_dir = Vec3::new(0.4, 0.9, 0.6).normalize();

        for (asset_idx, asset) in project.assets.iter().enumerate() {
            if !asset.visible {
                continue;
            }
            let is_active = project.active == asset_idx;
            let base_color = if is_active {
                [180, 185, 195]
            } else {
                [140, 145, 155]
            };

            // Rasterização de faces
            for face in &asset.mesh.faces {
                if face.verts.len() < 3 {
                    continue;
                }

                // Cálculo de normal da face
                let p0 = asset.mesh.verts[face.verts[0] as usize].vec();
                let p1 = asset.mesh.verts[face.verts[1] as usize].vec();
                let p2 = asset.mesh.verts[face.verts[2] as usize].vec();
                let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();

                let diff = normal.dot(light_dir).max(0.0);
                let light = 0.35 + 0.65 * diff;

                let shaded_color = [
                    (base_color[0] as f32 * light).min(255.0) as u8,
                    (base_color[1] as f32 * light).min(255.0) as u8,
                    (base_color[2] as f32 * light).min(255.0) as u8,
                    255,
                ];

                // Triangulação em leque
                for i in 1..(face.verts.len() - 1) {
                    let v0 = asset.mesh.verts[face.verts[0] as usize].vec();
                    let v1 = asset.mesh.verts[face.verts[i] as usize].vec();
                    let v2 = asset.mesh.verts[face.verts[i + 1] as usize].vec();

                    if let (Some(sp0), Some(sp1), Some(sp2)) = (
                        self.project_point(v0, &vp),
                        self.project_point(v1, &vp),
                        self.project_point(v2, &vp),
                    ) {
                        self.draw_triangle_3d(sp0, sp1, sp2, shaded_color);
                    }
                }
            }

            // Wireframe das arestas do ativo
            let edge_color = if is_active {
                [40, 42, 48, 255]
            } else {
                [30, 32, 36, 255]
            };

            for face in &asset.mesh.faces {
                let flen = face.verts.len();
                for i in 0..flen {
                    let idx_a = face.verts[i] as usize;
                    let idx_b = face.verts[(i + 1) % flen] as usize;
                    let v0 = asset.mesh.verts[idx_a].vec();
                    let v1 = asset.mesh.verts[idx_b].vec();

                    if let (Some(sp0), Some(sp1)) =
                        (self.project_point(v0, &vp), self.project_point(v1, &vp))
                    {
                        self.draw_line_2d(
                            sp0.0 as i32,
                            sp0.1 as i32,
                            sp1.0 as i32,
                            sp1.1 as i32,
                            edge_color,
                        );
                    }
                }
            }

            // Vértices selecionados em modo Point
            if self.selection_domain == SelectionDomain::Vertex && is_active {
                for vert in &asset.mesh.verts {
                    if !vert.selected {
                        continue;
                    }
                    let Some(sp) = self.project_point(vert.vec(), &vp) else {
                        continue;
                    };
                    let cx = sp.0 as i32;
                    let cy = sp.1 as i32;
                    let yellow = [255, 220, 30, 255];
                    for dy in -2..=2 {
                        for dx in -2..=2 {
                            let px = cx + dx;
                            let py = cy + dy;
                            if px >= 0
                                && px < self.width as i32
                                && py >= 0
                                && py < self.height as i32
                            {
                                let idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                                if idx + 4 <= self.color_buffer.len() {
                                    self.color_buffer[idx..idx + 4].copy_from_slice(&yellow);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut pixel_buffer =
            slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(self.width, self.height);
        let dest = pixel_buffer.make_mut_slice();
        for (i, pixel) in self.color_buffer.as_chunks::<4>().0.iter().enumerate() {
            if i < dest.len() {
                dest[i] = slint::Rgba8Pixel {
                    r: pixel[0],
                    g: pixel[1],
                    b: pixel[2],
                    a: pixel[3],
                };
            }
        }

        Some(slint::Image::from_rgba8(pixel_buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_viewport_initializes_and_renders_frame() {
        let mut viewport = Software3dViewport::new(320, 240);
        let project = Project::default();
        let camera = Camera::default();

        let image = viewport.render_frame(&project, &camera);
        assert!(image.is_some());
        let img = image.expect("image");
        assert_eq!(img.size().width, 320);
        assert_eq!(img.size().height, 240);
    }

    #[test]
    fn software_viewport_resizes_correctly() {
        let mut viewport = Software3dViewport::new(100, 100);
        viewport.resize(200, 150);
        assert_eq!(viewport.width, 200);
        assert_eq!(viewport.height, 150);

        let project = Project::default();
        let camera = Camera::default();
        let image = viewport.render_frame(&project, &camera).expect("frame");
        assert_eq!(image.size().width, 200);
        assert_eq!(image.size().height, 150);
    }

    #[test]
    fn software_viewport_renders_with_selection_domains() {
        let mut viewport = Software3dViewport::new(160, 120);
        let mut project = Project::default();
        if let Some(v) = project.active_mesh_mut().and_then(|m| m.verts.first_mut()) {
            v.selected = true;
        }
        let camera = Camera::default();

        viewport.set_selection_domain(SelectionDomain::Vertex);
        let frame_v = viewport.render_frame(&project, &camera);
        assert!(frame_v.is_some());

        viewport.set_selection_domain(SelectionDomain::Edge);
        let frame_e = viewport.render_frame(&project, &camera);
        assert!(frame_e.is_some());

        viewport.set_selection_domain(SelectionDomain::Face);
        let frame_f = viewport.render_frame(&project, &camera);
        assert!(frame_f.is_some());
    }
}
