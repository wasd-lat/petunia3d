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
                if self.uv_pinned.contains(&(fi, c_idx))
                    && let Some(&existing) = self.faces[fi].uv.get(c_idx)
                {
                    uv.push(existing);
                    continue;
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
                if self.uv_pinned.contains(&(fi, c_idx))
                    && let Some(&existing) = self.faces[fi].uv.get(c_idx)
                {
                    uv.push(existing);
                    continue;
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

    /// Mapeia uma coordenada UV [0.0..1.0] para uma posição e normal no espaço 3D da superfície da malha.
    pub fn uv_to_world(&self, uv: [f32; 2]) -> Option<(Vec3, Vec3)> {
        let u = uv[0];
        let v = uv[1];
        for (fi, face) in self.faces.iter().enumerate() {
            if face.uv.len() < face.verts.len() || face.verts.len() < 3 {
                continue;
            }
            let fnorm = self.face_normal(fi);
            for corners in self.face_triangle_corners(fi) {
                let [ca, cb, cc] = corners;
                let uv0 = face.uv[ca];
                let uv1 = face.uv[cb];
                let uv2 = face.uv[cc];

                let denom =
                    (uv1[1] - uv2[1]) * (uv0[0] - uv2[0]) + (uv2[0] - uv1[0]) * (uv0[1] - uv2[1]);
                if denom.abs() < 1e-7 {
                    continue;
                }

                let w0 =
                    ((uv1[1] - uv2[1]) * (u - uv2[0]) + (uv2[0] - uv1[0]) * (v - uv2[1])) / denom;
                let w1 =
                    ((uv2[1] - uv0[1]) * (u - uv2[0]) + (uv0[0] - uv2[0]) * (v - uv2[1])) / denom;
                let w2 = 1.0 - w0 - w1;

                const EPS: f32 = 1e-4;
                if w0 >= -EPS && w1 >= -EPS && w2 >= -EPS {
                    let va = self.verts[face.verts[ca] as usize].vec();
                    let vb = self.verts[face.verts[cb] as usize].vec();
                    let vc = self.verts[face.verts[cc] as usize].vec();
                    let pos = va * w0 + vb * w1 + vc * w2;
                    return Some((pos, fnorm));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_to_world_on_plane() {
        let mesh = Mesh::plane(2.0);
        // Plane vertices: (-1, -1, 0), (1, -1, 0), (1, 1, 0), (-1, 1, 0)
        // UVs: (0, 0), (1, 0), (1, 1), (0, 1)
        let hit = mesh.uv_to_world([0.5, 0.5]);
        assert!(hit.is_some());
        let (pos, norm) = hit.unwrap();
        assert!((pos.x).abs() < 1e-3);
        assert!((pos.y).abs() < 1e-3);
        assert!((pos.z).abs() < 1e-3);
        assert!(norm.y.abs() > 0.9);
    }
}
