//! Mesh decals: the part of a mesh inside a box, cut out as a mesh of its
//! own with the box's image projected on it (bullet holes, scorch marks,
//! splats, signs, tyre marks).

use glam::{Mat4, Vec2, Vec3};

use super::MeshData;

/// A corner while clipping: in the projector's box, and its normal in the
/// mesh.
#[derive(Clone, Copy)]
struct Corner {
    at: Vec3,
    normal: Vec3,
}

impl Corner {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            at: self.at.lerp(other.at, t),
            normal: self.normal.lerp(other.normal, t),
        }
    }
}

impl MeshData {
    /// A decal: the part of this mesh inside a box, as a mesh of its own,
    /// with an image projected straight onto it. `projector` places the
    /// box in this mesh's space: it maps the unit cube (-0.5 to 0.5 on each
    /// axis) onto the box, so its scale is the decal's size and depth. The
    /// image is projected along the box's -Z (looking from +Z), its top
    /// along +Y; only faces turned toward +Z get it.
    ///
    /// Each corner is lifted `lift` meters along its normal so the decal
    /// sits on the surface without flickering (a few millimeters is
    /// typical). Draw the result with the decal's material (usually cut out
    /// or transparent) and the same transform as the mesh it was cut from.
    /// It's empty when the box misses the mesh.
    ///
    /// ```
    /// # use ponko::*;
    /// // A 4 x 4 m floor facing up.
    /// let floor = MeshData::plane(4.0);
    /// // A 0.5 m scorch mark at (1, 0, 0), projected from above (the box's
    /// // +Z turned to point up), 0.2 m deep.
    /// let down = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    /// let at = Mat4::from_scale_rotation_translation(vec3(0.5, 0.5, 0.2), down, vec3(1.0, 0.0, 0.0));
    /// let mark = floor.decal(at, 0.002);
    /// assert!(!mark.indices.is_empty());
    /// assert!(mark.positions.iter().all(|p| (p.x - 1.0).abs() <= 0.25 + 1e-4 && p.z.abs() <= 0.25 + 1e-4));
    /// // Then: ctx.gfx.create_mesh(&mark, scorch_material), drawn where the floor is.
    /// ```
    #[must_use]
    pub fn decal(&self, projector: Mat4, lift: f32) -> MeshData {
        let mut out = MeshData::default();
        let into_box = projector.inverse();
        if !into_box.is_finite() {
            return out;
        }
        let normals = self.normals_or_computed();
        // The box's +Z in the mesh, for which faces look at the projector.
        let toward = projector.transform_vector3(Vec3::Z);
        let mut polygon = Vec::with_capacity(9);
        let mut scratch = Vec::with_capacity(9);
        for tri in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = tri.map(|i| self.positions[i as usize]);
            let face = (b - a).cross(c - a);
            if face.dot(toward) <= 0.0 {
                continue; // turned away from the projector
            }
            polygon.clear();
            polygon.extend(tri.map(|i| Corner {
                at: into_box.transform_point3(self.positions[i as usize]),
                normal: normals[i as usize],
            }));
            for axis in 0..3 {
                for sign in [-1.0f32, 1.0] {
                    // Keep the side where sign * at[axis] <= 0.5.
                    clip(&mut polygon, &mut scratch, |c| 0.5 - sign * c.at[axis]);
                    if polygon.len() < 3 {
                        break;
                    }
                }
            }
            if polygon.len() < 3 {
                continue;
            }
            let base = out.positions.len() as u32;
            for c in &polygon {
                let n = c.normal.normalize_or(face.normalize_or_zero());
                out.positions.push(projector.transform_point3(c.at) + n * lift);
                out.normals.push(n);
                out.uvs.push(Vec2::new(c.at.x + 0.5, 0.5 - c.at.y));
            }
            for k in 1..polygon.len() as u32 - 1 {
                out.indices.extend([base, base + k, base + k + 1]);
            }
        }
        out
    }
}

/// Clips `polygon` to where `inside(corner) >= 0` (Sutherland-Hodgman).
fn clip(polygon: &mut Vec<Corner>, scratch: &mut Vec<Corner>, inside: impl Fn(&Corner) -> f32) {
    scratch.clear();
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        let (da, db) = (inside(&a), inside(&b));
        if da >= 0.0 {
            scratch.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            scratch.push(a.lerp(b, da / (da - db)));
        }
    }
    std::mem::swap(polygon, scratch);
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    /// A 4 x 4 m floor facing +Y, as two triangles.
    fn floor() -> MeshData {
        MeshData::plane(4.0)
    }

    /// A box `size` wide looking straight down at `at`.
    fn from_above(at: Vec3, size: f32) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            Vec3::new(size, size, 1.0),
            Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            at,
        )
    }

    #[test]
    fn decals_are_clipped_to_their_box_and_lifted() {
        let mark = floor().decal(from_above(Vec3::new(0.5, 0.0, 0.5), 1.0), 0.01);
        assert!(mark.check().is_ok());
        assert!(!mark.indices.is_empty());
        for p in &mark.positions {
            assert!(
                (0.0 - 1e-4..=1.0 + 1e-4).contains(&p.x) && (0.0 - 1e-4..=1.0 + 1e-4).contains(&p.z),
                "{p}"
            );
            assert!((p.y - 0.01).abs() < 1e-5, "lifted along the normal: {p}");
        }
        // It covers the whole box: 1 m², whatever the triangles.
        let area: f32 = mark
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| mark.positions[i as usize]);
                (b - a).cross(c - a).length() / 2.0
            })
            .sum();
        assert!((area - 1.0).abs() < 1e-4, "{area}");
    }

    #[test]
    fn decal_uvs_follow_the_box() {
        let mark = floor().decal(from_above(Vec3::ZERO, 2.0), 0.0);
        for (p, uv) in mark.positions.iter().zip(&mark.uvs) {
            // Looking down, the box's x is the world's x and its y (the
            // image's up) is the world's -z.
            assert!((uv.x - (p.x / 2.0 + 0.5)).abs() < 1e-5);
            assert!((uv.y - (p.z / 2.0 + 0.5)).abs() < 1e-5);
            assert!((0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y));
        }
    }

    #[test]
    fn faces_turned_away_and_boxes_that_miss_get_nothing() {
        // Projected from below: the floor faces away.
        let below = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
        assert!(floor().decal(below, 0.0).indices.is_empty());
        // Above the floor, out of the box's depth.
        assert!(
            floor()
                .decal(from_above(Vec3::new(0.0, 5.0, 0.0), 1.0), 0.0)
                .indices
                .is_empty()
        );
        // Beside the floor.
        assert!(
            floor()
                .decal(from_above(Vec3::new(10.0, 0.0, 0.0), 1.0), 0.0)
                .indices
                .is_empty()
        );
        // A box with no size.
        assert!(floor().decal(Mat4::ZERO, 0.0).indices.is_empty());
    }

    #[test]
    fn decals_wrap_around_corners() {
        // A cube's corner, seen from a box at 45 degrees: three faces
        // could be hit, but only the two facing the projector are.
        let cube = MeshData::cube(1.0);
        let projector = Mat4::from_scale_rotation_translation(
            Vec3::splat(0.6),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
            Vec3::new(0.5, 0.0, 0.5),
        );
        let mark = cube.decal(projector, 0.0);
        let mut faces: Vec<[i32; 3]> = mark.normals.iter().map(|n| n.round().as_ivec3().to_array()).collect();
        faces.sort_unstable();
        faces.dedup();
        assert_eq!(faces, [[0, 0, 1], [1, 0, 0]]);
    }
}
