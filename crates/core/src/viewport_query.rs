//! Shared viewport query contract for picking, hover, screen-space paint and decals.
//! CPU fallback; GPU query buffers can fill the same DTO later.

use glam::Vec3;
use uuid::Uuid;

/// Per-sample hit from a viewport query (pixel or ray).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportQuerySample {
    pub object_id: Uuid,
    pub face_index: usize,
    pub depth: f32,
    pub uv: [f32; 2],
    pub world: Vec3,
    pub normal: Vec3,
}

/// Batch of samples covering a dab / pick / hover.
#[derive(Clone, Debug, Default)]
pub struct ViewportQueryBuffer {
    pub samples: Vec<ViewportQuerySample>,
}

impl ViewportQueryBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, sample: ViewportQuerySample) {
        self.samples.push(sample);
    }

    pub fn nearest(&self) -> Option<&ViewportQuerySample> {
        self.samples.iter().min_by(|a, b| a.depth.total_cmp(&b.depth))
    }
}

/// Surface attachment for persistent decals. Topology edits must revalidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentValidity {
    Valid,
    NeedsReprojection,
    Invalid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAttachment {
    pub object_id: Uuid,
    pub face_index: usize,
    pub barycentric: [f32; 3],
    pub topology_revision: u64,
}

impl SurfaceAttachment {
    pub fn validate(&self, object_id: Uuid, face_count: usize, topology_revision: u64) -> AttachmentValidity {
        if self.object_id != object_id {
            return AttachmentValidity::Invalid;
        }
        if self.face_index >= face_count {
            return AttachmentValidity::Invalid;
        }
        if self.topology_revision != topology_revision {
            AttachmentValidity::NeedsReprojection
        } else {
            AttachmentValidity::Valid
        }
    }
}
