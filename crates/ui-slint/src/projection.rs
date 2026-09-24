// Screen-space projection, gizmo geometry, raycasting, and viewport math.
// Projeção em espaço de tela, geometria do gizmo, raycasting e matemática de viewport.

use crate::*;
use petunia_core::{AppState, Camera, PivotPoint, SelectionDomain, Workspace};

pub(crate) fn project_preview_segment(
    camera: &Camera,
    viewport: [f32; 2],
    a: glam::Vec3,
    b: glam::Vec3,
    commands: &mut String,
) {
    let matrix = camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = matrix * point.extend(1.0);
        if !clip.is_finite() || clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
            return None;
        }
        Some([
            (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
            (0.5 - clip.y / clip.w * 0.5) * viewport[1],
        ])
    };
    if let (Some(a), Some(b)) = (project(a), project(b)) {
        use std::fmt::Write as _;
        let _ = write!(
            commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            a[0], a[1], b[0], b[1]
        );
    }
}

pub(crate) fn pivot_id(pivot: PivotPoint) -> &'static str {
    match pivot {
        PivotPoint::MedianPoint => "median",
        PivotPoint::BoundingBoxCenter => "bounds",
        PivotPoint::Cursor3D => "cursor",
        PivotPoint::IndividualOrigins => "individual",
    }
}

pub(crate) fn pivot_from_id(id: &str) -> Option<PivotPoint> {
    match id {
        "median" => Some(PivotPoint::MedianPoint),
        "bounds" => Some(PivotPoint::BoundingBoxCenter),
        "cursor" => Some(PivotPoint::Cursor3D),
        "individual" => Some(PivotPoint::IndividualOrigins),
        _ => None,
    }
}

/// Projeta o pivô da seleção e os três eixos do mundo para o overlay Slint.
/// Distância de um ponto a um segmento, em espaço de tela.
pub fn point_segment_distance(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [point[0] - a[0], point[1] - a[1]];
    let length_squared = ab[0] * ab[0] + ab[1] * ab[1];
    if length_squared <= 1.0e-6 {
        return (ap[0] * ap[0] + ap[1] * ap[1]).sqrt();
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / length_squared).clamp(0.0, 1.0);
    let closest = [a[0] + ab[0] * t, a[1] + ab[1] * t];
    ((point[0] - closest[0]).powi(2) + (point[1] - closest[1]).powi(2)).sqrt()
}

pub(crate) fn parse_lasso_path(path: &str) -> Option<Vec<[f32; 2]>> {
    if path.len() > 131_072 {
        return None;
    }
    let mut polygon = Vec::new();
    for part in path.split(';').filter(|part| !part.is_empty()) {
        let (x, y) = part.split_once(',')?;
        let x: f32 = x.parse().ok()?;
        let y: f32 = y.parse().ok()?;
        if !x.is_finite() || !y.is_finite() || polygon.len() >= 4096 {
            return None;
        }
        // O grab entrega valores fora da viewport. Projetar na borda mantém
        // a forma do laço, em vez de descartar pontos e abrir um corte.
        polygon.push([x.clamp(0.0, 1.0) * 2.0 - 1.0, 1.0 - y.clamp(0.0, 1.0) * 2.0]);
    }
    (polygon.len() >= 3).then_some(polygon)
}

pub(crate) fn compute_gizmo(state: &AppState, width: f32, height: f32) -> GizmoModel {
    /// Comprimento das hastes do gizmo de transformação, em px lógicos.
    const ROD_LENGTH: f32 = 72.0;
    /// Tamanho da seta: recuo da ponta e meia-largura da base.
    const ARROW_BACK: f32 = 13.0;
    const ARROW_HALF: f32 = 5.5;
    /// Tripé de navegação: no alto à direita, abaixo dos controles da viewport.
    const VIEW_MARGIN: f32 = 54.0;
    const VIEW_TOP: f32 = 108.0;
    const VIEW_LENGTH: f32 = 38.0;

    if width <= 1.0 || height <= 1.0 {
        return GizmoModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.05 {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        Some([
            (clip.x * inv_w * 0.5 + 0.5) * width,
            (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
        ])
    };
    // Camera-space directions preserve foreshortening: an axis pointing at
    // the viewer should shrink, not turn into a full-length diagonal.
    let screen_direction = |axis: glam::Vec3| -> [f32; 2] {
        [
            axis.dot(state.session.camera.right()),
            -axis.dot(state.session.camera.up()),
        ]
    };

    let mut model = GizmoModel::default();

    // Tripé de navegação: sempre visível quando a viewport tem tamanho válido.
    // Ele mostra a orientação da câmera, não a cena: por isso as hastes partem
    // de uma âncora fixa no canto, não de um ponto projetado.
    {
        model.view_origin_x = width - VIEW_MARGIN;
        model.view_origin_y = VIEW_TOP.min(height - VIEW_MARGIN);
        let origin = [VIEW_MARGIN, VIEW_MARGIN];
        for (axis, slot, endpoint) in [
            (
                glam::Vec3::X,
                &mut model.view_x_commands,
                &mut model.view_x_end,
            ),
            (
                glam::Vec3::Y,
                &mut model.view_y_commands,
                &mut model.view_y_end,
            ),
            (
                glam::Vec3::Z,
                &mut model.view_z_commands,
                &mut model.view_z_end,
            ),
        ] {
            let direction = screen_direction(axis);
            let end = [
                origin[0] + direction[0] * VIEW_LENGTH,
                origin[1] + direction[1] * VIEW_LENGTH,
            ];
            *endpoint = end;
            *slot = format!(
                "M {:.2} {:.2} L {:.2} {:.2} ",
                origin[0], origin[1], end[0], end[1]
            );
        }
    }

    // Hastes de transformação: tamanho fixo em tela. O modo combinado mantém
    // três famílias de handle simultâneas e selecionáveis.
    if state.workspace != Workspace::Model
        || !matches!(
            state.session.tools.active_tool.as_str(),
            "move" | "rotate" | "scale" | "transform"
        )
    {
        return model;
    }
    let Some(asset) = state.project.active() else {
        return model;
    };
    if asset.mesh.verts.is_empty() {
        return model;
    };
    let pivot = state.session.tools.modal.as_ref().map_or_else(
        || state.calculate_pivot(state.session.pivot_point),
        |modal| modal.pivot,
    );
    let Some(origin) = project(pivot) else {
        return model;
    };
    model.visible = true;
    model.origin_x = origin[0];
    model.origin_y = origin[1];

    for (axis, rod, arrow, scale, rotate) in [
        (
            glam::Vec3::X,
            &mut model.x_commands,
            &mut model.x_arrow_commands,
            &mut model.x_scale_commands,
            &mut model.x_rotate_commands,
        ),
        (
            glam::Vec3::Y,
            &mut model.y_commands,
            &mut model.y_arrow_commands,
            &mut model.y_scale_commands,
            &mut model.y_rotate_commands,
        ),
        (
            glam::Vec3::Z,
            &mut model.z_commands,
            &mut model.z_arrow_commands,
            &mut model.z_scale_commands,
            &mut model.z_rotate_commands,
        ),
    ] {
        if state.session.tools.active_tool == "rotate" {
            let tangent = if axis == glam::Vec3::X {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let bitangent = axis.cross(tangent);
            for segment in 0..=64 {
                let angle = segment as f32 * std::f32::consts::TAU / 64.0;
                let direction = screen_direction(tangent * angle.cos() + bitangent * angle.sin());
                rod.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if segment == 0 { "M" } else { "L" },
                    origin[0] + direction[0] * ROD_LENGTH,
                    origin[1] + direction[1] * ROD_LENGTH
                ));
            }
            continue;
        }
        let direction = screen_direction(axis);
        let end = [
            origin[0] + direction[0] * ROD_LENGTH,
            origin[1] + direction[1] * ROD_LENGTH,
        ];
        *rod = format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            origin[0], origin[1], end[0], end[1]
        );
        if state.session.tools.active_tool == "transform" {
            let scale_center = [
                origin[0] + direction[0] * 43.0,
                origin[1] + direction[1] * 43.0,
            ];
            let r = 5.0;
            *scale = format!(
                "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                scale_center[0] - r,
                scale_center[1] - r,
                scale_center[0] + r,
                scale_center[1] - r,
                scale_center[0] + r,
                scale_center[1] + r,
                scale_center[0] - r,
                scale_center[1] + r,
            );
            let tangent = if axis == glam::Vec3::X {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let bitangent = axis.cross(tangent);
            for segment in 0..=64 {
                let angle = segment as f32 * std::f32::consts::TAU / 64.0;
                let ring_direction =
                    screen_direction(tangent * angle.cos() + bitangent * angle.sin());
                rotate.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if segment == 0 { "M" } else { "L" },
                    origin[0] + ring_direction[0] * 91.0,
                    origin[1] + ring_direction[1] * 91.0
                ));
            }
        }
        if state.session.tools.active_tool == "scale" {
            let r = ARROW_HALF;
            *arrow = format!(
                "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                end[0] - r,
                end[1] - r,
                end[0] + r,
                end[1] - r,
                end[0] + r,
                end[1] + r,
                end[0] - r,
                end[1] + r
            );
            continue;
        }
        // Seta: ponta em `end`, base recuada ao longo da haste.
        let base = [
            end[0] - direction[0] * ARROW_BACK,
            end[1] - direction[1] * ARROW_BACK,
        ];
        let perpendicular = [-direction[1], direction[0]];
        let left = [
            base[0] + perpendicular[0] * ARROW_HALF,
            base[1] + perpendicular[1] * ARROW_HALF,
        ];
        let right = [
            base[0] - perpendicular[0] * ARROW_HALF,
            base[1] - perpendicular[1] * ARROW_HALF,
        ];
        *arrow = format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            end[0], end[1], left[0], left[1], right[0], right[1]
        );
    }
    model
}

/// Tracejado pontilhado do "cordão" entre a base da seleção (pivô) e o
/// mouse durante uma ferramenta de manipulação ativa (Move/Rotate/Scale por
/// arrasto ou modal de teclado, Extrude/Inset/Bevel paramétricos...).
/// O Path do Slint não tem dash nativo, então o padrão nasce aqui em Rust:
/// pontos de 2px com 4px de intervalo ao longo do segmento base→mouse.
/// A linha cresce/encolhe sozinha conforme o mouse se afasta/aproxima.
pub(crate) fn dotted_link_commands(from: [f32; 2], to: [f32; 2]) -> String {
    const DASH: f32 = 2.0;
    const GAP: f32 = 4.0;
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let length = dx.hypot(dy);
    if length < 0.5 {
        return String::new();
    }
    let (ux, uy) = (dx / length, dy / length);
    let mut commands = String::new();
    let mut cursor = 0.0;
    while cursor < length {
        let end = (cursor + DASH).min(length);
        commands.push_str(&format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            from[0] + ux * cursor,
            from[1] + uy * cursor,
            from[0] + ux * end,
            from[1] + uy * end,
        ));
        cursor = end + GAP;
    }
    commands
}

/// Active tool pivot-to-pointer or guide-line feedback cord (P3D-131).
/// Cordão pivô→mouse ou guia de telemetria da ferramenta ativa (P3D-131).
pub(crate) fn compute_drag_link(
    state: &AppState,
    width: f32,
    height: f32,
    pointer: [f32; 2],
    link_active: bool,
) -> String {
    if !link_active || width <= 1.0 || height <= 1.0 {
        return String::new();
    }
    let view_proj = state.session.camera.view_proj();
    let fb = state.current_tool_feedback();
    let pivot = fb
        .as_ref()
        .map(|f| f.origin)
        .unwrap_or_else(|| state.calculate_pivot(state.session.pivot_point));
    let clip = view_proj * pivot.extend(1.0);
    if clip.w <= 0.05 {
        return String::new();
    }
    let inv_w = 1.0 / clip.w;
    let base = [
        (clip.x * inv_w * 0.5 + 0.5) * width,
        (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
    ];

    let target = if let Some(fb) = fb
        && fb.is_snapped
        && (fb.current - fb.origin).length() > 1e-4
    {
        // When snapped to magnetic target with non-zero delta, project the 3D snapped coordinate
        // Quando atraído por alvo magnético com delta não-nulo, projeta a coordenada 3D sob snap
        let clip_t = view_proj * fb.current.extend(1.0);
        if clip_t.w > 0.05 {
            let inv_t = 1.0 / clip_t.w;
            [
                (clip_t.x * inv_t * 0.5 + 0.5) * width,
                (1.0 - (clip_t.y * inv_t * 0.5 + 0.5)) * height,
            ]
        } else {
            pointer
        }
    } else {
        pointer
    };

    dotted_link_commands(base, target)
}

/// Resumo legível da seleção (Object Info do Blender): o que está
/// selecionado e quanto. Linha única para a pill da viewport e o inspector.
pub(crate) fn format_selection_summary(state: &AppState) -> String {
    fn plural(count: usize, singular: &str, plural: &str) -> Option<String> {
        if count == 0 {
            None
        } else if count == 1 {
            Some(format!("1 {singular}"))
        } else {
            Some(format!("{count} {plural}"))
        }
    }
    if state.selection_domain() == SelectionDomain::Object {
        // Espelha a regra da UI: ativo conta como selecionado.
        let active_id = state.project.active().map(|a| a.id);
        let total = state
            .project
            .assets
            .iter()
            .filter(|a| state.session.selection.assets.contains(&a.id) || active_id == Some(a.id))
            .count();
        return plural(total, "object selected", "objects selected")
            .unwrap_or_else(|| "No selection".to_string());
    }
    let details = state.query_selection_details();
    let mut parts = Vec::new();
    if let Some(text) = plural(details.selected_verts_count, "point", "points") {
        parts.push(text);
    }
    if let Some(text) = plural(details.selected_edges_count, "edge", "edges") {
        parts.push(text);
    }
    if let Some(text) = plural(details.selected_faces_count, "face", "faces") {
        parts.push(text);
    }
    if parts.is_empty() {
        "No selection".to_string()
    } else {
        format!("Selected: {}", parts.join(" · "))
    }
}

pub(crate) fn compute_selection_overlay(
    state: &AppState,
    width: f32,
    height: f32,
    backend_draws_guides: bool,
) -> SelectionOverlayModel {
    if state.selection_domain() != SelectionDomain::Object {
        return compute_asset_overlay(
            state,
            width,
            height,
            backend_draws_guides,
            state.project.active,
        );
    }
    let mut overlay = SelectionOverlayModel::default();
    for (index, asset) in state.project.assets.iter().enumerate() {
        if !asset.visible {
            continue;
        }
        let is_active = index == state.project.active;
        let selected = state.session.selection.assets.contains(&asset.id) || is_active;
        let hovered = state.session.tools.hover == petunia_core::HoverTarget::Object(index);
        if !selected && !hovered {
            continue;
        }
        let part = compute_asset_overlay(state, width, height, backend_draws_guides, index);
        if is_active {
            overlay
                .active_outline_commands
                .push_str(&part.outline_commands);
        } else if selected {
            overlay.outline_commands.push_str(&part.outline_commands);
        } else {
            overlay
                .unselected_outline_commands
                .push_str(&part.outline_commands);
        }
    }
    overlay.visible = !overlay.outline_commands.is_empty()
        || !overlay.active_outline_commands.is_empty()
        || !overlay.unselected_outline_commands.is_empty();
    overlay
}

pub(crate) fn compute_asset_overlay(
    state: &AppState,
    width: f32,
    height: f32,
    backend_draws_guides: bool,
    index: usize,
) -> SelectionOverlayModel {
    /// Teto de segmentos por frame: malhas grandes não podem gerar uma string
    /// gigante a cada sync de propriedades.
    const MAX_SEGMENTS: usize = 4_000;

    if width <= 1.0 || height <= 1.0 {
        return SelectionOverlayModel::default();
    }
    if state.workspace == Workspace::Paint {
        return SelectionOverlayModel::default();
    }
    let Some(asset) = state.project.assets.get(index) else {
        return SelectionOverlayModel::default();
    };
    let evaluated = asset.evaluated_mesh();
    let mesh = &evaluated;
    if mesh.verts.is_empty() {
        return SelectionOverlayModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        Some([
            (clip.x * inv_w * 0.5 + 0.5) * width,
            (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
        ])
    };

    let mut outline = String::new();
    let mut points = String::new();
    let mut unselected_outline = String::new();
    let mut unselected_points = String::new();
    let mut segments = 0usize;
    let push_segment = |commands: &mut String, a: [f32; 2], b: [f32; 2]| {
        commands.push_str(&format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            a[0], a[1], b[0], b[1]
        ));
    };
    let push_disc = |commands: &mut String, center: [f32; 2], radius: f32| {
        use std::fmt::Write as _;
        for step in 0..12 {
            let angle = step as f32 * std::f32::consts::TAU / 12.0;
            let action = if step == 0 { 'M' } else { 'L' };
            let _ = write!(
                commands,
                "{action} {:.2} {:.2} ",
                center[0] + radius * angle.cos(),
                center[1] + radius * angle.sin()
            );
        }
        commands.push_str("Z ");
    };

    let domain = state.selection_domain();
    let mut truncated = false;

    match domain {
        SelectionDomain::Object => {
            // Silhueta: aresta entre face frontal e traseira (ou borda aberta
            // frontal). Não desenhar todas as arestas de faces frontais, o que
            // faria um objeto selecionado parecer uma caixa de arame gigante.
            let mut adjacent = std::collections::HashMap::<(u32, u32), (usize, usize)>::new();
            let eye = state.session.camera.eye();
            for (fi, face) in mesh.faces.iter().enumerate() {
                if face.verts.len() < 3 {
                    continue;
                }
                let face_center = face
                    .verts
                    .iter()
                    .filter_map(|&vi| mesh.verts.get(vi as usize))
                    .map(|vertex| vertex.vec())
                    .sum::<glam::Vec3>()
                    / face.verts.len() as f32;
                let front = mesh.face_normal(fi).dot(eye - face_center) > 0.0;
                for index in 0..face.verts.len() {
                    let a = face.verts[index];
                    let b = face.verts[(index + 1) % face.verts.len()];
                    let entry = adjacent.entry((a.min(b), a.max(b))).or_default();
                    if front {
                        entry.0 += 1;
                    } else {
                        entry.1 += 1;
                    }
                }
            }
            for ((a, b), (front, back)) in adjacent {
                if front == 0 || (back == 0 && front > 1) {
                    continue;
                }
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                if let (Some(pa), Some(pb)) = (project(va.vec()), project(vb.vec())) {
                    push_segment(&mut outline, pa, pb);
                    segments += 1;
                }
            }
        }
        SelectionDomain::Vertex => {
            if backend_draws_guides {
                return SelectionOverlayModel::default();
            }
            for (index, vertex) in mesh.verts.iter().enumerate() {
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let Some(sp) = project(vertex.vec()) else {
                    continue;
                };
                let hovered =
                    state.session.tools.hover == petunia_core::HoverTarget::Vertex(index as u32);
                let radius = if hovered {
                    (state.ui.selection_thickness * 2.5).clamp(6.5, 8.5)
                } else if vertex.selected {
                    (state.ui.selection_thickness * 2.0).clamp(5.0, 7.0)
                } else {
                    (state.ui.selection_thickness * 1.5).clamp(3.5, 5.5)
                };
                let target = if hovered || vertex.selected {
                    &mut points
                } else {
                    &mut unselected_points
                };
                push_disc(target, sp, radius);
                segments += 1;
            }
        }
        SelectionDomain::Edge => {
            if backend_draws_guides {
                return SelectionOverlayModel::default();
            }
            for (a, b) in mesh.edges_unique() {
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let selected =
                    mesh.selected_edges.contains(&(a, b)) || mesh.selected_edges.contains(&(b, a));
                if selected {
                    // O GPU desenha a aresta selecionada com depth test.
                    continue;
                }
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                if let (Some(pa), Some(pb)) = (project(va.vec()), project(vb.vec())) {
                    push_segment(&mut unselected_outline, pa, pb);
                    segments += 1;
                }
            }
        }
        SelectionDomain::Face => {
            // A seleção e o hover de faces são preenchidos pelo renderer.
            // Marcadores centrais em todas as faces poluíam a malha.
        }
    }

    let _ = truncated;
    let visible = !outline.is_empty()
        || !points.is_empty()
        || !unselected_outline.is_empty()
        || !unselected_points.is_empty();
    SelectionOverlayModel {
        visible,
        outline_commands: outline,
        // O roteamento ativo × selecionado acontece no chamador
        // (compute_selection_overlay); aqui nasce sempre vazio.
        active_outline_commands: String::new(),
        point_commands: points,
        unselected_outline_commands: unselected_outline,
        unselected_point_commands: unselected_points,
        accent: domain != SelectionDomain::Object,
    }
}

pub(crate) fn pick_face_hit(
    state: &AppState,
    origin: glam::Vec3,
    direction: glam::Vec3,
) -> Option<(usize, glam::Vec3)> {
    let mesh = state.project.active_mesh()?;
    let mut best = None;
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for corners in mesh.face_triangle_corners(face_index) {
            let [p0, p1, p2] = corners.map(|i| mesh.verts[face.verts[i] as usize].vec());
            if let Some(distance) = ray_triangle(origin, direction, p0, p1, p2)
                && best.is_none_or(|(_, current)| distance < current)
            {
                best = Some((face_index, distance));
            }
        }
    }
    best.map(|(face, distance)| (face, origin + direction * distance))
}

pub(crate) fn ray_triangle(
    origin: glam::Vec3,
    direction: glam::Vec3,
    p0: glam::Vec3,
    p1: glam::Vec3,
    p2: glam::Vec3,
) -> Option<f32> {
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let pvec = direction.cross(edge2);
    let det = edge1.dot(pvec);
    if det.abs() < 1e-7 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = origin - p0;
    let u = tvec.dot(pvec) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(edge1);
    let v = direction.dot(qvec) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(qvec) * inv;
    (distance > 1e-4).then_some(distance)
}
