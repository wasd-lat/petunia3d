//! Pointer projection for modal transforms. All results are measured from the
//! immutable operation pivot, in world units, regardless of zoom or projection.

use glam::{Vec2, Vec3};

use crate::Camera;

pub fn project_pixel(camera: &Camera, viewport: Vec2, point: Vec3) -> Option<Vec2> {
    let clip = camera.view_proj() * point.extend(1.0);
    if !clip.is_finite() || clip.w <= 0.0 || viewport.min_element() <= 0.0 {
        return None;
    }
    Some(Vec2::new(clip.x / clip.w + 1.0, 1.0 - clip.y / clip.w) * viewport * 0.5)
}

pub fn plane_point(camera: &Camera, viewport: Vec2, pixel: Vec2, pivot: Vec3, normal: Vec3) -> Option<Vec3> {
    if !viewport.is_finite() || viewport.min_element() <= 0.0 || !pixel.is_finite() { return None; }
    let ndc = pixel / viewport * 2.0 - Vec2::ONE;
    let (origin, direction) = camera.ray(ndc.x, -ndc.y);
    let denominator = direction.dot(normal);
    if denominator.abs() < 1.0e-4 { return None; }
    let distance = (pivot - origin).dot(normal) / denominator;
    let point = origin + direction * distance;
    (distance >= 0.0 && point.is_finite()).then_some(point)
}

pub fn plane_delta(camera: &Camera, viewport: Vec2, start: Vec2, current: Vec2, pivot: Vec3, normal: Vec3) -> Option<Vec3> {
    Some(plane_point(camera, viewport, current, pivot, normal)? - plane_point(camera, viewport, start, pivot, normal)?)
}

pub fn axis_delta(camera: &Camera, viewport: Vec2, start: Vec2, current: Vec2, pivot: Vec3, axis: Vec3) -> Option<f32> {
    // The plane contains the axis and faces the camera as much as possible.
    let normal = (camera.forward() - axis * camera.forward().dot(axis)).normalize_or_zero();
    if normal.length_squared() < 0.5 { return None; }
    plane_delta(camera, viewport, start, current, pivot, normal).map(|delta| delta.dot(axis))
}

/// Signed angle in degrees, wrapped to [-180, 180]. The caller unwraps successive
/// values to support multiple complete turns without losing precision.
pub fn rotation_angle(camera: &Camera, viewport: Vec2, start: Vec2, current: Vec2, pivot: Vec3, normal: Vec3) -> Option<f32> {
    let a = (plane_point(camera, viewport, start, pivot, normal)? - pivot).normalize_or_zero();
    let b = (plane_point(camera, viewport, current, pivot, normal)? - pivot).normalize_or_zero();
    if a.length_squared() < 0.5 || b.length_squared() < 0.5 { return None; }
    Some(normal.dot(a.cross(b)).atan2(a.dot(b)).to_degrees())
}
