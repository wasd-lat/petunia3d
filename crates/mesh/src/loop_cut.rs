//! Quad-ring discovery and shared-vertex loop cuts. All edits are transactional:
//! failure returns an error and never mutates the supplied mesh.

#![forbid(unsafe_code)]

use crate::{Face, Mesh, Vertex};
use glam::{Vec2, Vec3};
use std::collections::{HashMap, HashSet};

type Edge = (u32, u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopCutError {
    InvalidEdge,
    InvalidMesh,
    NonQuad,
    NonManifold,
    TwistedRing,
    InvalidParameters,
    StaleRing,
}

impl std::fmt::Display for LoopCutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidEdge => "Choose an edge belonging to a quad",
            Self::InvalidMesh => "Loop cut requires valid finite geometry and UVs",
            Self::NonQuad => "Loop cut stopped: the ring contains a triangle or ngon",
            Self::NonManifold => "Loop cut requires manifold edges",
            Self::TwistedRing => "Loop cut cannot cross itself or a twisted ring",
            Self::InvalidParameters => "Use 1–32 cuts and a finite slide between -1 and 1",
            Self::StaleRing => "Mesh topology changed; choose the loop again",
        })
    }
}
impl std::error::Error for LoopCutError {}

#[derive(Debug, Clone)]
struct RingFace {
    index: usize,
    original: [u32; 4],
    /// Cyclic order with entry edge at corners 0 and 1.
    corners: [u32; 4],
    offset: usize,
}

#[derive(Debug, Clone)]
pub struct LoopRing {
    faces: Vec<RingFace>,
    /// Each edge is directed consistently across the ring for shared slide values.
    edges: Vec<Edge>,
    closed: bool,
}

fn key((a, b): Edge) -> Edge {
    (a.min(b), a.max(b))
}

impl LoopRing {
    pub fn discover(mesh: &Mesh, seed: Edge) -> Result<Self, LoopCutError> {
        if seed.0 == seed.1
            || seed.0 as usize >= mesh.verts.len()
            || seed.1 as usize >= mesh.verts.len()
        {
            return Err(LoopCutError::InvalidEdge);
        }
        let mut adjacency: HashMap<Edge, Vec<usize>> = HashMap::new();
        for (index, face) in mesh.faces.iter().enumerate() {
            if face.verts.len() < 3
                || face.uv.len() != face.verts.len()
                || face.verts.iter().any(|&v| v as usize >= mesh.verts.len())
            {
                return Err(LoopCutError::InvalidMesh);
            }
            for i in 0..face.verts.len() {
                adjacency
                    .entry(key((face.verts[i], face.verts[(i + 1) % face.verts.len()])))
                    .or_default()
                    .push(index);
            }
        }
        if !adjacency.contains_key(&key(seed)) {
            return Err(LoopCutError::InvalidEdge);
        }
        let mut directions = HashMap::from([(key(seed), seed)]);
        let mut pending = vec![seed];
        let mut visited_faces = HashMap::new();
        let mut faces = Vec::new();
        let mut edges = Vec::new();
        let mut closed = true;
        while let Some(entry) = pending.pop() {
            edges.push(entry);
            let adjacent = adjacency
                .get(&key(entry))
                .ok_or(LoopCutError::InvalidEdge)?;
            if adjacent.len() > 2 {
                return Err(LoopCutError::NonManifold);
            }
            closed &= adjacent.len() == 2;
            for &face_index in adjacent {
                let face = &mesh.faces[face_index];
                if face.verts.len() != 4 {
                    return Err(LoopCutError::NonQuad);
                }
                let original = [face.verts[0], face.verts[1], face.verts[2], face.verts[3]];
                if original.iter().copied().collect::<HashSet<_>>().len() != 4
                    || original
                        .iter()
                        .any(|&i| !mesh.verts[i as usize].vec().is_finite())
                    || face.uv.iter().flatten().any(|v| !v.is_finite())
                {
                    return Err(LoopCutError::InvalidMesh);
                }
                if let Some(previous) = visited_faces.get(&face_index) {
                    let previous: &RingFace = &faces[*previous];
                    let opposite = key((previous.corners[2], previous.corners[3]));
                    if key(entry) != key((previous.corners[0], previous.corners[1]))
                        && key(entry) != opposite
                    {
                        return Err(LoopCutError::TwistedRing);
                    }
                    continue;
                }
                let offset = (0..4)
                    .find(|&i| key((original[i], original[(i + 1) % 4])) == key(entry))
                    .ok_or(LoopCutError::InvalidMesh)?;
                let corners = std::array::from_fn(|i| original[(offset + i) % 4]);
                let exit = if corners[0] == entry.0 {
                    (corners[3], corners[2])
                } else {
                    (corners[2], corners[3])
                };
                if let Some(&existing) = directions.get(&key(exit)) {
                    if existing != exit {
                        return Err(LoopCutError::TwistedRing);
                    }
                } else {
                    directions.insert(key(exit), exit);
                    pending.push(exit);
                }
                visited_faces.insert(face_index, faces.len());
                faces.push(RingFace {
                    index: face_index,
                    original,
                    corners,
                    offset,
                });
            }
        }
        Ok(Self {
            faces,
            edges,
            closed,
        })
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn validate(&self, mesh: &Mesh) -> Result<(), LoopCutError> {
        for face in &self.faces {
            let current = mesh.faces.get(face.index).ok_or(LoopCutError::StaleRing)?;
            if current.verts != face.original || current.uv.len() != 4 {
                return Err(LoopCutError::StaleRing);
            }
            if current.uv.iter().flatten().any(|v| !v.is_finite()) {
                return Err(LoopCutError::InvalidMesh);
            }
        }
        for &(a, b) in &self.edges {
            for i in [a, b] {
                if !mesh
                    .verts
                    .get(i as usize)
                    .is_some_and(|v| v.vec().is_finite() && v.color.iter().all(|v| v.is_finite()))
                {
                    return Err(LoopCutError::InvalidMesh);
                }
            }
        }
        Ok(())
    }

    pub fn preview(
        &self,
        mesh: &Mesh,
        cuts: usize,
        slide: f32,
    ) -> Result<Vec<[Vec3; 2]>, LoopCutError> {
        self.validate(mesh)?;
        let fractions = fractions(cuts, slide)?;
        let oriented: HashMap<_, _> = self.edges.iter().map(|&edge| (key(edge), edge)).collect();
        let mut lines = Vec::with_capacity(self.faces.len() * cuts);
        for face in &self.faces {
            let entry = oriented[&key((face.corners[0], face.corners[1]))];
            let exit = oriented[&key((face.corners[2], face.corners[3]))];
            for &fraction in &fractions {
                lines.push([
                    interpolate(mesh, entry, fraction),
                    interpolate(mesh, exit, fraction),
                ]);
            }
        }
        Ok(lines)
    }

    /// Splits each ring quad into strips, sharing every new boundary vertex.
    /// Existing face UV seams are preserved through independent interpolation.
    pub fn apply(&self, mesh: &Mesh, cuts: usize, slide: f32) -> Result<Mesh, LoopCutError> {
        self.apply_even(mesh, cuts, slide, false)
    }

    /// Even Loop Cut: equal spacing along the ring, independent of slide.
    pub fn apply_even(
        &self,
        mesh: &Mesh,
        cuts: usize,
        slide: f32,
        even: bool,
    ) -> Result<Mesh, LoopCutError> {
        self.validate(mesh)?;
        let fractions = fractions_mode(cuts, slide, even)?;
        let mut result = mesh.clone();
        result.deselect_all();
        let mut splits = HashMap::new();
        for &(a, b) in &self.edges {
            let va = &mesh.verts[a as usize];
            let vb = &mesh.verts[b as usize];
            let mut points = vec![(a, 0.0)];
            for &fraction in &fractions {
                let position = va.vec().lerp(vb.vec(), fraction);
                let index =
                    u32::try_from(result.verts.len()).map_err(|_| LoopCutError::InvalidMesh)?;
                result.verts.push(Vertex {
                    pos: position.to_array(),
                    color: Vec3::from_array(va.color)
                        .lerp(Vec3::from_array(vb.color), fraction)
                        .to_array(),
                    selected: true,
                });
                points.push((index, fraction));
            }
            points.push((b, 1.0));
            splits.insert(key((a, b)), points);
        }
        let ring_faces: HashMap<_, _> = self.faces.iter().map(|f| (f.index, f)).collect();
        let mut faces = Vec::with_capacity(mesh.faces.len() + self.faces.len() * cuts);
        for (index, original) in result.faces.iter().enumerate() {
            let Some(ring_face) = ring_faces.get(&index) else {
                faces.push(original.clone());
                continue;
            };
            let [a, b, c, d] = ring_face.corners;
            let entry = ordered_split(&splits, a, b)?;
            let exit = ordered_split(&splits, d, c)?;
            let uv: [Vec2; 4] =
                std::array::from_fn(|i| Vec2::from_array(original.uv[(ring_face.offset + i) % 4]));
            for strip in 0..=cuts {
                let mut face = Face::with_uv(
                    vec![
                        entry[strip].0,
                        entry[strip + 1].0,
                        exit[strip + 1].0,
                        exit[strip].0,
                    ],
                    vec![
                        uv[0].lerp(uv[1], entry[strip].1).to_array(),
                        uv[0].lerp(uv[1], entry[strip + 1].1).to_array(),
                        uv[3].lerp(uv[2], exit[strip + 1].1).to_array(),
                        uv[3].lerp(uv[2], exit[strip].1).to_array(),
                    ],
                );
                face.selected = false;
                faces.push(face);
                if strip > 0 {
                    result
                        .selected_edges
                        .insert(key((entry[strip].0, exit[strip].0)));
                }
            }
        }
        result.faces = faces;
        Ok(result)
    }
}

fn fractions(cuts: usize, slide: f32) -> Result<Vec<f32>, LoopCutError> {
    fractions_mode(cuts, slide, false)
}

/// Uniform (even) distribution ignores slide so strips have equal length.
fn fractions_mode(cuts: usize, slide: f32, even: bool) -> Result<Vec<f32>, LoopCutError> {
    if !(1..=32).contains(&cuts) || !slide.is_finite() || !(-1.0..=1.0).contains(&slide) {
        return Err(LoopCutError::InvalidParameters);
    }
    if even {
        return Ok((1..=cuts).map(|i| i as f32 / (cuts + 1) as f32).collect());
    }
    let slide = slide.clamp(-0.999, 0.999);
    Ok((1..=cuts)
        .map(|i| (i as f32 + slide) / (cuts + 1) as f32)
        .collect())
}

fn interpolate(mesh: &Mesh, edge: Edge, fraction: f32) -> Vec3 {
    mesh.verts[edge.0 as usize]
        .vec()
        .lerp(mesh.verts[edge.1 as usize].vec(), fraction)
}

fn ordered_split(
    splits: &HashMap<Edge, Vec<(u32, f32)>>,
    a: u32,
    b: u32,
) -> Result<Vec<(u32, f32)>, LoopCutError> {
    let points = splits.get(&key((a, b))).ok_or(LoopCutError::StaleRing)?;
    if points.first().is_some_and(|&(id, _)| id == a) {
        Ok(points.clone())
    } else {
        Ok(points.iter().rev().map(|&(id, t)| (id, 1.0 - t)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_closed(mesh: &Mesh) {
        let mut edges: HashMap<Edge, usize> = HashMap::new();
        for face in &mesh.faces {
            assert_eq!(face.verts.len(), 4);
            assert_eq!(face.uv.len(), 4);
            for i in 0..4 {
                *edges
                    .entry(key((face.verts[i], face.verts[(i + 1) % 4])))
                    .or_default() += 1;
            }
        }
        assert!(edges.values().all(|&count| count == 2));
        assert_eq!(
            mesh.verts.len() as isize - edges.len() as isize + mesh.faces.len() as isize,
            2
        );
    }

    #[test]
    fn cube_ring_closes_and_multiple_cuts_share_vertices() {
        let mesh = Mesh::cube(2.0);
        for seed in mesh.edges_unique() {
            let ring = LoopRing::discover(&mesh, seed).unwrap();
            assert!(ring.is_closed());
            assert_eq!(ring.face_count(), 4);
            assert_eq!(ring.edge_count(), 4);
            for cuts in [1, 3, 8] {
                let result = ring.apply(&mesh, cuts, 0.0).unwrap();
                assert_eq!(result.verts.len(), 8 + 4 * cuts);
                assert_eq!(result.faces.len(), 6 + 4 * cuts);
                assert_eq!(result.selected_edges.len(), 4 * cuts);
                assert_closed(&result);
            }
        }
    }

    #[test]
    fn slide_preview_matches_new_vertices_at_extremes() {
        let mesh = Mesh::cube(2.0);
        let ring = LoopRing::discover(&mesh, mesh.edges_unique()[0]).unwrap();
        for slide in [-1.0, -0.4, 0.0, 0.7, 1.0] {
            let preview = ring.preview(&mesh, 2, slide).unwrap();
            let result = ring.apply(&mesh, 2, slide).unwrap();
            assert_eq!(preview.len(), 8);
            for point in preview.iter().flatten() {
                assert!(
                    result
                        .verts
                        .iter()
                        .skip(8)
                        .any(|v| v.vec().distance(*point) < 1e-6)
                );
            }
            assert_closed(&result);
            assert!(
                result
                    .faces
                    .iter()
                    .enumerate()
                    .all(|(i, _)| result.face_normal(i).length_squared() > 0.9)
            );
        }
    }

    #[test]
    fn open_quad_strip_preserves_uv_interpolation() {
        let mut mesh = Mesh {
            verts: vec![
                Vertex::new(0.0, 0.0, 0.0),
                Vertex::new(1.0, 0.0, 0.0),
                Vertex::new(1.0, 1.0, 0.0),
                Vertex::new(0.0, 1.0, 0.0),
            ],
            faces: vec![Face::with_uv(
                vec![0, 1, 2, 3],
                vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            )],
            ..Mesh::default()
        };
        mesh.verts[0].color = [1.0, 0.0, 0.0];
        mesh.verts[1].color = [0.0, 0.0, 1.0];
        let ring = LoopRing::discover(&mesh, (0, 1)).unwrap();
        assert!(!ring.is_closed());
        let result = ring.apply(&mesh, 1, 0.0).unwrap();
        assert_eq!(result.faces.len(), 2);
        assert_eq!(result.verts[4].color, [0.5, 0.0, 0.5]);
        assert_eq!(
            result.faces[0].uv,
            vec![[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]]
        );
    }

    #[test]
    fn invalid_ring_and_parameters_never_mutate_source() {
        let mesh = Mesh::cube(2.0);
        let before = mesh.verts.iter().map(|v| v.pos).collect::<Vec<_>>();
        let ring = LoopRing::discover(&mesh, mesh.edges_unique()[0]).unwrap();
        for (cuts, slide) in [(0, 0.0), (33, 0.0), (1, f32::NAN), (1, 2.0)] {
            assert!(matches!(
                ring.apply(&mesh, cuts, slide),
                Err(LoopCutError::InvalidParameters)
            ));
        }
        assert_eq!(before, mesh.verts.iter().map(|v| v.pos).collect::<Vec<_>>());
        let mut stale = mesh.clone();
        stale.faces[0].verts.reverse();
        assert!(ring.apply(&stale, 1, 0.0).is_err());
        let mut triangle = mesh.clone();
        triangle.faces[0].verts.pop();
        triangle.faces[0].uv.pop();
        let seed = (triangle.faces[0].verts[0], triangle.faces[0].verts[1]);
        assert!(matches!(
            LoopRing::discover(&triangle, seed),
            Err(LoopCutError::NonQuad)
        ));
    }

    #[test]
    fn cylinder_side_ring_preserves_triangle_caps_and_winding() {
        let mesh = Mesh::cylinder(12, 1.0, 2.0);
        // Anel inferior 0..12, superior 12..24: aresta vertical (0, 12).
        let ring = LoopRing::discover(&mesh, (0, 12)).unwrap();
        assert!(ring.is_closed());
        assert_eq!(ring.face_count(), 12);
        let result = ring.apply(&mesh, 3, 0.35).unwrap();
        assert_eq!(
            result
                .faces
                .iter()
                .filter(|face| face.verts.len() == 3)
                .count(),
            24
        );
        let mut edges: HashMap<Edge, usize> = HashMap::new();
        for (index, face) in result.faces.iter().enumerate() {
            for i in 0..face.verts.len() {
                *edges
                    .entry(key((face.verts[i], face.verts[(i + 1) % face.verts.len()])))
                    .or_default() += 1;
            }
            if face.verts.len() == 4 {
                let center = result.face_centroid(index);
                let radial = Vec3::new(center.x, 0.0, center.z).normalize();
                assert!(result.face_normal(index).dot(radial) > 0.9);
            }
        }
        assert!(edges.values().all(|&count| count == 2));
    }

    #[test]
    fn nonmanifold_branch_is_rejected() {
        let mut mesh = Mesh::cube(2.0);
        let face = mesh.faces[0].clone();
        let seed = (face.verts[0], face.verts[1]);
        mesh.faces.push(face);
        assert!(matches!(
            LoopRing::discover(&mesh, seed),
            Err(LoopCutError::NonManifold)
        ));
    }
}
