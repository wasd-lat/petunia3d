//! Petunia3D mesh — UVs e normais.

use super::Mesh;
use glam::Vec3;

impl Mesh {
    // ---------------- UV ----------------

    /// Projeção planar por face (eixo dominante da normal), normalizada no bbox.
    pub fn project_planar(&mut self) {
        if self.verts.is_empty() || self.faces.is_empty() {
            return;
        }
        let mut bb_min = Vec3::splat(f32::MAX);
        let mut bb_max = Vec3::splat(f32::MIN);
        for v in &self.verts {
            bb_min = bb_min.min(v.vec());
            bb_max = bb_max.max(v.vec());
        }
        let size = (bb_max - bb_min).max(Vec3::splat(1e-6));
        for fi in 0..self.faces.len() {
            let n = self.face_normal(fi).abs();
            let idx: Vec<u32> = self.faces[fi].verts.clone();
            let mut uv = Vec::new();
            for (c_idx, &vi) in idx.iter().enumerate() {
                if self.uv_pinned.contains(&(fi, c_idx)) {
                    if let Some(&existing) = self.faces[fi].uv.get(c_idx) {
                        uv.push(existing);
                        continue;
                    }
                }
                let p = self.verts[vi as usize].vec();
                let q = (p - bb_min) / size;
                // eixo dominante vira o descartado
                let t = if n.y >= n.x && n.y >= n.z {
                    [q.x, q.z]
                } else if n.x >= n.z {
                    [q.z, q.y]
                } else {
                    [q.x, q.y]
                };
                uv.push(t);
            }
            self.faces[fi].uv = uv;
        }
    }

    /// Projeção cúbica canônica (Box Mapping): projeta cada face no plano
    /// alinhado (+X, -X, +Y, -Y, +Z, -Z) com escala normalizada no bbox.
    pub fn project_cube(&mut self) {
        if self.verts.is_empty() || self.faces.is_empty() {
            return;
        }
        let mut bb_min = Vec3::splat(f32::MAX);
        let mut bb_max = Vec3::splat(f32::MIN);
        for v in &self.verts {
            bb_min = bb_min.min(v.vec());
            bb_max = bb_max.max(v.vec());
        }
        let size = (bb_max - bb_min).max(Vec3::splat(1e-6));

        for fi in 0..self.faces.len() {
            let n = self.face_normal(fi);
            let nx = n.x.abs();
            let ny = n.y.abs();
            let nz = n.z.abs();

            let idx: Vec<u32> = self.faces[fi].verts.clone();
            let mut uv = Vec::with_capacity(idx.len());
            for (c_idx, &vi) in idx.iter().enumerate() {
                if self.uv_pinned.contains(&(fi, c_idx)) {
                    if let Some(&existing) = self.faces[fi].uv.get(c_idx) {
                        uv.push(existing);
                        continue;
                    }
                }
                let p = self.verts[vi as usize].vec();
                let q = (p - bb_min) / size;
                let t = if nx >= ny && nx >= nz {
                    if n.x > 0.0 {
                        [q.z, q.y]
                    } else {
                        [1.0 - q.z, q.y]
                    }
                } else if ny >= nx && ny >= nz {
                    if n.y > 0.0 {
                        [q.x, q.z]
                    } else {
                        [q.x, 1.0 - q.z]
                    }
                } else {
                    if n.z > 0.0 {
                        [q.x, q.y]
                    } else {
                        [1.0 - q.x, q.y]
                    }
                };
                uv.push(t);
            }
            self.faces[fi].uv = uv;
        }
    }

    /// Executa unwrap automático genérico usando o provider xatlas (P1-06 / P3D-064).
    pub fn unwrap_auto(&mut self) -> Result<usize, crate::uv_xatlas::UnwrapError> {
        crate::uv_xatlas::unwrap_fallback(self)
    }

    /// Normais por vértice (média ponderada por área) p/ export.
    pub fn compute_normals(&self) -> Vec<[f32; 3]> {
        let mut acc = vec![Vec3::ZERO; self.verts.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            let n = self.face_normal(fi);
            // área aproximada
            let mut area = 0.0;
            if f.verts.len() >= 3 {
                let a = self.verts[f.verts[0] as usize].vec();
                for k in 1..f.verts.len() - 1 {
                    let b = self.verts[f.verts[k] as usize].vec();
                    let c = self.verts[f.verts[k + 1] as usize].vec();
                    area += (b - a).cross(c - a).length() * 0.5;
                }
            }
            for &vi in &f.verts {
                acc[vi as usize] += n * area.max(1e-9);
            }
        }
        acc.iter()
            .map(|v| v.normalize_or_zero().to_array())
            .collect()
    }
}
