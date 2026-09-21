//! UV islands, packing, texel density and diagnostics (UV0 only).

use std::collections::{HashMap, HashSet, VecDeque};

use super::{Mesh, edge_key};

/// One connected UV chart.
#[derive(Clone, Debug)]
pub struct UvIsland {
    pub faces: Vec<usize>,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl UvIsland {
    pub fn area(&self) -> f32 {
        let w = (self.max[0] - self.min[0]).max(0.0);
        let h = (self.max[1] - self.min[1]).max(0.0);
        w * h
    }

    pub fn center(&self) -> [f32; 2] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
        ]
    }
}

/// Stretch / overlap / invalid UV report.
#[derive(Clone, Debug, Default)]
pub struct UvDiagnostics {
    pub island_count: usize,
    pub overlapping_islands: usize,
    pub zero_area_faces: usize,
    pub out_of_range_corners: usize,
    pub mean_stretch: f32,
    pub max_stretch: f32,
}

impl Mesh {
    pub fn mark_seam(&mut self, a: u32, b: u32) {
        self.uv_seams.insert(edge_key(a, b));
    }

    pub fn clear_seam(&mut self, a: u32, b: u32) {
        self.uv_seams.remove(&edge_key(a, b));
    }

    pub fn toggle_seam(&mut self, a: u32, b: u32) {
        let k = edge_key(a, b);
        if !self.uv_seams.remove(&k) {
            self.uv_seams.insert(k);
        }
    }

    /// Faces sharing a non-seam UV edge form an island.
    pub fn uv_islands(&self) -> Vec<UvIsland> {
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            for k in 0..n {
                let e = edge_key(face.verts[k], face.verts[(k + 1) % n]);
                edge_faces.entry(e).or_default().push(fi);
            }
        }
        for (edge, faces) in &edge_faces {
            if self.uv_seams.contains(edge) {
                continue;
            }
            if faces.len() == 2 {
                adj.entry(faces[0]).or_default().push(faces[1]);
                adj.entry(faces[1]).or_default().push(faces[0]);
            }
        }
        let mut seen = vec![false; self.faces.len()];
        let mut islands = Vec::new();
        for start in 0..self.faces.len() {
            if seen[start] {
                continue;
            }
            let mut q = VecDeque::new();
            let mut faces = Vec::new();
            seen[start] = true;
            q.push_back(start);
            while let Some(fi) = q.pop_front() {
                faces.push(fi);
                if let Some(nbs) = adj.get(&fi) {
                    for &nb in nbs {
                        if !seen[nb] {
                            seen[nb] = true;
                            q.push_back(nb);
                        }
                    }
                }
            }
            let mut min = [f32::MAX, f32::MAX];
            let mut max = [f32::MIN, f32::MIN];
            for &fi in &faces {
                for uv in &self.faces[fi].uv {
                    min[0] = min[0].min(uv[0]);
                    min[1] = min[1].min(uv[1]);
                    max[0] = max[0].max(uv[0]);
                    max[1] = max[1].max(uv[1]);
                }
            }
            if min[0] > max[0] {
                min = [0.0, 0.0];
                max = [0.0, 0.0];
            }
            islands.push(UvIsland { faces, min, max });
        }
        islands
    }

    /// Pack islands into 0..1 with padding, no overlaps.
    pub fn pack_uv_islands(&mut self, padding: f32) -> usize {
        let padding = padding.clamp(0.0, 0.25);
        let mut islands = self.uv_islands();
        if islands.is_empty() {
            return 0;
        }
        islands.sort_by(|a, b| b.area().partial_cmp(&a.area()).unwrap_or(std::cmp::Ordering::Equal));
        let n = islands.len();
        let cols = (n as f32).sqrt().ceil().max(1.0) as usize;
        let cell = (1.0 / cols as f32).max(1e-6);
        let usable = (cell - padding * 2.0).max(1e-6);
        for (i, island) in islands.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let ox = col as f32 * cell + padding;
            let oy = row as f32 * cell + padding;
            let w = (island.max[0] - island.min[0]).max(1e-6);
            let h = (island.max[1] - island.min[1]).max(1e-6);
            let s = (usable / w).min(usable / h);
            for &fi in &island.faces {
                for uv in &mut self.faces[fi].uv {
                    uv[0] = ox + (uv[0] - island.min[0]) * s;
                    uv[1] = oy + (uv[1] - island.min[1]) * s;
                }
            }
        }
        n
    }

    /// Average texels-per-unit for selected (or all) faces.
    pub fn texel_density(&self, texture_w: u32, selected_only: bool) -> f32 {
        let mut acc = 0.0;
        let mut n = 0.0;
        for face in &self.faces {
            if selected_only && !face.selected {
                continue;
            }
            if face.verts.len() < 3 || face.uv.len() < 3 {
                continue;
            }
            let a = self.verts[face.verts[0] as usize].vec();
            let b = self.verts[face.verts[1] as usize].vec();
            let c = self.verts[face.verts[2] as usize].vec();
            let world = (b - a).cross(c - a).length() * 0.5;
            let ua = glam::Vec2::from_array(face.uv[0]);
            let ub = glam::Vec2::from_array(face.uv[1]);
            let uc = glam::Vec2::from_array(face.uv[2]);
            let uv_area = (ub - ua).perp_dot(uc - ua).abs() * 0.5;
            if world > 1e-8 {
                acc += (uv_area * texture_w as f32) / world;
                n += 1.0;
            }
        }
        if n <= 0.0 { 0.0 } else { acc / n }
    }

    pub fn normalize_texel_density(&mut self, texture_w: u32, target: f32) {
        let current = self.texel_density(texture_w, false).max(1e-6);
        let s = (target / current).clamp(0.01, 100.0);
        for face in &mut self.faces {
            for uv in &mut face.uv {
                uv[0] = 0.5 + (uv[0] - 0.5) * s;
                uv[1] = 0.5 + (uv[1] - 0.5) * s;
            }
        }
    }

    pub fn project_from_view(&mut self, view_x: glam::Vec3, view_y: glam::Vec3, view_origin: glam::Vec3) {
        let vx = view_x.normalize_or_zero();
        let vy = view_y.normalize_or_zero();
        if vx.length_squared() < 1e-8 || vy.length_squared() < 1e-8 {
            self.project_planar();
            return;
        }
        let mut us = Vec::new();
        let mut vs = Vec::new();
        for v in &self.verts {
            let p = v.vec() - view_origin;
            us.push(p.dot(vx));
            vs.push(p.dot(vy));
        }
        let (u_min, u_max) = minmax(&us);
        let (v_min, v_max) = minmax(&vs);
        let du = (u_max - u_min).max(1e-6);
        let dv = (v_max - v_min).max(1e-6);
        for face in &mut self.faces {
            face.uv = face
                .verts
                .iter()
                .map(|&vi| {
                    [
                        (us[vi as usize] - u_min) / du,
                        (vs[vi as usize] - v_min) / dv,
                    ]
                })
                .collect();
        }
    }

    pub fn uv_diagnostics(&self) -> UvDiagnostics {
        let islands = self.uv_islands();
        let mut overlapping = 0;
        for i in 0..islands.len() {
            for j in (i + 1)..islands.len() {
                if aabb_overlap(islands[i].min, islands[i].max, islands[j].min, islands[j].max) {
                    overlapping += 1;
                }
            }
        }
        let mut zero_area = 0;
        let mut oor = 0;
        let mut stretch_sum = 0.0;
        let mut stretch_max = 0.0;
        let mut stretch_n = 0.0;
        for face in &self.faces {
            if face.uv.len() < 3 || face.verts.len() < 3 {
                zero_area += 1;
                continue;
            }
            for uv in &face.uv {
                if !(0.0..=1.0).contains(&uv[0]) || !(0.0..=1.0).contains(&uv[1]) {
                    oor += 1;
                }
            }
            let a = self.verts[face.verts[0] as usize].vec();
            let b = self.verts[face.verts[1] as usize].vec();
            let c = self.verts[face.verts[2] as usize].vec();
            let world = (b - a).length().max(1e-8);
            let uv_len = (glam::Vec2::from_array(face.uv[1]) - glam::Vec2::from_array(face.uv[0]))
                .length()
                .max(1e-8);
            let s = (uv_len / world - 1.0).abs();
            stretch_sum += s;
            stretch_max = stretch_max.max(s);
            stretch_n += 1.0;
            let ua = glam::Vec2::from_array(face.uv[0]);
            let ub = glam::Vec2::from_array(face.uv[1]);
            let uc = glam::Vec2::from_array(face.uv[2]);
            if (ub - ua).perp_dot(uc - ua).abs() < 1e-10 {
                zero_area += 1;
            }
        }
        UvDiagnostics {
            island_count: islands.len(),
            overlapping_islands: overlapping,
            zero_area_faces: zero_area,
            out_of_range_corners: oor,
            mean_stretch: if stretch_n == 0.0 {
                0.0
            } else {
                stretch_sum / stretch_n
            },
            max_stretch: stretch_max,
        }
    }
}

fn minmax(v: &[f32]) -> (f32, f32) {
    let mut mn = f32::MAX;
    let mut mx = f32::MIN;
    for &x in v {
        mn = mn.min(x);
        mx = mx.max(x);
    }
    if mn > mx { (0.0, 1.0) } else { (mn, mx) }
}

fn aabb_overlap(amin: [f32; 2], amax: [f32; 2], bmin: [f32; 2], bmax: [f32; 2]) -> bool {
    amin[0] < bmax[0] && amax[0] > bmin[0] && amin[1] < bmax[1] && amax[1] > bmin[1]
}
