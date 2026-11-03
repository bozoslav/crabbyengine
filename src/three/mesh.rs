//! [`MeshData`]: triangles on the CPU, before they go to the GPU.

use glam::{Vec2, Vec3};

/// Triangles to make a [`Mesh`](crate::Mesh) from: corners with their
/// normals and texture coordinates, and three indices per triangle
/// (counter-clockwise seen from the front).
///
/// ```
/// # use crabbyengine::*;
/// // One triangle facing +Z.
/// let tri = MeshData {
///     positions: vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)],
///     normals: vec![Vec3::Z; 3],
///     uvs: vec![vec2(0.0, 1.0), vec2(1.0, 1.0), vec2(0.0, 0.0)],
///     indices: vec![0, 1, 2],
/// };
/// assert!(tri.check().is_ok());
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    /// Corner positions.
    pub positions: Vec<Vec3>,
    /// One unit normal per corner. Empty: computed from the triangles.
    pub normals: Vec<Vec3>,
    /// One texture coordinate per corner (0..1, top-left origin). Empty:
    /// all zero.
    pub uvs: Vec<Vec2>,
    /// Three corner indices per triangle.
    pub indices: Vec<u32>,
}

impl MeshData {
    /// A cube `size` wide, centered on the origin, with each face's texture
    /// coordinates covering the whole image.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let cube = MeshData::cube(2.0);
    /// assert_eq!(cube.indices.len(), 36);
    /// ```
    #[must_use]
    pub fn cube(size: f32) -> Self {
        let h = size / 2.0;
        let mut mesh = Self::default();
        // Each face: its normal and two axes along it (right, up).
        let faces = [
            (Vec3::X, Vec3::NEG_Z, Vec3::Y),
            (Vec3::NEG_X, Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::X, Vec3::NEG_Z),
            (Vec3::NEG_Y, Vec3::X, Vec3::Z),
            (Vec3::Z, Vec3::X, Vec3::Y),
            (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
        ];
        for (normal, right, up) in faces {
            let base = mesh.positions.len() as u32;
            for (u, v) in [(0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)] {
                let at = normal * h + right * (u * 2.0 - 1.0) * h + up * (1.0 - v * 2.0) * h;
                mesh.positions.push(at);
                mesh.normals.push(normal);
                mesh.uvs.push(Vec2::new(u, v));
            }
            mesh.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        mesh
    }

    /// A flat square `size` wide on the XZ plane, facing up (+Y): ground.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let ground = MeshData::plane(100.0);
    /// assert!(ground.normals.iter().all(|n| *n == Vec3::Y));
    /// ```
    #[must_use]
    pub fn plane(size: f32) -> Self {
        let h = size / 2.0;
        Self {
            positions: vec![
                Vec3::new(-h, 0.0, h),
                Vec3::new(h, 0.0, h),
                Vec3::new(h, 0.0, -h),
                Vec3::new(-h, 0.0, -h),
            ],
            normals: vec![Vec3::Y; 4],
            uvs: vec![
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 0.0),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    /// A sphere of `radius` around the origin, made of `segments` slices
    /// around and half as many from pole to pole (at least 3).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let ball = MeshData::sphere(0.5, 24);
    /// assert!(ball.positions.iter().all(|p| (p.length() - 0.5).abs() < 1e-5));
    /// ```
    #[must_use]
    pub fn sphere(radius: f32, segments: u32) -> Self {
        let around = segments.max(3);
        let rings = (around / 2).max(2);
        let mut mesh = Self::default();
        for ring in 0..=rings {
            let v = ring as f32 / rings as f32;
            let polar = v * std::f32::consts::PI;
            for step in 0..=around {
                let u = step as f32 / around as f32;
                let azimuth = u * std::f32::consts::TAU;
                let normal = Vec3::new(polar.sin() * azimuth.cos(), polar.cos(), -polar.sin() * azimuth.sin());
                mesh.positions.push(normal * radius);
                mesh.normals.push(normal);
                mesh.uvs.push(Vec2::new(u, v));
            }
        }
        let row = around + 1;
        for ring in 0..rings {
            for step in 0..around {
                let a = ring * row + step;
                let b = a + row;
                if ring > 0 {
                    mesh.indices.extend([a, b, a + 1]);
                }
                if ring + 1 < rings {
                    mesh.indices.extend([a + 1, b, b + 1]);
                }
            }
        }
        mesh
    }

    /// Checks the data fits together: whole triangles, indices in range,
    /// one normal and texture coordinate per corner when given, and finite
    /// numbers.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let broken = MeshData { positions: vec![Vec3::ZERO], indices: vec![0, 1, 2], ..Default::default() };
    /// assert!(broken.check().is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Says what doesn't fit.
    pub fn check(&self) -> Result<(), String> {
        let corners = self.positions.len();
        if self.indices.is_empty() || !self.indices.len().is_multiple_of(3) {
            return Err(format!("{} indices: needs 3 per triangle", self.indices.len()));
        }
        if let Some(&bad) = self.indices.iter().find(|&&i| i as usize >= corners) {
            return Err(format!("index {bad} is past the last of {corners} corners"));
        }
        if !self.normals.is_empty() && self.normals.len() != corners {
            return Err(format!("{} normals for {corners} corners", self.normals.len()));
        }
        if !self.uvs.is_empty() && self.uvs.len() != corners {
            return Err(format!("{} texture coordinates for {corners} corners", self.uvs.len()));
        }
        if !self.positions.iter().all(|p| p.is_finite()) || !self.normals.iter().all(|n| n.is_finite()) {
            return Err("a position or normal isn't a finite number".into());
        }
        Ok(())
    }

    /// The smallest sphere around the origin-centered box that holds every
    /// corner: center and radius. For culling.
    pub(crate) fn bounds(&self) -> (Vec3, f32) {
        let (min, max) = self
            .positions
            .iter()
            .fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            });
        if min.x > max.x {
            return (Vec3::ZERO, 0.0);
        }
        let center = (min + max) / 2.0;
        let radius = self.positions.iter().map(|p| p.distance(center)).fold(0.0, f32::max);
        (center, radius)
    }

    /// Normals for every corner, averaged from the triangles around it,
    /// where none were given.
    pub(crate) fn normals_or_computed(&self) -> std::borrow::Cow<'_, [Vec3]> {
        if !self.normals.is_empty() {
            return std::borrow::Cow::Borrowed(&self.normals);
        }
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        for tri in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| self.positions[i as usize]);
            let n = (b - a).cross(c - a); // area-weighted
            for &i in tri {
                normals[i as usize] += n;
            }
        }
        for n in &mut normals {
            *n = n.normalize_or(Vec3::Y);
        }
        std::borrow::Cow::Owned(normals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every triangle's winding agrees with its corners' normals (faces out).
    fn faces_out(mesh: &MeshData) {
        for tri in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| mesh.positions[i as usize]);
            let face = (b - a).cross(c - a);
            if face.length() < 1e-9 {
                continue; // degenerate at a pole
            }
            let normal = mesh.normals[tri[0] as usize] + mesh.normals[tri[1] as usize] + mesh.normals[tri[2] as usize];
            assert!(face.dot(normal) > 0.0, "triangle {tri:?} faces in");
        }
    }

    #[test]
    fn built_in_shapes_are_valid_and_face_out() {
        for mesh in [MeshData::cube(1.0), MeshData::plane(3.0), MeshData::sphere(1.0, 16)] {
            mesh.check().unwrap();
            faces_out(&mesh);
        }
    }

    #[test]
    fn bounds_hold_every_corner() {
        let mesh = MeshData::cube(2.0);
        let (center, radius) = mesh.bounds();
        assert!(center.length() < 1e-6);
        assert!((radius - 3f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn missing_normals_are_computed_from_the_triangles() {
        let mut plane = MeshData::plane(1.0);
        plane.normals.clear();
        assert!(
            plane
                .normals_or_computed()
                .iter()
                .all(|n| (*n - Vec3::Y).length() < 1e-6)
        );
    }
}
