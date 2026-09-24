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
        if (f[0] - x).hypot(f[1] - y) < 0.15 {
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
    state.profile.closed = false;
}

pub fn generate_extrude(state: &mut AppState) {
    let p: ProfileState = state.profile.clone();
    if p.points.len() < 3 || !p.closed {
        state.set_status(state.t("profile.need_closed"));
        return;
    }
    match Mesh::from_polygon(&p.points, p.depth.max(0.05)) {
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
    if p.points.len() < 2 {
        state.set_status(state.t("profile.need_points"));
        return;
    }
    // perfil aberto vale para revolve (não exige closed)
    match Mesh::revolve(&p.points, p.revolve_segments.max(3)) {
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
    state.profile.points = points;
    state.profile.closed = true;
    state.session.tools.active_tool = "draw_profile".to_string();
    state.set_status(format!(
        "Profile 2D Circle created (r={:.1}, {} segs): choose Extrude or Revolve",
        r, segs
    ));
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
