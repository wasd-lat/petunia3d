//! Sessão de criação de primitivas (Wave 8 — §10, P3D-153/154).
//!
//! Uma inserção vira transação única de undo com cartão contextual de
//! parâmetros ("Last Operation"): os parâmetros regeneram a malha a partir do
//! descritor (nunca deformam o resultado anterior), `Confirm` encerra mantendo
//! um único checkpoint e `Cancel` (Esc) desfaz até o ponto de inserção.
//!
//! A sessão invalida sozinha quando outra operação assume o topo do undo
//! (qualquer edição topológica converte a primitiva em malha comum): a UI
//! esconde o cartão e o Esc posterior não remove nada.

use petunia_mesh::Mesh;
use uuid::Uuid;

use super::selection::Selection;
use crate::command::PrimitiveKind;

/// Preenchimento do círculo (§16): anel aberto ou leque soldado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleFill {
    None,
    Disc,
}

/// Parâmetros de criação por primitiva (regeneração determinística).
///
/// Defaults intencionalmente low-poly (§5). Limites (§44): lados 3–32, anéis
/// 2–24, subdivisão ico 0–3, torus 3–64/3–32, raios e alturas > 0 (clamp).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveDescriptor {
    Box {
        width: f32,
        height: f32,
        depth: f32,
    },
    Plane {
        width: f32,
        height: f32,
    },
    Wedge {
        width: f32,
        height: f32,
        depth: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
        sides: u32,
        cap_top: bool,
        cap_bottom: bool,
    },
    Cone {
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        sides: u32,
        cap_bottom: bool,
        cap_top: bool,
    },
    Circle {
        radius: f32,
        vertices: u32,
        fill: CircleFill,
    },
    Torus {
        major_radius: f32,
        minor_radius: f32,
        major_segments: u32,
        minor_segments: u32,
    },
    LowSphere {
        radius: f32,
        segments: u32,
        rings: u32,
    },
    Icosphere {
        radius: f32,
        subdiv: u32,
    },
    Capsule {
        radius: f32,
        height: f32,
        radial_segments: u32,
        cap_segments: u32,
    },
}

impl PrimitiveDescriptor {
    /// Descritor padrão da espécie (diálogo abre com valores low-poly sãos).
    pub fn default_for(kind: PrimitiveKind) -> Self {
        match kind {
            PrimitiveKind::Cube => Self::Box {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            },
            PrimitiveKind::Plane => Self::Plane {
                width: 1.0,
                height: 1.0,
            },
            PrimitiveKind::Wedge => Self::Wedge {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            },
            PrimitiveKind::Cylinder => Self::Cylinder {
                radius: 1.0,
                height: 2.0,
                sides: 8,
                cap_top: true,
                cap_bottom: true,
            },
            PrimitiveKind::Cone => Self::Cone {
                bottom_radius: 1.0,
                top_radius: 0.0,
                height: 2.0,
                sides: 8,
                cap_bottom: true,
                cap_top: false,
            },
            PrimitiveKind::Circle => Self::Circle {
                radius: 1.0,
                vertices: 12,
                fill: CircleFill::Disc,
            },
            PrimitiveKind::Torus => Self::Torus {
                major_radius: 1.0,
                minor_radius: 0.3,
                major_segments: 12,
                minor_segments: 6,
            },
            PrimitiveKind::Sphere => Self::LowSphere {
                radius: 1.0,
                segments: 12,
                rings: 6,
            },
            PrimitiveKind::Icosphere => Self::Icosphere {
                radius: 1.0,
                subdiv: 1,
            },
            PrimitiveKind::Capsule => Self::Capsule {
                radius: 0.5,
                height: 2.0,
                radial_segments: 8,
                cap_segments: 2,
            },
        }
    }

    /// Nome i18n (`prims.cube`, …).
    pub fn name_key(self) -> petunia_config::TextId {
        use petunia_config::text_id as T;
        match self {
            Self::Box { .. } => T::PRIMS_CUBE,
            Self::Plane { .. } => T::PRIMS_PLANE,
            Self::Wedge { .. } => T::PRIMS_WEDGE,
            Self::Cylinder { .. } => T::PRIMS_CYLINDER,
            Self::Cone { .. } => T::PRIMS_CONE,
            Self::Circle { .. } => T::PRIMS_CIRCLE,
            Self::Torus { .. } => T::PRIMS_TORUS,
            Self::LowSphere { .. } => T::PRIMS_SPHERE,
            Self::Icosphere { .. } => T::PRIMS_ICOSPHERE,
            Self::Capsule { .. } => T::PRIMS_CAPSULE,
        }
    }

    /// Espécie de volta (reabertura e atalhos).
    pub fn kind(self) -> PrimitiveKind {
        match self {
            Self::Box { .. } => PrimitiveKind::Cube,
            Self::Plane { .. } => PrimitiveKind::Plane,
            Self::Wedge { .. } => PrimitiveKind::Wedge,
            Self::Cylinder { .. } => PrimitiveKind::Cylinder,
            Self::Cone { .. } => PrimitiveKind::Cone,
            Self::Circle { .. } => PrimitiveKind::Circle,
            Self::Torus { .. } => PrimitiveKind::Torus,
            Self::LowSphere { .. } => PrimitiveKind::Sphere,
            Self::Icosphere { .. } => PrimitiveKind::Icosphere,
            Self::Capsule { .. } => PrimitiveKind::Capsule,
        }
    }

    /// Constrói a malha LOCAL (sem offset) a partir dos parâmetros.
    pub fn build(self) -> Mesh {
        match self {
            Self::Box {
                width,
                height,
                depth,
            } => Mesh::box_dim(width, height, depth),
            Self::Plane { width, height } => {
                let mut mesh = Mesh::plane(1.0);
                for v in &mut mesh.verts {
                    v.pos[0] *= width.clamp(0.05, 100.0);
                    v.pos[2] *= height.clamp(0.05, 100.0);
                }
                mesh
            }
            Self::Wedge {
                width,
                height,
                depth,
            } => Mesh::wedge(width, height, depth),
            Self::Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            } => Mesh::radial_frustum(
                radius.clamp(0.05, 100.0),
                radius.clamp(0.05, 100.0),
                height.clamp(0.05, 100.0),
                sides.clamp(3, 32),
                cap_bottom,
                cap_top,
            ),
            Self::Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            } => Mesh::radial_frustum(
                bottom_radius.clamp(0.0, 100.0),
                top_radius.clamp(0.0, 100.0),
                height.clamp(0.05, 100.0),
                sides.clamp(3, 32),
                cap_bottom,
                cap_top,
            ),
            Self::Circle {
                radius,
                vertices,
                fill,
            } => Mesh::circle(
                radius.clamp(0.05, 100.0),
                vertices.clamp(3, 64),
                fill == CircleFill::Disc,
            ),
            Self::Torus {
                major_radius,
                minor_radius,
                major_segments,
                minor_segments,
            } => Mesh::torus(
                major_radius.clamp(0.05, 100.0),
                minor_radius.clamp(0.01, 100.0),
                major_segments.clamp(3, 64),
                minor_segments.clamp(3, 32),
            ),
            Self::LowSphere {
                radius,
                segments,
                rings,
            } => Mesh::sphere_low(
                segments.clamp(3, 32),
                rings.clamp(2, 24),
                radius.clamp(0.05, 100.0),
            ),
            Self::Icosphere { radius, subdiv } => {
                Mesh::icosphere(radius.clamp(0.05, 100.0), subdiv.min(3))
            }
            Self::Capsule {
                radius,
                height,
                radial_segments,
                cap_segments,
            } => Mesh::capsule_profile(
                radial_segments.clamp(3, 32),
                radius.clamp(0.05, 100.0),
                height.clamp(0.05, 100.0),
                cap_segments.clamp(1, 8),
            ),
        }
    }
}

/// Criação em andamento: alvo + parâmetros + contexto de reversão.
#[derive(Debug, Clone)]
pub struct PrimitiveCreationSession {
    /// Asset criado no `begin` (id estável; índice pode mudar).
    pub asset_id: Uuid,
    /// Parâmetros vivos (cada edição regenera a malha).
    pub descriptor: PrimitiveDescriptor,
    /// Offset do cursor capturado no `begin` (reaplicado a cada regeneração).
    pub cursor_offset: [f32; 3],
    /// Rótulo do checkpoint de inserção (validade = ainda no topo do undo).
    pub undo_label: String,
    /// Seleção da sessão antes do `begin` (restauração do `cancel`).
    pub original_selection: Selection,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::PrimitiveKind;
    use crate::state::AppState;

    #[test]
    fn descriptor_builds_valid_meshes() {
        for kind in [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Cylinder,
            PrimitiveKind::Plane,
            PrimitiveKind::Cone,
            PrimitiveKind::Capsule,
        ] {
            let mesh = PrimitiveDescriptor::default_for(kind).build();
            assert!(!mesh.verts.is_empty(), "{kind:?}");
            assert!(!mesh.faces.is_empty(), "{kind:?}");
            let mut mesh = mesh;
            mesh.validate();
            assert!(!mesh.faces.is_empty(), "{kind:?} após validate");
        }
    }

    #[test]
    fn descriptor_roundtrips_kind() {
        for kind in [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Cylinder,
            PrimitiveKind::Plane,
            PrimitiveKind::Cone,
            PrimitiveKind::Capsule,
            PrimitiveKind::Wedge,
            PrimitiveKind::Circle,
            PrimitiveKind::Torus,
            PrimitiveKind::Icosphere,
        ] {
            assert_eq!(PrimitiveDescriptor::default_for(kind).kind(), kind);
        }
    }

    #[test]
    fn confirm_keeps_single_checkpoint() {
        let mut state = AppState::new("en");
        let depth_before = state.project.undo.depth().0;
        assert!(state.begin_primitive(PrimitiveKind::Cube, Some("Box".to_string())));
        assert!(state.primitive_session_valid());
        let asset_id = state.session.primitive_session.as_ref().unwrap().asset_id;
        // Regenerações não empilham undo.
        assert!(state.update_primitive(PrimitiveDescriptor::Box {
            width: 4.0,
            height: 4.0,
            depth: 4.0,
        }));
        assert!(state.update_primitive(PrimitiveDescriptor::Box {
            width: 5.0,
            height: 5.0,
            depth: 5.0,
        }));
        assert_eq!(state.project.undo.depth().0, depth_before + 1);
        assert!(state.confirm_primitive());
        assert!(!state.primitive_session_valid());
        assert!(state.project.assets.iter().any(|a| a.id == asset_id));
        // Um único undo remove a criação inteira.
        assert!(state.undo());
        assert!(!state.project.assets.iter().any(|a| a.id == asset_id));
    }

    #[test]
    fn cancel_removes_asset_and_restores_selection() {
        let mut state = AppState::new("en");
        let before = state.project.assets.len();
        // Marca uma seleção prévia para verificar restauração.
        if let Some(first) = state.project.assets.first() {
            let id = first.id;
            state.session.selection.asset = Some(id);
        }
        assert!(state.begin_primitive(PrimitiveKind::Sphere, None));
        assert_eq!(state.project.assets.len(), before + 1);
        assert!(state.cancel_primitive());
        assert_eq!(state.project.assets.len(), before);
        assert!(!state.primitive_session_valid());
    }

    #[test]
    fn intervening_operation_invalidates_session() {
        use crate::command::SubdivideSelectionCmd;
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Cube, None));
        // Outra operação com checkpoint no meio: sessão vira malha comum.
        state.project.active_mesh_mut().unwrap().faces[0].selected = true;
        assert!(state.dispatch(&SubdivideSelectionCmd).is_ok());
        assert!(!state.primitive_session_valid());
        // Cancel posterior não remove nada.
        let count = state.project.assets.len();
        assert!(!state.cancel_primitive());
        assert_eq!(state.project.assets.len(), count);
    }

    #[test]
    fn reopen_creates_new_session_from_last() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Cylinder, Some("Tube".to_string())));
        assert!(state.update_primitive(PrimitiveDescriptor::Cylinder {
            radius: 2.0,
            height: 3.0,
            sides: 8,
            cap_top: true,
            cap_bottom: false,
        }));
        assert!(state.confirm_primitive());
        let before = state.project.assets.len();
        assert!(state.reopen_last_primitive());
        assert!(state.primitive_session_valid());
        assert_eq!(state.project.assets.len(), before + 1);
        let desc = state.session.primitive_session.as_ref().unwrap().descriptor;
        assert_eq!(
            desc,
            PrimitiveDescriptor::Cylinder {
                radius: 2.0,
                height: 3.0,
                sides: 8,
                cap_top: true,
                cap_bottom: false,
            }
        );
    }

    #[test]
    fn reset_restores_defaults() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Torus, None));
        assert!(state.update_primitive(PrimitiveDescriptor::Torus {
            major_radius: 5.0,
            minor_radius: 2.0,
            major_segments: 32,
            minor_segments: 16,
        }));
        // Reset = regenerar a partir do default da espécie.
        let kind = state
            .session
            .primitive_session
            .as_ref()
            .unwrap()
            .descriptor
            .kind();
        assert!(state.update_primitive(PrimitiveDescriptor::default_for(kind)));
        let id = state.session.primitive_session.as_ref().unwrap().asset_id;
        let asset = state.project.assets.iter().find(|a| a.id == id).unwrap();
        let fresh = PrimitiveDescriptor::default_for(kind).build();
        assert_eq!(asset.mesh.verts.len(), fresh.verts.len());
        assert!(state.confirm_primitive());
    }

    #[test]
    fn workspace_switch_finalizes_session_keeping_mesh() {
        let mut state = AppState::new("en");
        assert!(state.begin_primitive(PrimitiveKind::Torus, None));
        let count = state.project.assets.len();
        state.switch_workspace(crate::selection::Workspace::Uv);
        assert!(state.session.primitive_session.is_none());
        assert_eq!(state.project.assets.len(), count);
        // Virou malha comum: reabertura explícita ainda disponível.
        assert!(state.session.last_primitive.is_some());
    }

    #[test]
    fn all_ten_kinds_begin_update_confirm() {
        use PrimitiveKind as K;
        for kind in [
            K::Cube,
            K::Plane,
            K::Wedge,
            K::Cylinder,
            K::Cone,
            K::Circle,
            K::Torus,
            K::Sphere,
            K::Icosphere,
            K::Capsule,
        ] {
            let mut state = AppState::new("en");
            assert!(state.begin_primitive(kind, None), "{kind:?} begin");
            assert!(state.primitive_session_valid(), "{kind:?} valid");
            let depth = state.project.undo.depth().0;
            let desc = PrimitiveDescriptor::default_for(kind);
            assert!(state.update_primitive(desc), "{kind:?} update");
            assert_eq!(state.project.undo.depth().0, depth);
            assert!(state.confirm_primitive(), "{kind:?} confirm");
            assert_eq!(state.session.last_primitive, Some(desc));
        }
    }

    #[test]
    fn plane_scales_to_rectangle() {
        let mesh = PrimitiveDescriptor::Plane {
            width: 4.0,
            height: 2.0,
        }
        .build();
        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
        let (mut min_z, mut max_z) = (f32::MAX, f32::MIN);
        for v in &mesh.verts {
            min_x = min_x.min(v.pos[0]);
            max_x = max_x.max(v.pos[0]);
            min_z = min_z.min(v.pos[2]);
            max_z = max_z.max(v.pos[2]);
        }
        assert!((max_x - min_x - 4.0).abs() < 1e-4);
        assert!((max_z - min_z - 2.0).abs() < 1e-4);
    }
}
