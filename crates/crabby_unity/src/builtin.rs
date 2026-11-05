//! Unity's built-in meshes (in the built-in resources, GUID
//! `0000000000000000e000000000000000`), made here since they aren't in
//! the project: Cube, Plane and Quad.

use glam::Vec3;

use crate::UnityMesh;

/// The built-in Cube's id in the built-in resources.
pub const CUBE: i64 = 10202;
/// The built-in Plane's id.
pub const PLANE: i64 = 10209;
/// The built-in Quad's id.
pub const QUAD: i64 = 10210;

/// The names of the built-in meshes not made here, for reports.
#[must_use]
pub(crate) fn name(file_id: i64) -> Option<&'static str> {
    Some(match file_id {
        CUBE => "Cube",
        10206 => "Cylinder",
        10207 => "Sphere",
        10208 => "Capsule",
        PLANE => "Plane",
        QUAD => "Quad",
        _ => return None,
    })
}

impl UnityMesh {
    /// The built-in mesh with this id, in Unity's space as a mesh asset
    /// would have it: Cube (10202, 1 m, each face's UVs 0 to 1), Plane
    /// (10209, 10 × 10 m facing up, 10 × 10 squares) or Quad (10210,
    /// 1 m facing -Z). `None` for the others (Sphere, Capsule, Cylinder)
    /// and ids that aren't built-in meshes.
    ///
    /// ```
    /// let cube = crabby_unity::UnityMesh::builtin(10202).unwrap();
    /// assert_eq!((cube.name.as_str(), cube.positions.len()), ("Cube", 24));
    /// assert!(crabby_unity::UnityMesh::builtin(10207).is_none(), "no Sphere yet");
    /// ```
    #[must_use]
    pub fn builtin(file_id: i64) -> Option<Self> {
        let mut mesh = Self {
            name: name(file_id)?.to_string(),
            skin: None,
            blend_shapes: Vec::new(),
            positions: Vec::new(),
            normals: Vec::new(),
            tangents: Vec::new(),
            colors: Vec::new(),
            uvs: Default::default(),
            submeshes: vec![Vec::new()],
        };
        let (x, y, z) = (Vec3::X, Vec3::Y, Vec3::Z);
        match file_id {
            CUBE => {
                // (normal, across, up): across × up = normal.
                for (n, du, dv) in [(x, -z, y), (-x, z, y), (z, x, y), (-z, -x, y), (y, x, -z), (-y, x, z)] {
                    mesh.face(n * 0.5, du, dv, 1, |_, s, t| [s, t]);
                }
            }
            PLANE => mesh.face(Vec3::ZERO, x * 10.0, -z * 10.0, 10, |p, _, _| {
                [0.5 - p.x / 10.0, 0.5 - p.z / 10.0]
            }),
            QUAD => mesh.face(Vec3::ZERO, y, x, 1, |p, _, _| [p.x + 0.5, p.y + 0.5]),
            _ => return None,
        }
        Some(mesh)
    }

    /// Adds a flat square of `segments²` cells centred on `center`,
    /// spanning `du` and `dv`, facing `du × dv`; `uv` gives a vertex's
    /// UV from its position and how far along `du` and `dv` it is (0 to 1).
    fn face(&mut self, center: Vec3, du: Vec3, dv: Vec3, segments: u32, uv: impl Fn(Vec3, f32, f32) -> [f32; 2]) {
        let normal = du.cross(dv).normalize();
        let base = self.positions.len() as u32;
        let row = segments + 1;
        for j in 0..row {
            for i in 0..row {
                let (s, t) = (i as f32 / segments as f32, j as f32 / segments as f32);
                let p = center + du * (s - 0.5) + dv * (t - 0.5);
                self.positions.push(p.into());
                self.normals.push(normal.into());
                self.uvs[0].push(uv(p, s, t));
            }
        }
        let tris = &mut self.submeshes[0];
        for j in 0..segments {
            for i in 0..segments {
                let a = base + j * row + i;
                let (b, c, d) = (a + 1, a + row + 1, a + row);
                // (b - a) × (c - a) along the normal, as Unity's meshes
                // have it before the mirror.
                tris.extend([a, b, c, a, c, d]);
            }
        }
    }
}
