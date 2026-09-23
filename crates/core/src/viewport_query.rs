//! Shared viewport query contract for picking, hover, screen-space paint and decals.
//! CPU fallback; GPU query buffers can fill the same DTO later.

use glam::Vec3;
use uuid::Uuid;

/// Per-sample hit from a viewport query (pixel or ray).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportQuerySample {
    pub object_id: Uuid,
    pub face_index: usize,
    pub depth: f32,
    pub uv: [f32; 2],
    pub world: Vec3,
    pub normal: Vec3,
}

/// Batch of samples covering a dab / pick / hover.
#[derive(Clone, Debug, Default)]
pub struct ViewportQueryBuffer {
    pub samples: Vec<ViewportQuerySample>,
}

impl ViewportQueryBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, sample: ViewportQuerySample) {
        self.samples.push(sample);
    }

    pub fn nearest(&self) -> Option<&ViewportQuerySample> {
        self.samples.iter().min_by(|a, b| a.depth.total_cmp(&b.depth))
    }
}

/// Surface attachment for persistent decals. Topology edits must revalidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentValidity {
    Valid,
    NeedsReprojection,
    Invalid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAttachment {
    pub object_id: Uuid,
    pub face_index: usize,
    pub barycentric: [f32; 3],
    pub topology_revision: u64,
}

impl SurfaceAttachment {
    pub fn validate(&self, object_id: Uuid, face_count: usize, topology_revision: u64) -> AttachmentValidity {
        if self.object_id != object_id {
            return AttachmentValidity::Invalid;
        }
        if self.face_index >= face_count {
            return AttachmentValidity::Invalid;
        }
        if self.topology_revision != topology_revision {
            AttachmentValidity::NeedsReprojection
        } else {
            AttachmentValidity::Valid
        }
    }
}

/// Immutable scene geometry for a batch of visibility queries. Evaluated
/// meshes and triangulation are created once per batch, never per candidate.
pub struct ViewportSceneQuery {
    surfaces: Vec<QuerySurface>,
}

struct QuerySurface {
    asset_index: usize,
    locked: bool,
    min: Vec3,
    max: Vec3,
    triangles: Vec<[Vec3; 3]>,
}

impl ViewportSceneQuery {
    pub fn new(project: &petunia_project::Project) -> Self {
        let surfaces = project.assets.iter().enumerate().filter(|(_, asset)| asset.visible)
            .filter_map(|(asset_index, asset)| {
                let mesh = asset.evaluated_mesh();
                let triangles: Vec<_> = mesh.faces.iter().enumerate().flat_map(|(fi, face)| {
                    mesh.face_triangle_corners(fi).into_iter().map(|corners|
                        corners.map(|corner| mesh.verts[face.verts[corner] as usize].vec()))
                }).collect();
                if triangles.is_empty() { return None; }
                let min = triangles.iter().flatten().copied().fold(Vec3::splat(f32::INFINITY), Vec3::min);
                let max = triangles.iter().flatten().copied().fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
                Some(QuerySurface { asset_index, locked: asset.locked, min, max, triangles })
            }).collect();
        Self { surfaces }
    }

    fn ray_hits_bounds(surface: &QuerySurface, origin: Vec3, direction: Vec3, limit: f32) -> bool {
        let (mut near, mut far) = (0.0_f32, limit);
        for axis in 0..3 {
            let lo = surface.min[axis] - 1.0e-5;
            let hi = surface.max[axis] + 1.0e-5;
            if direction[axis].abs() < 1.0e-10 {
                if origin[axis] < lo || origin[axis] > hi { return false; }
            } else {
                let a = (lo - origin[axis]) / direction[axis];
                let b = (hi - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if near > far { return false; }
            }
        }
        true
    }

    /// Locked objects still occlude, even though they cannot be picked.
    pub fn nearest_object(&self, camera: &crate::Camera, ndc: [f32; 2]) -> Option<usize> {
        let (origin, direction) = camera.ray(ndc[0], ndc[1]);
        let matrix = camera.view_proj();
        let mut nearest: Option<(usize, f32, bool)> = None;
        for surface in &self.surfaces {
            if !Self::ray_hits_bounds(surface, origin, direction, nearest.map_or(f32::INFINITY, |(_, d, _)| d)) { continue; }
            for &[a, b, c] in &surface.triangles {
                let Some(depth) = petunia_mesh::triangulate::ray_tri(origin, direction, a, b, c) else { continue; };
                let clip = matrix * (origin + direction * depth).extend(1.0);
                if clip.w > 0.0 && clip.z >= 0.0 && clip.z <= clip.w && nearest.is_none_or(|(_, d, _)| depth < d) {
                    nearest = Some((surface.asset_index, depth, surface.locked));
                }
            }
        }
        nearest.filter(|(_, _, locked)| !locked).map(|(index, _, _)| index)
    }

    pub fn point_visible(&self, camera: &crate::Camera, point: Vec3) -> bool {
        let matrix = camera.view_proj();
        let clip = matrix * point.extend(1.0);
        if !clip.is_finite() || clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w { return false; }
        let (origin, direction) = camera.ray(clip.x / clip.w, clip.y / clip.w);
        let target = (point - origin).dot(direction);
        let limit = target - 1.0e-5 * target.abs().max(1.0);
        for surface in &self.surfaces {
            if !Self::ray_hits_bounds(surface, origin, direction, limit) { continue; }
            for &[a, b, c] in &surface.triangles {
                if let Some(depth) = petunia_mesh::triangulate::ray_tri(origin, direction, a, b, c)
                    && depth < limit
                {
                    // Geometry outside the camera depth range cannot occlude.
                    let hit = matrix * (origin + direction * depth).extend(1.0);
                    if hit.w > 0.0 && hit.z >= 0.0 { return false; }
                }
            }
        }
        true
    }
}

/// Convenience query for callers with a single candidate. Use one scene query
/// for a batch (box selection or screen-space component picking).
pub fn point_visible(project: &petunia_project::Project, camera: &crate::Camera, point: Vec3) -> bool {
    ViewportSceneQuery::new(project).point_visible(camera, point)
}

impl crate::AppState {
    /// Rectangle in NDC coordinates. Selection changes are non-destructive and
    /// preserve the active domain; depth is bypassed only by X-Ray/Wireframe.
    pub fn select_viewport_box(&mut self, a: [f32; 2], b: [f32; 2], add: bool, subtract: bool) {
        if a.iter().chain(&b).any(|v| !v.is_finite()) || self.modal.is_some() || self.mesh_preview.is_some() || self.paint_stroke.is_some() { return; }
        let vp = self.session.camera.view_proj();
        let scene = ViewportSceneQuery::new(&self.project.project);
        let inside = |point: Vec3| {
            let clip = vp * point.extend(1.0);
            if clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w { return false; }
            let p = clip.truncate() / clip.w;
            p.x >= a[0].min(b[0]) && p.x <= a[0].max(b[0]) && p.y >= a[1].min(b[1]) && p.y <= a[1].max(b[1])
                && (self.session.show_xray || self.session.shading == crate::Shading::Wireframe
                    || scene.point_visible(&self.session.camera, point))
        };
        if self.selection_domain() == crate::SelectionDomain::Object {
            let hits: Vec<_> = self.project.assets.iter().filter(|asset| asset.visible && !asset.locked)
                .filter(|asset| asset.evaluated_mesh().verts.iter().any(|v| inside(v.vec())))
                .map(|asset| asset.id).collect();
            if !add && !subtract { self.session.selection.assets.clear(); }
            for id in hits {
                if subtract { self.session.selection.assets.retain(|selected| *selected != id); }
                else if !self.session.selection.assets.contains(&id) { self.session.selection.assets.push(id); }
            }
            self.project.active = self.session.selection.assets.last().and_then(|id| self.project.assets.iter().position(|asset| asset.id == *id)).unwrap_or(usize::MAX);
        } else {
            let Some(asset) = self.project.active().filter(|asset| asset.visible && !asset.locked) else { return; };
            let mesh = &asset.mesh;
            let domain = self.selection_domain();
            let vertices: Vec<_> = if domain == crate::SelectionDomain::Vertex { mesh.verts.iter().enumerate().filter(|(_, v)| inside(v.vec())).map(|(i, _)| i).collect() } else { Vec::new() };
            let edges: Vec<_> = if domain == crate::SelectionDomain::Edge { mesh.edges_unique().into_iter().filter(|&(a, b)| inside((mesh.verts[a as usize].vec() + mesh.verts[b as usize].vec()) * 0.5)).collect() } else { Vec::new() };
            let faces: Vec<_> = if domain == crate::SelectionDomain::Face { mesh.faces.iter().enumerate().filter(|(_, face)| !face.verts.is_empty() && inside(face.verts.iter().map(|&v| mesh.verts[v as usize].vec()).sum::<Vec3>() / face.verts.len() as f32)).map(|(i, _)| i).collect() } else { Vec::new() };
            let Some(mesh) = self.project.active_mesh_mut() else { return; };
            if !add && !subtract { mesh.deselect_all(); }
            for i in vertices { mesh.verts[i].selected = !subtract; }
            for edge in edges { if subtract { mesh.selected_edges.remove(&edge); } else { mesh.selected_edges.insert(edge); } }
            for i in faces { mesh.faces[i].selected = !subtract; }
            if domain == crate::SelectionDomain::Face { mesh.sync_vert_selection_from_faces(); }
            if domain == crate::SelectionDomain::Edge {
                for v in &mut mesh.verts { v.selected = false; }
                for &(a, b) in &mesh.selected_edges { mesh.verts[a as usize].selected = true; mesh.verts[b as usize].selected = true; }
            }
        }
        self.session.tools.hover = crate::HoverTarget::None;
        self.sync_selection();
        self.mark_dirty();
    }
}
