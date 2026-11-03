//! Each frame: the visible [`Model`]s, as GPU instances grouped by mesh.

use glam::{Mat4, Vec3, Vec4};

use super::{Assets3D, Model, Transform3D, linear};
use crate::World;

/// One drawn model, as the GPU reads it: the model matrix's columns, and the
/// tint in linear color.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance3D {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

/// This frame's 3D drawing: instances sorted by mesh, and where each mesh's
/// run starts. Kept between frames so it doesn't allocate once warm.
#[derive(Default)]
pub(crate) struct Frame3D {
    pub instances: Vec<Instance3D>,
    /// (mesh id, first instance, count), in mesh order.
    pub runs: Vec<(u32, u32, u32)>,
    pub view_proj: Mat4,
    /// Unsorted visible instances with their mesh, before grouping.
    scratch: Vec<(u32, Instance3D)>,
    counts: Vec<u32>,
}

impl Frame3D {
    /// Fills the frame with the models `view_proj` can see.
    pub fn collect(&mut self, world: &mut World, assets: &Assets3D, view_proj: Mat4) {
        self.view_proj = view_proj;
        self.instances.clear();
        self.runs.clear();
        self.scratch.clear();
        let planes = frustum_planes(view_proj);
        let scratch = &mut self.scratch;
        world.query::<(&Transform3D, &Model)>().for_each(|(t, m)| {
            let Some(&(center, radius)) = assets.bounds.get(m.mesh.id as usize) else {
                return;
            };
            let matrix = t.matrix();
            let world_center = matrix.transform_point3(center);
            let world_radius = radius * t.scale.abs().max_element();
            if !sphere_visible(&planes, world_center, world_radius) {
                return;
            }
            scratch.push((
                m.mesh.id,
                Instance3D {
                    model: matrix.to_cols_array_2d(),
                    color: linear(m.color),
                },
            ));
        });
        if self.scratch.is_empty() {
            return;
        }
        // Counting sort by mesh: one pass to count, one to place.
        self.counts.clear();
        self.counts.resize(assets.bounds.len(), 0);
        for (mesh, _) in &self.scratch {
            self.counts[*mesh as usize] += 1;
        }
        let mut start = 0;
        for (mesh, count) in self.counts.iter_mut().enumerate() {
            if *count > 0 {
                self.runs.push((mesh as u32, start, *count));
            }
            let n = *count;
            *count = start; // now: where this mesh's next instance goes
            start += n;
        }
        self.instances.resize(self.scratch.len(), Instance3D::default());
        for (mesh, instance) in &self.scratch {
            let at = &mut self.counts[*mesh as usize];
            self.instances[*at as usize] = *instance;
            *at += 1;
        }
    }
}

/// The six planes of the view frustum (`xyz` normal pointing in, `w`
/// distance), from a projection-times-view matrix with 0..1 depth.
fn frustum_planes(m: Mat4) -> [Vec4; 6] {
    let r = [m.row(0), m.row(1), m.row(2), m.row(3)];
    let planes = [r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]];
    planes.map(|p| p / p.truncate().length().max(f32::MIN_POSITIVE))
}

fn sphere_visible(planes: &[Vec4; 6], center: Vec3, radius: f32) -> bool {
    planes.iter().all(|p| p.truncate().dot(center) + p.w >= -radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Mesh};

    fn camera() -> Mat4 {
        let projection = glam::camera::rh::proj::directx::perspective(1.0, 1.0, 0.1, 100.0);
        projection * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y)
    }

    #[test]
    fn spheres_outside_the_view_are_culled() {
        let planes = frustum_planes(camera());
        assert!(sphere_visible(&planes, Vec3::ZERO, 1.0));
        assert!(!sphere_visible(&planes, Vec3::new(0.0, 0.0, 20.0), 1.0), "behind");
        assert!(!sphere_visible(&planes, Vec3::new(0.0, 0.0, -200.0), 1.0), "past far");
        assert!(!sphere_visible(&planes, Vec3::new(50.0, 0.0, 0.0), 1.0), "far right");
        assert!(sphere_visible(&planes, Vec3::new(5.0, 0.0, 0.0), 1.0), "just in");
        assert!(
            sphere_visible(&planes, Vec3::new(0.0, 0.0, 10.5), 1.0),
            "big enough to reach in"
        );
    }

    #[test]
    fn visible_models_are_grouped_by_mesh() {
        let mut world = World::new();
        let assets = Assets3D {
            bounds: vec![(Vec3::ZERO, 1.0); 3],
            ..Default::default()
        };
        let model = |id| Model::new(Mesh { id }).with_color(Color::WHITE);
        for (i, mesh) in [2, 0, 2, 2, 0].into_iter().enumerate() {
            world.spawn((Transform3D::at(Vec3::new(i as f32, 0.0, 0.0)), model(mesh)));
        }
        world.spawn((Transform3D::at(Vec3::new(0.0, 0.0, 50.0)), model(1))); // behind the camera
        let mut frame = Frame3D::default();
        frame.collect(&mut world, &assets, camera());
        assert_eq!(frame.runs, vec![(0, 0, 2), (2, 2, 3)]);
        let xs: Vec<f32> = frame.instances.iter().map(|i| i.model[3][0]).collect();
        assert_eq!(xs, vec![1.0, 4.0, 0.0, 2.0, 3.0]);
    }
}
