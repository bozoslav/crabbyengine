//! Punctual lights and retained, conservative CPU froxel lists.
use super::{Camera3D, Transform3D, linear};
use crate::{Color, World};
use glam::{Mat4, Vec3};

/// Omnidirectional light at its entity's [`Transform3D::position`]. Scale is ignored.
/// Intensity is linear radiant intensity; distance attenuation is inverse-square,
/// softened within one world unit and smoothly windowed to zero at `range`.
/// Invalid/nonpositive ranges or intensities disable the light.
/// ```
/// use crabbyengine::*;
/// let lamp = PointLight3D { intensity: 20.0, range: 12.0, ..Default::default() };
/// let mut world = World::new();
/// world.spawn((Transform3D::at(vec3(0.0, 3.0, 0.0)), lamp));
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PointLight3D {
    /// sRGB color (alpha ignored).
    pub color: Color,
    /// Linear radiant intensity. Default 1.
    pub intensity: f32,
    /// Reach in world units. Default 10; unaffected by transform scale.
    pub range: f32,
}
impl Default for PointLight3D {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            range: 10.0,
        }
    }
}

/// Cone light pointing along its entity's rotated local **-Z** (its forward).
/// Angles are cone **half angles in radians**, `0 <= inner <= outer < pi/2`.
/// Equal angles give a hard cone edge. Invalid angles disable the light.
/// Position, intensity, range and scale follow [`PointLight3D`]. No shadows yet.
/// ```
/// use crabbyengine::*;
/// let beam = SpotLight3D { inner_angle: 0.2, outer_angle: 0.5, ..Default::default() };
/// let mut world = World::new();
/// world.spawn((Transform3D::default(), beam));
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SpotLight3D {
    /// sRGB color (alpha ignored).
    pub color: Color,
    /// Linear radiant intensity. Default 1.
    pub intensity: f32,
    /// Reach in world units. Default 10.
    pub range: f32,
    /// Fully lit half angle in radians. Default 0.3.
    pub inner_angle: f32,
    /// Unlit beyond this half angle, radians. Default 0.5.
    pub outer_angle: f32,
}
impl Default for SpotLight3D {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            range: 10.0,
            inner_angle: 0.3,
            outer_angle: 0.5,
        }
    }
}

pub(crate) const GRID: [usize; 3] = [16, 9, 24];
const CELLS: usize = GRID[0] * GRID[1] * GRID[2];
pub(crate) const LIST_WIDTH: usize = 1024;

/// Four RGBA32Float texels, one row per light.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuLight {
    pub position_range: [f32; 4],
    pub color: [f32; 4],
    pub direction_outer: [f32; 4],
    pub inner: [f32; 4],
}

#[derive(Default)]
pub(crate) struct LightGrid {
    pub lights: Vec<GpuLight>,
    /// R32Uint texels: (offset, count) for each cluster, then packed indices.
    pub lists: Vec<u32>,
    /// near, inverse log depth span times slice count, padding.
    pub depth: [f32; 4],
    bounds: Vec<([usize; 3], [usize; 3])>,
    counts: Vec<u32>,
    cursors: Vec<u32>,
}

impl LightGrid {
    pub fn collect(&mut self, world: &mut World, camera: &Camera3D) {
        self.lights.clear();
        self.bounds.clear();
        let near = camera.near.max(f32::MIN_POSITIVE);
        let far = camera.far.max(near * 1.001);
        self.depth = [near, GRID[2] as f32 / (far / near).ln(), 0.0, 0.0];
        let view = camera.view();
        let proj = camera.view_proj() * view.inverse();
        for (t, p) in world.query::<(&Transform3D, &PointLight3D)>().iter() {
            self.add(t, *p, None, view, proj, far);
        }
        for (t, s) in world.query::<(&Transform3D, &SpotLight3D)>().iter() {
            if !s.inner_angle.is_finite()
                || !s.outer_angle.is_finite()
                || s.inner_angle < 0.0
                || s.outer_angle < s.inner_angle
                || s.outer_angle >= std::f32::consts::FRAC_PI_2
            {
                continue;
            }
            self.add(
                t,
                PointLight3D {
                    color: s.color,
                    intensity: s.intensity,
                    range: s.range,
                },
                Some(*s),
                view,
                proj,
                far,
            );
        }
        self.counts.resize(CELLS, 0);
        self.counts.fill(0);
        for &(lo, hi) in &self.bounds {
            visit(lo, hi, |c| self.counts[c] += 1);
        }
        let total: usize = self.counts.iter().map(|&n| n as usize).sum();
        let used = CELLS * 2 + total;
        self.lists.resize(used.div_ceil(LIST_WIDTH) * LIST_WIDTH, 0);
        self.cursors.resize(CELLS, 0);
        let mut offset = CELLS as u32 * 2;
        for c in 0..CELLS {
            self.lists[c * 2] = offset;
            self.lists[c * 2 + 1] = self.counts[c];
            self.cursors[c] = offset;
            offset += self.counts[c];
        }
        for (id, &(lo, hi)) in self.bounds.iter().enumerate() {
            visit(lo, hi, |c| {
                self.lists[self.cursors[c] as usize] = id as u32;
                self.cursors[c] += 1;
            });
        }
    }

    fn add(&mut self, t: &Transform3D, p: PointLight3D, spot: Option<SpotLight3D>, view: Mat4, proj: Mat4, far: f32) {
        if !t.position.is_finite()
            || !p.range.is_finite()
            || p.range <= 0.0
            || !p.intensity.is_finite()
            || p.intensity <= 0.0
        {
            return;
        }
        let rgba = linear(p.color);
        if !rgba[..3].iter().all(|v| v.is_finite() && *v >= 0.0) {
            return;
        }
        let center = view.transform_point3(t.position);
        let z = -center.z;
        let r = p.range;
        let near = self.depth[0];
        if z + r < near || z - r > far {
            return;
        }
        let mut lo = [0; 3];
        let mut hi = [GRID[0] - 1, GRID[1] - 1, GRID[2] - 1];
        // Project an enclosing view-space box. If it crosses the near plane,
        // cover all XY tiles: dividing by a behind-camera corner would miss lights.
        if z - r > near {
            for axis in 0..2 {
                let scale = if axis == 0 { proj.x_axis.x } else { proj.y_axis.y };
                let a = (center[axis] - r) * scale;
                let b = (center[axis] + r) * scale;
                let min = (a / (z - r)).min(a / (z + r));
                let max = (b / (z - r)).max(b / (z + r));
                if min > 1.0 || max < -1.0 {
                    return;
                }
                let tile = |v: f32| {
                    (((v * 0.5 + 0.5) * GRID[axis] as f32).floor() as i32).clamp(0, GRID[axis] as i32 - 1) as usize
                };
                lo[axis] = tile(min);
                hi[axis] = tile(max);
            }
        }
        let slice = |d: f32| {
            ((d.max(near) / near).ln() * self.depth[1])
                .floor()
                .clamp(0.0, (GRID[2] - 1) as f32) as usize
        };
        lo[2] = slice(z - r);
        hi[2] = slice(z + r);
        let direction = t.forward().try_normalize().unwrap_or(Vec3::NEG_Z);
        if spot.is_some() && (!t.rotation.is_finite() || t.rotation.length_squared() < 1e-8) {
            return;
        }
        let outer = spot.map_or(-2.0, |s| s.outer_angle.cos());
        self.lights.push(GpuLight {
            position_range: t.position.extend(r).to_array(),
            color: [rgba[0] * p.intensity, rgba[1] * p.intensity, rgba[2] * p.intensity, 0.0],
            direction_outer: direction.extend(outer).to_array(),
            inner: [spot.map_or(1.0, |s| s.inner_angle.cos()), 0.0, 0.0, 0.0],
        });
        self.bounds.push((lo, hi));
    }
}
fn visit(lo: [usize; 3], hi: [usize; 3], mut f: impl FnMut(usize)) {
    for z in lo[2]..=hi[2] {
        for y in lo[1]..=hi[1] {
            for x in lo[0]..=hi[0] {
                f((z * GRID[1] + y) * GRID[0] + x);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Counting;
    thread_local! { static ALLOCS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) }; }
    // SAFETY: delegates every allocation unchanged to the system allocator.
    unsafe impl std::alloc::GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
            let _ = ALLOCS.try_with(|n| {
                if let Some(v) = n.get() {
                    n.set(Some(v + 1));
                }
            });
            // SAFETY: forwarded allocation contract.
            unsafe { std::alloc::System.alloc(layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
            let _ = ALLOCS.try_with(|n| {
                if let Some(v) = n.get() {
                    n.set(Some(v + 1));
                }
            });
            // SAFETY: forwarded allocation contract.
            unsafe { std::alloc::System.realloc(ptr, layout, size) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
            // SAFETY: forwarded allocation contract.
            unsafe { std::alloc::System.dealloc(ptr, layout) }
        }
    }
    #[global_allocator]
    static ALLOCATOR: Counting = Counting;

    fn camera() -> Camera3D {
        let mut c = Camera3D::new(glam::Vec2::new(800.0, 600.0));
        c.position = Vec3::ZERO;
        c.rotation = glam::Quat::IDENTITY;
        c.far = 100.0;
        c
    }
    #[test]
    fn culls_outside_and_keeps_near_crossing_lights() {
        let mut w = World::new();
        for at in [
            Vec3::new(0.0, 0.0, -5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 30.0),
            Vec3::new(100.0, 0.0, -5.0),
            Vec3::new(0.0, 0.0, -150.0),
        ] {
            w.spawn((
                Transform3D::at(at),
                PointLight3D {
                    range: 2.0,
                    ..Default::default()
                },
            ));
        }
        let mut g = LightGrid::default();
        g.collect(&mut w, &camera());
        assert_eq!(g.lights.len(), 2);
        assert_eq!(g.bounds[1].0[..2], [0, 0]);
        assert_eq!(g.bounds[1].1[..2], [15, 8]);
    }
    #[test]
    fn overlapping_lights_are_not_truncated_and_capacity_is_retained() {
        let mut w = World::new();
        for _ in 0..1000 {
            w.spawn((Transform3D::default(), PointLight3D::default()));
        }
        let mut g = LightGrid::default();
        g.collect(&mut w, &camera());
        assert_eq!(g.lists[1], 1000);
        let ptrs = (
            g.lights.as_ptr(),
            g.lists.as_ptr(),
            g.bounds.as_ptr(),
            g.counts.as_ptr(),
            g.cursors.as_ptr(),
        );
        ALLOCS.with(|n| n.set(Some(0)));
        for _ in 0..20 {
            g.collect(&mut w, &camera());
        }
        let allocations = ALLOCS.with(|n| n.replace(None).unwrap());
        assert_eq!(allocations, 0, "light collection must not allocate once warm");
        g.collect(&mut w, &camera());
        assert_eq!(
            ptrs,
            (
                g.lights.as_ptr(),
                g.lists.as_ptr(),
                g.bounds.as_ptr(),
                g.counts.as_ptr(),
                g.cursors.as_ptr()
            )
        );
    }
    #[test]
    fn tiny_camera_near_plane_keeps_tiny_near_lights() {
        let mut c = camera();
        c.near = 1e-6;
        let mut w = World::new();
        w.spawn((
            Transform3D::at(Vec3::new(0.0, 0.0, -2e-5)),
            PointLight3D {
                range: 1e-5,
                ..Default::default()
            },
        ));
        let mut g = LightGrid::default();
        g.collect(&mut w, &c);
        assert_eq!(g.lights.len(), 1);
    }

    #[test]
    fn every_lit_visible_sample_has_its_light_in_the_cluster() {
        let c = camera();
        let mut w = World::new();
        for i in 0..80 {
            let position = Vec3::new((i % 9) as f32 * 3.1 - 12.0, (i % 5) as f32 - 2.0, -(i as f32 * 1.1));
            w.spawn((
                Transform3D::at(position),
                PointLight3D {
                    range: 4.0,
                    ..Default::default()
                },
            ));
        }
        let mut g = LightGrid::default();
        g.collect(&mut w, &c);
        for (id, light) in g.lights.iter().enumerate() {
            let center = Vec3::from_slice(&light.position_range);
            for dx in [-2.0, 0.0, 2.0] {
                for dy in [-2.0, 0.0, 2.0] {
                    for dz in [-2.0, 0.0, 2.0] {
                        let p = center + Vec3::new(dx, dy, dz);
                        let clip = c.view_proj() * p.extend(1.0);
                        let ndc = clip.truncate() / clip.w;
                        if clip.w <= 0.0 || ndc.x.abs() >= 1.0 || ndc.y.abs() >= 1.0 || ndc.z < 0.0 || ndc.z >= 1.0 {
                            continue;
                        }
                        let x = ((ndc.x * 0.5 + 0.5) * GRID[0] as f32) as usize;
                        let y = ((ndc.y * 0.5 + 0.5) * GRID[1] as f32) as usize;
                        let z = (((clip.w / g.depth[0]).ln() * g.depth[1]) as usize).min(GRID[2] - 1);
                        let cell = (z * GRID[1] + y) * GRID[0] + x;
                        let offset = g.lists[cell * 2] as usize;
                        let count = g.lists[cell * 2 + 1] as usize;
                        assert!(
                            g.lists[offset..offset + count].contains(&(id as u32)),
                            "light {id}, sample {p:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_lights_are_disabled_and_spots_rotate() {
        let mut w = World::new();
        for range in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            w.spawn((
                Transform3D::default(),
                PointLight3D {
                    range,
                    ..Default::default()
                },
            ));
        }
        w.spawn((
            Transform3D::default(),
            SpotLight3D {
                inner_angle: 1.0,
                outer_angle: 0.5,
                ..Default::default()
            },
        ));
        w.spawn((
            Transform3D::default().rotated(glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            SpotLight3D::default(),
        ));
        let mut g = LightGrid::default();
        g.collect(&mut w, &camera());
        assert_eq!(g.lights.len(), 1);
        assert!((Vec3::from_slice(&g.lights[0].direction_outer) - Vec3::NEG_X).length() < 1e-5);
    }
}
