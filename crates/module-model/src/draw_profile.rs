//! Draw Profile (§9) — desenha silhueta 2D sobre referência/vista ortográfica,
//! triangula (ear clipping) e gera malha por extrusão ou revolve.
//!
//! Fluxo: ative numa vista ortográfica → cliques adicionam pontos → clique
//! perto do 1º ponto (ou botão) fecha → Gerar (extrude/revolve).

use petunia_core::{AppState, ProfileState};
use petunia_mesh::Mesh;

use super::Tool;

/// Converte cursor NDC em coords 2D do plano do perfil (com snap opcional).
pub fn profile_screen_to_plane(state: &AppState, nx: f32, ny: f32) -> Option<[f32; 2]> {
    let (origin, dir) = state.camera.ray(nx, ny);
    let p = &state.profile;
    let n = glam::Vec3::from(p.normal);
    let denom = dir.dot(n);
    if denom.abs() < 1e-6 {
        return None; // raio paralelo ao plano
    }
    let o = glam::Vec3::from(p.origin);
    let t = (o - origin).dot(n) / denom;
    if t < 0.0 {
        return None;
    }
    let hit = origin + dir * t;
    let d = hit - o;
    let mut x = d.dot(glam::Vec3::from(p.right));
    let mut y = d.dot(glam::Vec3::from(p.up));
    if p.snap {
        x = (x * 4.0).round() / 4.0;
        y = (y * 4.0).round() / 4.0;
    }
    Some([x, y])
}

/// Adiciona ponto (clique no viewport). Fecha se perto do primeiro.
pub fn profile_add_point(state: &mut AppState, nx: f32, ny: f32) {
    if state.profile.closed {
        return;
    }
    let Some([x, y]) = profile_screen_to_plane(state, nx, ny) else {
        return;
    };
    let n = state.profile.points.len();
    if n >= 3 {
        let f = state.profile.points[0];
        if (f[0] - x).hypot(f[1] - y) < 0.25 {
            state.profile.closed = true;
            state.set_status(state.t("profile.closed"));
            state.mark_dirty();
            return;
        }
    }
    if state.profile.points.len() >= 512 {
        state.set_status("max 512 pts".to_string());
        return;
    }
    state.profile.points.push([x, y]);
    state
        .profile
        .nodes
        .push(petunia_mesh::curve::BezierNode::new([x, y]));
    state.mark_dirty();
}

/// Captura o frame 2D da câmera atual (chamar ao ativar).
pub fn profile_capture_frame(state: &mut AppState) {
    let right = state.camera.right().to_array();
    let up = state.camera.up().to_array();
    let origin = state.camera.target.to_array();
    let normal = (-state.camera.forward()).to_array();
    state.profile.right = right;
    state.profile.up = up;
    state.profile.origin = origin;
    state.profile.normal = normal;
    state.profile.points.clear();
    state.profile.nodes.clear();
    state.profile.closed = false;
}

pub fn generate_extrude(state: &mut AppState) {
    let p: ProfileState = state.profile.clone();
    let effective = p.effective_points();
    if effective.len() < 3 || !p.closed {
        state.set_status(state.t("profile.need_closed"));
        return;
    }
    match Mesh::from_polygon(&effective, p.depth.max(0.05)) {
        Ok(mut m) => {
            // leva do frame XY/Z-local para o mundo
            let r = glam::Vec3::from(p.right);
            let u = glam::Vec3::from(p.up);
            let o = glam::Vec3::from(p.origin);
            let n = glam::Vec3::from(p.normal);
            for v in &mut m.verts {
                let q = glam::Vec3::from(v.pos);
                v.pos = (o + r * q.x + u * q.y + n * q.z).to_array();
            }
            state.checkpoint("draw profile");
            state.project.add("Profile", m);
            state.profile.clear();
            state.sync_selection();
            state.emit_mesh_changed();
            state.set_status(state.t("profile.generated"));
        }
        Err(e) => state.set_status(format!("profile: {e}")),
    }
}

pub fn generate_revolve(state: &mut AppState) {
    let p = state.profile.clone();
    let effective = p.effective_points();
    if effective.len() < 2 {
        state.set_status(state.t("profile.need_points"));
        return;
    }
    // perfil aberto vale para revolve (não exige closed)
    let angle = if p.revolve_angle <= 0.0 {
        360.0
    } else {
        p.revolve_angle
    };
    match Mesh::revolve_angle(&effective, p.revolve_segments.max(3), angle) {
        Ok(mut m) => {
            let r = glam::Vec3::from(p.right);
            let u = glam::Vec3::from(p.up);
            let n = glam::Vec3::from(p.normal);
            let o = glam::Vec3::from(p.origin);
            for v in &mut m.verts {
                let q = glam::Vec3::from(v.pos);
                v.pos = (o + r * q.x + u * q.y + n * q.z).to_array();
            }
            state.checkpoint("revolve profile");
            state.project.add("Revolved", m);
            state.profile.clear();
            state.sync_selection();
            state.emit_mesh_changed();
            state.set_status(state.t("profile.generated"));
        }
        Err(e) => state.set_status(format!("revolve: {e}")),
    }
}

/// Extrai caminho guia 3D a partir da malha ativa (arestas selecionadas em cadeia ou vértices selecionados).
pub fn extract_sweep_path_from_mesh(mesh: &Mesh) -> Option<(Vec<glam::Vec3>, bool)> {
    if !mesh.selected_edges.is_empty() {
        use std::collections::HashMap;
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
        for &(a, b) in &mesh.selected_edges {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }

        let start = adj
            .iter()
            .find(|(_, neighbors)| neighbors.len() == 1)
            .map(|(&v, _)| v)
            .or_else(|| adj.keys().copied().next())?;

        let mut chain = vec![start];
        let mut visited_edges = std::collections::HashSet::new();
        let mut curr = start;

        loop {
            let next_opt = adj.get(&curr).and_then(|neighbors| {
                neighbors.iter().copied().find(|&n| {
                    let key = petunia_mesh::edge_key(curr, n);
                    !visited_edges.contains(&key)
                })
            });

            if let Some(next) = next_opt {
                visited_edges.insert(petunia_mesh::edge_key(curr, next));
                chain.push(next);
                curr = next;
                if curr == start {
                    break;
                }
            } else {
                break;
            }
        }

        if chain.len() >= 2 {
            let is_closed = chain.len() > 2 && chain.first() == chain.last();
            let pts: Vec<glam::Vec3> = chain
                .into_iter()
                .filter_map(|idx| mesh.verts.get(idx as usize).map(|v| v.vec()))
                .collect();
            if pts.len() >= 2 {
                return Some((pts, is_closed));
            }
        }
    }

    let selected_verts: Vec<glam::Vec3> = mesh
        .verts
        .iter()
        .filter(|v| v.selected)
        .map(|v| v.vec())
        .collect();

    if selected_verts.len() >= 2 {
        Some((selected_verts, false))
    } else {
        None
    }
}

/// Gera uma malha por varredura 3D (Sweep) do perfil 2D ao longo de um caminho guia 3D.
pub fn generate_sweep(state: &mut AppState) {
    let p = state.profile.clone();
    let effective = p.effective_points();
    if effective.len() < 2 {
        state.set_status(state.t("profile.need_points"));
        return;
    }

    let extracted = state
        .project
        .active_mesh()
        .and_then(extract_sweep_path_from_mesh);

    let (path, closed_path) = if let Some((path_pts, is_closed)) = extracted {
        (path_pts, is_closed)
    } else {
        let r = glam::Vec3::from(p.right).normalize_or_zero();
        let u = glam::Vec3::from(p.up).normalize_or_zero();
        let n = glam::Vec3::from(p.normal).normalize_or_zero();
        let o = glam::Vec3::from(p.origin);
        let l = p.depth.max(1.0);
        let default_path = vec![
            o,
            o + n * (l * 0.3) + u * (l * 0.15),
            o + n * (l * 0.7) + u * (l * 0.2) + r * (l * 0.15),
            o + n * l + r * (l * 0.3),
        ];
        (default_path, false)
    };

    let options = petunia_mesh::sweep::SweepOptions {
        closed_path,
        closed_profile: p.closed,
        cap_start: true,
        cap_end: true,
        miter: true,
        miter_limit: 3.0,
    };

    match Mesh::from_sweep(&effective, &path, options) {
        Ok(m) => {
            state.checkpoint("sweep profile");
            state.project.add("Sweep", m);
            state.profile.clear();
            state.sync_selection();
            state.emit_mesh_changed();
            state.set_status(state.t("profile.generated"));
        }
        Err(e) => state.set_status(format!("sweep: {e}")),
    }
}

/// Define um perfil 2D retangular centralizado no plano ativo.
pub fn profile_set_rectangle(state: &mut AppState, width: f32, height: f32) {
    profile_capture_frame(state);
    let w = if width.is_finite() && width > 0.0 {
        width
    } else {
        2.0
    };
    let h = if height.is_finite() && height > 0.0 {
        height
    } else {
        1.5
    };
    state.profile.points = vec![
        [-w * 0.5, -h * 0.5],
        [w * 0.5, -h * 0.5],
        [w * 0.5, h * 0.5],
        [-w * 0.5, h * 0.5],
    ];
    state.profile.nodes = state
        .profile
        .points
        .iter()
        .map(|&p| petunia_mesh::curve::BezierNode::new(p))
        .collect();
    state.profile.closed = true;
    state.session.tools.active_tool = "draw_profile".to_string();
    state.set_status(format!(
        "Profile 2D Rectangle created ({:.1} x {:.1}): choose Extrude or Revolve",
        w, h
    ));
    state.mark_dirty();
}

/// Define um perfil 2D circular centralizado no plano ativo.
pub fn profile_set_circle(state: &mut AppState, radius: f32, segments: usize) {
    profile_capture_frame(state);
    let r = if radius.is_finite() && radius > 0.0 {
        radius
    } else {
        1.0
    };
    let segs = segments.clamp(6, 64);
    let mut points = Vec::with_capacity(segs);
    for i in 0..segs {
        let angle = std::f32::consts::TAU * (i as f32) / (segs as f32);
        points.push([r * angle.cos(), r * angle.sin()]);
    }
    state.profile.points = points.clone();
    state.profile.nodes = points
        .into_iter()
        .map(petunia_mesh::curve::BezierNode::new)
        .collect();
    state.profile.closed = true;
    state.session.tools.active_tool = "draw_profile".to_string();
    state.set_status(format!(
        "Profile 2D Circle created (r={:.1}, {} segs): choose Extrude or Revolve",
        r, segs
    ));
    state.mark_dirty();
}

/// Converte todos os nós do perfil atual em curvas Bézier suaves (G1/C1 contínuas).
pub fn profile_smooth_curves(state: &mut AppState) {
    if state.profile.nodes.is_empty() && !state.profile.points.is_empty() {
        state.profile.nodes = state
            .profile
            .points
            .iter()
            .map(|&p| petunia_mesh::curve::BezierNode::new(p))
            .collect();
    }
    let mut path = petunia_mesh::curve::BezierPath {
        nodes: state.profile.nodes.clone(),
        closed: state.profile.closed,
    };
    path.auto_smooth(0.25);
    state.profile.nodes = path.nodes;
    state.mark_dirty();
    state.set_status("Profile curves smoothed (Cubic Bézier)".to_string());
}

/// Converte todos os nós do perfil atual em cantos retos (Sharp).
pub fn profile_clear_curves(state: &mut AppState) {
    for node in &mut state.profile.nodes {
        node.handle_in = None;
        node.handle_out = None;
        node.kind = petunia_mesh::curve::BezierNodeKind::Sharp;
    }
    state.mark_dirty();
    state.set_status("Profile corners sharpened".to_string());
}

/// Ajusta a espessura de parede (Wall Thickness) para perfis ocos.
pub fn profile_set_wall_thickness(state: &mut AppState, thickness: f32) {
    state.profile.wall_thickness = thickness.max(0.0);
    state.mark_dirty();
}

/// Ajusta a tolerância de suavização de tesselação das curvas Bézier.
pub fn profile_set_curve_smoothness(state: &mut AppState, smoothness: f32) {
    state.profile.curve_smoothness = smoothness.clamp(0.005, 0.2);
    state.mark_dirty();
}

#[derive(Default)]
pub struct DrawProfileTool;

impl Tool for DrawProfileTool {
    fn id(&self) -> &'static str {
        "draw_profile"
    }
    fn label_key(&self) -> &'static str {
        "tools.draw_profile"
    }
    fn hint_key(&self) -> &'static str {
        "hints.draw_profile"
    }
    fn icon(&self) -> &'static str {
        "✎"
    }
    fn shortcut(&self) -> &'static str {
        "Shift+P"
    }
    fn on_activate(&self, state: &mut AppState) {
        profile_capture_frame(state);
        state.set_status(state.t("hints.draw_profile"));
        state.mark_dirty();
    }
}
