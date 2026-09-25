//! Chanfro transacional de aresta convexa com extremos trivalentes.

use std::collections::HashMap;

use crate::{Face, Mesh};

/// Não publica geometria parcial: rejeita topologias sem solução implementada.
#[allow(dead_code)]
pub(crate) fn bevel_edge(mesh: &Mesh, a: u32, b: u32, amount: f32) -> Option<Mesh> {
    bevel_edge_segments_clamped(mesh, a, b, amount, 1, true)
}

/// Chanfro transacional com suporte a multi-segmentos para filetagem arredondada.
#[allow(dead_code)]
pub(crate) fn bevel_edge_segments(
    mesh: &Mesh,
    a: u32,
    b: u32,
    amount: f32,
    segments: u32,
) -> Option<Mesh> {
    bevel_edge_segments_clamped(mesh, a, b, amount, segments, true)
}

/// Chanfro transacional com suporte a multi-segmentos e clamp overlap opcional.
pub(crate) fn bevel_edge_segments_clamped(
    mesh: &Mesh,
    a: u32,
    b: u32,
    amount: f32,
    segments: u32,
    clamp_overlap: bool,
) -> Option<Mesh> {
    if mesh.verts.iter().any(|v| !v.vec().is_finite())
        || mesh.faces.iter().any(|face| {
            face.verts.len() < 3
                || face.uv.len() != face.verts.len()
                || face.verts.iter().any(|&v| v as usize >= mesh.verts.len())
        })
    {
        return None;
    }
    let adjacent = mesh.edge_faces(a, b);
    if adjacent.len() != 2 || a == b {
        return None;
    }
    let input_report = mesh.validate_topology();
    if !input_report.is_manifold {
        return None;
    }
    for endpoint in [a, b] {
        if mesh
            .faces
            .iter()
            .filter(|f| f.verts.contains(&endpoint))
            .count()
            != 3
        {
            return None;
        }
    }
    let origin = mesh.verts.get(a as usize)?.vec();
    for (&face, &other) in adjacent.iter().zip(adjacent.iter().rev()) {
        let normal = mesh.face_normal(face);
        if !normal.is_finite() || normal.length_squared() < 0.5 {
            return None;
        }
        // Uma dobra côncava exige uma solução diferente de interseção de planos.
        if mesh.faces[other].verts.iter().any(|&v| {
            mesh.verts
                .get(v as usize)
                .is_none_or(|p| !p.vec().is_finite() || normal.dot(p.vec() - origin) > 1e-5)
        }) {
            return None;
        }
    }
    if mesh
        .face_normal(adjacent[0])
        .dot(mesh.face_normal(adjacent[1]))
        > 0.9999
    {
        return None;
    }

    let mut result = mesh.clone();
    // Cada offset fica em uma aresta já existente; preserva os planos incidentes.
    let mut offsets: HashMap<(u32, u32), (u32, f32)> = HashMap::new();
    for &fi in &adjacent {
        let face = &mesh.faces[fi];
        for (endpoint, other) in [(a, b), (b, a)] {
            let k = face.verts.iter().position(|&v| v == endpoint)?;
            let prev = face.verts[(k + face.verts.len() - 1) % face.verts.len()];
            let next = face.verts[(k + 1) % face.verts.len()];
            let neighbor = if prev == other {
                next
            } else if next == other {
                prev
            } else {
                return None;
            };
            let mut vertex = mesh.verts.get(endpoint as usize)?.clone();
            let target = mesh.verts.get(neighbor as usize)?;
            let length = vertex.vec().distance(target.vec());
            if !length.is_finite() || length <= 1e-6 {
                return None;
            }
            let max_t = if clamp_overlap { 0.45 } else { 0.95 };
            let t = (amount / length).min(max_t);
            vertex.pos = vertex.vec().lerp(target.vec(), t).to_array();
            for (color, target_color) in vertex.color.iter_mut().zip(target.color) {
                *color += (target_color - *color) * t;
            }
            vertex.selected = true;
            let id = u32::try_from(result.verts.len()).ok()?;
            result.verts.push(vertex);
            offsets.insert((endpoint, neighbor), (id, t));
        }
    }
    if offsets.len() != 4 {
        return None;
    }

    let edge_offsets = |fi: usize| -> Option<(u32, u32)> {
        let face = &mesh.faces[fi];
        let endpoint_offset = |endpoint, other| -> Option<u32> {
            let k = face.verts.iter().position(|&v| v == endpoint)?;
            let prev = face.verts[(k + face.verts.len() - 1) % face.verts.len()];
            let next = face.verts[(k + 1) % face.verts.len()];
            let neighbor = if prev == other { next } else { prev };
            offsets.get(&(endpoint, neighbor)).map(|&(id, _)| id)
        };
        Some((endpoint_offset(a, b)?, endpoint_offset(b, a)?))
    };
    let (a1, b1) = edge_offsets(adjacent[0])?;
    let (a2, b2) = edge_offsets(adjacent[1])?;
    let face = &mesh.faces[adjacent[0]];
    let k = face.verts.iter().position(|&v| v == a)?;
    let winding_reversed = face.verts[(k + 1) % face.verts.len()] == b;

    let segs = segments.clamp(1, 16) as usize;
    let mut rings: Vec<(u32, u32)> = Vec::with_capacity(segs + 1);
    rings.push((a1, b1));

    if segs > 1 {
        let p_a = mesh.verts[a as usize].vec();
        let p_b = mesh.verts[b as usize].vec();
        let va1 = result.verts[a1 as usize].vec();
        let va2 = result.verts[a2 as usize].vec();
        let vb1 = result.verts[b1 as usize].vec();
        let vb2 = result.verts[b2 as usize].vec();
        let ma = (va1 + va2) * 0.5;
        let mb = (vb1 + vb2) * 0.5;

        for s in 1..segs {
            let t = s as f32 / segs as f32;
            let bulge = (1.0 - (2.0 * t - 1.0) * (2.0 * t - 1.0)) * 0.4142;
            let pos_a = va1.lerp(va2, t) + (p_a - ma) * bulge;
            let pos_b = vb1.lerp(vb2, t) + (p_b - mb) * bulge;

            let mut va = result.verts[a1 as usize].clone();
            va.pos = pos_a.to_array();
            va.selected = true;
            let idx_a = result.verts.len() as u32;
            result.verts.push(va);

            let mut vb = result.verts[b1 as usize].clone();
            vb.pos = pos_b.to_array();
            vb.selected = true;
            let idx_b = result.verts.len() as u32;
            result.verts.push(vb);

            rings.push((idx_a, idx_b));
        }
    }
    rings.push((a2, b2));

    for (fi, face) in mesh.faces.iter().enumerate() {
        if face.uv.len() != face.verts.len() {
            return None;
        }
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        for (k, &vertex) in face.verts.iter().enumerate() {
            if vertex != a && vertex != b {
                vertices.push(vertex);
                uvs.push(face.uv[k]);
                continue;
            }
            let prev = (k + face.verts.len() - 1) % face.verts.len();
            let next = (k + 1) % face.verts.len();
            let prev_neighbor = face.verts[prev];
            let next_neighbor = face.verts[next];

            let is_adjacent = adjacent.contains(&fi);
            if is_adjacent {
                for (neighbor_k, neighbor) in [(prev, prev_neighbor), (next, next_neighbor)] {
                    if neighbor == a || neighbor == b {
                        continue;
                    }
                    let &(offset, t) = offsets.get(&(vertex, neighbor))?;
                    vertices.push(offset);
                    uvs.push([
                        face.uv[k][0] + (face.uv[neighbor_k][0] - face.uv[k][0]) * t,
                        face.uv[k][1] + (face.uv[neighbor_k][1] - face.uv[k][1]) * t,
                    ]);
                }
            } else {
                let &(offset_prev, t_prev) = offsets.get(&(vertex, prev_neighbor))?;
                let &(offset_next, t_next) = offsets.get(&(vertex, next_neighbor))?;
                let uv_prev = [
                    face.uv[k][0] + (face.uv[prev][0] - face.uv[k][0]) * t_prev,
                    face.uv[k][1] + (face.uv[prev][1] - face.uv[k][1]) * t_prev,
                ];
                let uv_next = [
                    face.uv[k][0] + (face.uv[next][0] - face.uv[k][0]) * t_next,
                    face.uv[k][1] + (face.uv[next][1] - face.uv[k][1]) * t_next,
                ];

                vertices.push(offset_prev);
                uvs.push(uv_prev);

                if segs > 1 {
                    let is_vertex_a = vertex == a;
                    let forward = if is_vertex_a {
                        offset_prev == a1
                    } else {
                        offset_prev == b1
                    };
                    for s in 1..segs {
                        let step = if forward { s } else { segs - s };
                        let ring_v = if is_vertex_a {
                            rings[step].0
                        } else {
                            rings[step].1
                        };
                        let alpha = s as f32 / segs as f32;
                        let uv_interp = [
                            uv_prev[0] + (uv_next[0] - uv_prev[0]) * alpha,
                            uv_prev[1] + (uv_next[1] - uv_prev[1]) * alpha,
                        ];
                        vertices.push(ring_v);
                        uvs.push(uv_interp);
                    }
                }

                vertices.push(offset_next);
                uvs.push(uv_next);
            }
        }
        result.faces[fi] = Face::with_uv(vertices, uvs);
    }

    for s in 0..segs {
        let (ra0, rb0) = rings[s];
        let (ra1, rb1) = rings[s + 1];
        let indices = if winding_reversed {
            vec![rb0, ra0, ra1, rb1]
        } else {
            vec![ra0, rb0, rb1, ra1]
        };
        let mut ribbon = Face::with_uv(
            indices,
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        );
        ribbon.selected = true;
        result.faces.push(ribbon);
    }

    result.selected_edges.clear();
    result.remove_isolated_vertices();
    result.sync_vert_selection_from_faces();
    let report = result.validate_topology();
    if !report.is_manifold || report.is_closed != input_report.is_closed {
        return None;
    }
    // O offset por arestas nem sempre produz ribbon plano em sólidos irregulares.
    // Rejeita o candidato inteiro antes de publicar uma face torcida/degenerada.
    for (fi, face) in result.faces.iter().enumerate() {
        let normal = result.face_normal(fi);
        let origin = result.verts.get(*face.verts.first()? as usize)?.vec();
        if !normal.is_finite()
            || normal.length_squared() < 0.5
            || face.verts.iter().any(|&v| {
                let offset = result.verts[v as usize].vec() - origin;
                !offset.is_finite() || normal.dot(offset).abs() > 1e-5 * offset.length().max(1.0)
            })
        {
            return None;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cube_edge_bevel_preserves_closed_planar_surface() {
        let cube = Mesh::cube(2.0);
        for edge in cube.edges_unique() {
            let mut mesh = cube.clone();
            mesh.selected_edges.insert(edge);
            assert_eq!(mesh.bevel_selected(0.2), (1, 0));
            let report = mesh.validate_topology();
            assert!(
                report.is_manifold && report.is_closed,
                "{edge:?}: {report:?}"
            );
            assert_eq!((mesh.verts.len(), mesh.faces.len()), (10, 7));
            for (fi, face) in mesh.faces.iter().enumerate() {
                let normal = mesh.face_normal(fi);
                let origin = mesh.verts[face.verts[0] as usize].vec();
                assert!(
                    face.verts
                        .iter()
                        .all(|&vi| normal.dot(mesh.verts[vi as usize].vec() - origin).abs() < 1e-5)
                );
                assert!(normal.dot(mesh.face_centroid(fi)) > 0.0);
                assert_eq!(face.verts.len(), face.uv.len());
            }
        }
    }

    #[test]
    fn unsupported_multiple_edges_and_nonfinite_width_are_atomic() {
        let mut mesh = Mesh::cube(2.0);
        mesh.selected_edges.extend([(0, 1), (1, 2)]);
        let before = format!("{mesh:?}");
        assert_eq!(mesh.bevel_selected(0.2), (0, 2));
        assert_eq!(format!("{mesh:?}"), before);
        mesh.selected_edges.remove(&(1, 2));
        for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            let before = format!("{mesh:?}");
            assert_eq!(mesh.bevel_selected(value), (0, 1));
            assert_eq!(format!("{mesh:?}"), before);
        }
    }

    #[test]
    fn twisted_surface_bevel_is_rejected_without_partial_geometry() {
        let mut mesh = Mesh::cube(2.0);
        mesh.verts[6].pos[0] += 0.7;
        mesh.selected_edges.insert((0, 1));
        let before = format!("{mesh:?}");
        assert_eq!(mesh.bevel_selected(0.2), (0, 1));
        assert_eq!(format!("{mesh:?}"), before);
    }

    #[test]
    fn corrupt_face_indices_and_uvs_are_rejected_before_geometry_access() {
        for invalid_index in [false, true] {
            let mut mesh = Mesh::cube(2.0);
            mesh.selected_edges.insert((0, 1));
            if invalid_index {
                mesh.faces[0].verts[1] = u32::MAX;
            } else {
                mesh.faces[0].uv.clear();
            }
            let before = format!("{mesh:?}");
            assert_eq!(mesh.bevel_selected(0.2), (0, 1));
            assert_eq!(format!("{mesh:?}"), before);
        }
    }
}
