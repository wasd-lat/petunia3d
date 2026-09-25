//! Algoritmo de Varredura 3D (Sweep) com Rotation Minimizing Frames (RMF).
//!
//! Permite varrer um perfil 2D ao longo de uma curva guia 3D arbitrária (spine),
//! com minimização de torção (Parallel Transport / Double Reflection RMF),
//! mitering com contenção em cantos vivos e tampas (end caps) trianguladas.

use glam::Vec3;

use super::{Face, Mesh, Vertex, triangulate};

/// Opções de configuração para a operação de Sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepOptions {
    /// Se o caminho forma um laço fechado contínuo.
    pub closed_path: bool,
    /// Se o perfil de seção transversal é um laço fechado (ex: tubo vs fita/fita aberta).
    pub closed_profile: bool,
    /// Se deve gerar tampa no início (se o caminho for aberto e o perfil fechado).
    pub cap_start: bool,
    /// Se deve gerar tampa no fim (se o caminho for aberto e o perfil fechado).
    pub cap_end: bool,
    /// Se deve alinhar os anéis no plano bissetor em cantos vivos (mitering).
    pub miter: bool,
    /// Limite máximo de extensão do miter (previne pontas infinitas em cantos agudos).
    pub miter_limit: f32,
}

impl Default for SweepOptions {
    fn default() -> Self {
        Self {
            closed_path: false,
            closed_profile: true,
            cap_start: true,
            cap_end: true,
            miter: true,
            miter_limit: 3.0,
        }
    }
}

/// Sistema de coordenadas ortonormais local (Frame RMF) ao longo do caminho.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SweepFrame {
    pub origin: Vec3,
    pub tangent: Vec3,
    pub normal: Vec3,   // Eixo local X do perfil
    pub binormal: Vec3, // Eixo local Y do perfil
}

impl SweepFrame {
    /// Projeta um ponto 2D do perfil no espaço 3D usando este frame.
    #[inline]
    pub fn to_world(&self, pt: [f32; 2]) -> Vec3 {
        self.origin + self.normal * pt[0] + self.binormal * pt[1]
    }
}

/// Calcula Rotation Minimizing Frames (RMF) usando o método de Double Reflection (Wang et al., 2008).
///
/// Este método elimina o gimbal lock e torções indesejadas (flipping) em curvas 3D,
/// garantindo transporte paralelo de 4ª ordem globalmente sem singularidades.
pub fn compute_rmf_frames(path: &[Vec3], closed: bool) -> Result<Vec<SweepFrame>, String> {
    let n = path.len();
    if n < 2 {
        return Err("O caminho de Sweep precisa de pelo menos 2 pontos".to_string());
    }

    // Calcula tangentes para cada ponto do caminho
    let mut tangents = Vec::with_capacity(n);
    for i in 0..n {
        let t = if closed {
            let prev = path[(i + n - 1) % n];
            let next = path[(i + 1) % n];
            (next - prev).normalize_or_zero()
        } else if i == 0 {
            (path[1] - path[0]).normalize_or_zero()
        } else if i == n - 1 {
            (path[n - 1] - path[n - 2]).normalize_or_zero()
        } else {
            let prev = path[i - 1];
            let next = path[i + 1];
            (next - prev).normalize_or_zero()
        };
        let t = if t.length_squared() > 1e-6 {
            t
        } else if i + 1 < n {
            (path[i + 1] - path[i]).normalize_or_zero()
        } else {
            Vec3::Z
        };
        tangents.push(t);
    }

    // Define o frame inicial no ponto 0
    let t0 = tangents[0];
    let up_guide = if t0.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let r0 = t0.cross(up_guide).normalize_or_zero();
    let s0 = t0.cross(r0).normalize_or_zero();

    let mut frames = Vec::with_capacity(n);
    frames.push(SweepFrame {
        origin: path[0],
        tangent: t0,
        normal: r0,
        binormal: s0,
    });

    // Propaga via Double Reflection
    for i in 0..(n - 1) {
        let x_i = path[i];
        let x_next = path[i + 1];
        let v1 = x_next - x_i;
        let c1 = v1.length_squared();

        let r_curr = frames[i].normal;
        let t_curr = frames[i].tangent;
        let t_next = tangents[i + 1];

        if c1 < 1e-8 {
            frames.push(SweepFrame {
                origin: x_next,
                tangent: t_next,
                normal: r_curr,
                binormal: frames[i].binormal,
            });
            continue;
        }

        // Primeira reflexão (em torno do hiperplano médio entre x_i e x_next)
        let r_i_l = r_curr - (2.0 / c1) * v1.dot(r_curr) * v1;
        let t_i_l = t_curr - (2.0 / c1) * v1.dot(t_curr) * v1;

        // Segunda reflexão (em torno do hiperplano médio entre t_i_l e t_next)
        let v2 = t_next - t_i_l;
        let c2 = v2.length_squared();

        let r_next = if c2 < 1e-8 {
            r_i_l
        } else {
            r_i_l - (2.0 / c2) * v2.dot(r_i_l) * v2
        };

        let r_next = r_next.normalize_or_zero();
        let s_next = t_next.cross(r_next).normalize_or_zero();

        frames.push(SweepFrame {
            origin: x_next,
            tangent: t_next,
            normal: r_next,
            binormal: s_next,
        });
    }

    // Se o caminho for fechado, distribui o erro angular acumulado (twist compensation)
    if closed && n > 2 {
        let last_frame = frames[n - 1];
        let v_close = path[0] - path[n - 1];
        let c_close = v_close.length_squared();

        let (r_end, _s_end) = if c_close > 1e-8 {
            let r_l =
                last_frame.normal - (2.0 / c_close) * v_close.dot(last_frame.normal) * v_close;
            let t_l =
                last_frame.tangent - (2.0 / c_close) * v_close.dot(last_frame.tangent) * v_close;
            let v_t = t0 - t_l;
            let c_t = v_t.length_squared();
            let r_proj = if c_t < 1e-8 {
                r_l
            } else {
                r_l - (2.0 / c_t) * v_t.dot(r_l) * v_t
            };
            let r_norm = r_proj.normalize_or_zero();
            (r_norm, t0.cross(r_norm).normalize_or_zero())
        } else {
            (last_frame.normal, last_frame.binormal)
        };

        // Ângulo de defasagem entre o fechamento e o frame inicial
        let cos_theta = r_end.dot(r0).clamp(-1.0, 1.0);
        let sin_theta = r_end.dot(s0);
        let delta_angle = sin_theta.atan2(cos_theta);

        // Desrotaciona gradualmente os frames ao longo do comprimento
        for (i, frame) in frames.iter_mut().enumerate() {
            let factor = (i as f32) / (n as f32);
            let angle = -delta_angle * factor;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            let r_rotated = frame.normal * cos_a + frame.binormal * sin_a;
            let s_rotated = frame.tangent.cross(r_rotated).normalize_or_zero();
            frame.normal = r_rotated;
            frame.binormal = s_rotated;
        }
    }

    Ok(frames)
}

/// Aplica o alinhamento de bissetriz (mitering) em vértices de canto.
fn apply_mitering(
    frame: &SweepFrame,
    prev_tangent: Option<Vec3>,
    next_tangent: Option<Vec3>,
    pt: [f32; 2],
    miter_limit: f32,
) -> Vec3 {
    let p_local = frame.normal * pt[0] + frame.binormal * pt[1];

    let (Some(t_in), Some(t_out)) = (prev_tangent, next_tangent) else {
        return frame.origin + p_local;
    };

    let bisector = (t_in + t_out).normalize_or_zero();
    let cos_half = t_in.dot(bisector);

    if cos_half <= 0.1 || bisector.length_squared() < 0.5 {
        return frame.origin + p_local;
    }

    // Fator de escala do miter na direção de curvatura
    let miter_scale = (1.0 / cos_half).min(miter_limit.max(1.0));

    // Vetor de dobra no plano (tangente de entrada x saída)
    let bend_dir = t_in.cross(t_out);
    if bend_dir.length_squared() < 1e-6 {
        return frame.origin + p_local;
    }

    let bend_axis = bend_dir.normalize();
    let p_proj_bend = p_local.dot(bend_axis) * bend_axis;
    let p_perp_bend = p_local - p_proj_bend;

    frame.origin + p_proj_bend + p_perp_bend * miter_scale
}

/// Executa a varredura (Sweep) de um perfil 2D ao longo de um caminho 3D.
pub fn generate_sweep(
    profile: &[[f32; 2]],
    path: &[Vec3],
    options: SweepOptions,
) -> Result<Mesh, String> {
    let num_profile = profile.len();
    if num_profile < 2 {
        return Err("O perfil para Sweep precisa ter pelo menos 2 pontos".to_string());
    }
    let num_path = path.len();
    if num_path < 2 {
        return Err("O caminho para Sweep precisa ter pelo menos 2 pontos".to_string());
    }

    let frames = compute_rmf_frames(path, options.closed_path)?;

    // Calcula parâmetros UV ao longo do perfil (eixo U)
    let mut profile_u = Vec::with_capacity(num_profile);
    profile_u.push(0.0);
    let mut total_profile_len = 0.0;
    for i in 0..(num_profile - 1) {
        let d = (profile[i + 1][0] - profile[i][0]).hypot(profile[i + 1][1] - profile[i][1]);
        total_profile_len += d;
        profile_u.push(total_profile_len);
    }
    if total_profile_len > 1e-6 {
        for u in &mut profile_u {
            *u /= total_profile_len;
        }
    }

    // Calcula parâmetros UV ao longo do caminho (eixo V)
    let mut path_v = Vec::with_capacity(num_path);
    path_v.push(0.0);
    let mut total_path_len = 0.0;
    for i in 0..(num_path - 1) {
        let d = (path[i + 1] - path[i]).length();
        total_path_len += d;
        path_v.push(total_path_len);
    }
    if options.closed_path {
        total_path_len += (path[0] - path[num_path - 1]).length();
    }
    if total_path_len > 1e-6 {
        for v in &mut path_v {
            *v /= total_path_len;
        }
    }

    let mut mesh = Mesh::default();

    // 1. Gera todos os anéis de vértices
    for i in 0..num_path {
        let frame = &frames[i];
        let prev_tangent = if i > 0 {
            Some(frames[i - 1].tangent)
        } else if options.closed_path {
            Some(frames[num_path - 1].tangent)
        } else {
            None
        };
        let next_tangent = if i + 1 < num_path {
            Some(frames[i + 1].tangent)
        } else if options.closed_path {
            Some(frames[0].tangent)
        } else {
            None
        };

        for &pt in profile {
            let pt_world = if options.miter {
                apply_mitering(frame, prev_tangent, next_tangent, pt, options.miter_limit)
            } else {
                frame.to_world(pt)
            };
            mesh.verts
                .push(Vertex::new(pt_world.x, pt_world.y, pt_world.z));
        }
    }

    // 2. Conecta os anéis com faces quads
    let ring_steps = if options.closed_path {
        num_path
    } else {
        num_path - 1
    };

    for i in 0..ring_steps {
        let ring_curr = i;
        let ring_next = (i + 1) % num_path;

        let v_curr = path_v[ring_curr];
        let v_next = if options.closed_path && ring_next == 0 {
            1.0
        } else {
            path_v[ring_next]
        };

        for j in 0..(num_profile - 1) {
            let j_next = j + 1;
            let u_curr = profile_u[j];
            let u_next = profile_u[j_next];

            let idx_0 = (ring_curr * num_profile + j) as u32;
            let idx_1 = (ring_curr * num_profile + j_next) as u32;
            let idx_2 = (ring_next * num_profile + j_next) as u32;
            let idx_3 = (ring_next * num_profile + j) as u32;

            mesh.push_face(Face::with_uv(
                vec![idx_0, idx_1, idx_2, idx_3],
                vec![
                    [u_curr, v_curr],
                    [u_next, v_curr],
                    [u_next, v_next],
                    [u_curr, v_next],
                ],
            ));
        }

        // Se o perfil for fechado, fecha a lateral conectando o último ponto ao primeiro
        let is_profile_closed = options.closed_profile || {
            let p_first = profile[0];
            let p_last = profile[num_profile - 1];
            (p_first[0] - p_last[0]).hypot(p_first[1] - p_last[1]) < 1e-4
        };

        if is_profile_closed {
            let j = num_profile - 1;
            let j_next = 0;
            let idx_0 = (ring_curr * num_profile + j) as u32;
            let idx_1 = (ring_curr * num_profile + j_next) as u32;
            let idx_2 = (ring_next * num_profile + j_next) as u32;
            let idx_3 = (ring_next * num_profile + j) as u32;

            mesh.push_face(Face::with_uv(
                vec![idx_0, idx_1, idx_2, idx_3],
                vec![[1.0, v_curr], [0.0, v_curr], [0.0, v_next], [1.0, v_next]],
            ));
        }
    }

    // 3. Gera tampas de início e fim se aplicável
    if !options.closed_path && num_profile >= 3 && (options.cap_start || options.cap_end) {
        let Ok(tris) = triangulate::ear_clip(profile) else {
            return Ok(mesh);
        };
        if options.cap_start {
            for &[a, b, c] in &tris {
                let idx_a = a as u32;
                let idx_b = b as u32;
                let idx_c = c as u32;
                // Tampa inicial com normal voltada para trás (ordem invertida c, b, a)
                mesh.push_face(Face::with_uv(
                    vec![idx_c, idx_b, idx_a],
                    vec![profile[c], profile[b], profile[a]],
                ));
            }
        }

        if options.cap_end {
            let off = ((num_path - 1) * num_profile) as u32;
            for &[a, b, c] in &tris {
                let idx_a = off + a as u32;
                let idx_b = off + b as u32;
                let idx_c = off + c as u32;
                // Tampa final com normal voltada para a frente (ordem a, b, c)
                mesh.push_face(Face::with_uv(
                    vec![idx_a, idx_b, idx_c],
                    vec![profile[a], profile[b], profile[c]],
                ));
            }
        }
    }

    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rmf_straight_line() {
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 10.0),
        ];
        let frames = compute_rmf_frames(&path, false).unwrap();
        assert_eq!(frames.len(), 3);
        for f in &frames {
            assert!((f.tangent.dot(Vec3::Z) - 1.0).abs() < 1e-4);
            assert!(f.normal.is_normalized());
            assert!(f.binormal.is_normalized());
            assert!(f.normal.dot(f.tangent).abs() < 1e-4);
        }
    }

    #[test]
    fn test_rmf_90_degree_bend() {
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(5.0, 5.0, 0.0),
        ];
        let frames = compute_rmf_frames(&path, false).unwrap();
        assert_eq!(frames.len(), 3);
        // Cada frame deve ser estritamente ortonormal
        for f in &frames {
            assert!(f.tangent.is_normalized());
            assert!(f.normal.is_normalized());
            assert!(f.binormal.is_normalized());
            assert!(f.normal.dot(f.tangent).abs() < 1e-4);
            assert!(f.binormal.dot(f.tangent).abs() < 1e-4);
            assert!(f.normal.dot(f.binormal).abs() < 1e-4);
        }
    }

    #[test]
    fn test_sweep_straight_pipe() {
        let profile = vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ];
        let mesh = generate_sweep(
            &profile,
            &path,
            SweepOptions {
                closed_path: false,
                closed_profile: true,
                cap_start: true,
                cap_end: true,
                miter: true,
                miter_limit: 3.0,
            },
        )
        .unwrap();

        // 3 anéis de 4 vértices = 12 vértices
        assert_eq!(mesh.verts.len(), 12);
        // Deve conter faces laterais e tampas
        assert!(!mesh.faces.is_empty());
        for v in &mesh.verts {
            assert!(v.pos[0].is_finite());
            assert!(v.pos[1].is_finite());
            assert!(v.pos[2].is_finite());
        }
    }

    #[test]
    fn test_sweep_closed_loop() {
        // Caminho quadrado fechado
        let path = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
        ];
        let profile = vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
        let mesh = generate_sweep(
            &profile,
            &path,
            SweepOptions {
                closed_path: true,
                closed_profile: true,
                cap_start: false,
                cap_end: false,
                miter: true,
                miter_limit: 3.0,
            },
        )
        .unwrap();

        // 4 anéis de 4 vértices = 16 vértices
        assert_eq!(mesh.verts.len(), 16);
        assert!(!mesh.faces.is_empty());
    }
}
