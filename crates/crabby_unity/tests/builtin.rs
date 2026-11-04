use crabby_unity::UnityMesh;
use glam::Vec3;

fn v(p: [f32; 3]) -> Vec3 {
    Vec3::from(p)
}

/// Every triangle's (b - a) × (c - a) runs along its corners' normals,
/// as in Unity's own meshes (the importer turns the winding around when
/// it mirrors them).
fn wound_like_unity(mesh: &UnityMesh) {
    for tri in mesh.submeshes[0].as_chunks::<3>().0 {
        let [a, b, c] = [0, 1, 2].map(|i| v(mesh.positions[tri[i] as usize]));
        let face = (b - a).cross(c - a).normalize();
        for &i in tri {
            assert!(
                face.dot(v(mesh.normals[i as usize])) > 0.999,
                "{} triangle {tri:?}",
                mesh.name
            );
        }
    }
}

fn bounds(mesh: &UnityMesh) -> (Vec3, Vec3) {
    let ps = mesh.positions.iter().map(|&p| v(p));
    (ps.clone().fold(Vec3::MAX, Vec3::min), ps.fold(Vec3::MIN, Vec3::max))
}

#[test]
fn the_cube_is_a_metre_with_faces_pointing_out() {
    let cube = UnityMesh::builtin(10202).unwrap();
    assert_eq!((cube.positions.len(), cube.submeshes[0].len()), (24, 36));
    assert_eq!(bounds(&cube), (Vec3::splat(-0.5), Vec3::splat(0.5)));
    wound_like_unity(&cube);
    for (p, n) in cube.positions.iter().zip(&cube.normals) {
        assert_eq!(v(*p).dot(v(*n)), 0.5, "on its face, facing away from the middle");
    }
    assert!(cube.uvs[0].iter().all(|uv| uv.iter().all(|c| (0.0..=1.0).contains(c))));
}

#[test]
fn the_plane_is_ten_metres_facing_up() {
    let plane = UnityMesh::builtin(10209).unwrap();
    assert_eq!((plane.positions.len(), plane.submeshes[0].len()), (121, 600));
    assert_eq!(bounds(&plane), (Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 0.0, 5.0)));
    wound_like_unity(&plane);
    assert!(plane.normals.iter().all(|&n| n == [0.0, 1.0, 0.0]));
}

#[test]
fn the_quad_is_a_metre_facing_minus_z() {
    let quad = UnityMesh::builtin(10210).unwrap();
    assert_eq!(bounds(&quad), (Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0)));
    wound_like_unity(&quad);
    assert!(quad.normals.iter().all(|&n| n == [0.0, 0.0, -1.0]));
    for (p, uv) in quad.positions.iter().zip(&quad.uvs[0]) {
        assert_eq!(*uv, [p[0] + 0.5, p[1] + 0.5], "UVs follow X and Y");
    }
}

#[test]
fn other_ids_are_not_made() {
    for id in [10206, 10207, 10208, 0, 4300000] {
        assert!(UnityMesh::builtin(id).is_none(), "{id}");
    }
}
