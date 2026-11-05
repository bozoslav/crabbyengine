//! Skinned meshes: a [`Skeleton`] of joints, posed every frame, bends the
//! meshes bound to it (characters, animals, hands).

use std::sync::Arc;

use glam::{Mat4, Vec3};

use super::collect::Instance3D;
use super::{Assets3D, MeshData, Model, Transform3D, linear};
use crate::{Entity, World};

/// One joint of a [`Skeleton`]: its name, the joint it hangs from, and
/// where it sits relative to that joint when nothing moves it (its rest
/// pose).
///
/// ```
/// # use crabbyengine::*;
/// let shoulder = Joint { name: "shoulder".into(), parent: None, rest: Transform3D::default() };
/// let elbow = Joint { name: "elbow".into(), parent: Some(0), rest: Transform3D::at(vec3(0.0, -0.3, 0.0)) };
/// let arm = Skeleton::new(vec![shoulder, elbow]);
/// assert_eq!(arm.find("elbow"), Some(1));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    /// Its name, for finding it and for binding meshes made for another
    /// skeleton with the same names.
    pub name: String,
    /// The joint it hangs from (an index into the same list, before this
    /// one); `None` for a root, which sits in the entity's own space.
    pub parent: Option<usize>,
    /// Where it sits relative to its parent at rest.
    pub rest: Transform3D,
}

/// A component that poses the skinned [`Model`] on the same entity (and the
/// models whose [`SkinOf`] points at it): each joint's `pose` is where it
/// sits relative to its parent this frame, and the mesh's corners follow
/// the joints they're weighted to. Loaded from a glTF file with
/// [`Ctx::load_skeleton`](crate::Ctx::load_skeleton), or made in code with
/// [`Skeleton::new`]. Cloning one is cheap: the joints are shared.
///
/// A skinned model without a skeleton draws in the pose it was modelled in.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let body = ctx.load_mesh("models/fox.glb");
/// let skeleton = ctx.load_skeleton("models/fox.glb");
/// let head = skeleton.find("head").unwrap();
/// let fox = ctx.world.spawn((Transform3D::default(), Model::new(body), skeleton));
/// // Later, in a system: turn the head.
/// let s = ctx.world.get_mut::<Skeleton>(fox).unwrap();
/// s.pose[head].rotation = Quat::from_rotation_y(0.5);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Skeleton {
    joints: Arc<[Joint]>,
    /// Where each joint sits relative to its parent, as the next frame
    /// draws it; starts at the rest pose. Joints past its end stay at rest.
    pub pose: Vec<Transform3D>,
    /// Each joint's matrix in the entity's space, worked out from `pose`
    /// just before drawing.
    matrices: Vec<Mat4>,
}

impl Skeleton {
    /// A skeleton of `joints`, at rest.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let root = Joint { name: "root".into(), parent: None, rest: Transform3D::at(vec3(0.0, 1.0, 0.0)) };
    /// let s = Skeleton::new(vec![root]);
    /// assert_eq!(s.joint_matrix(0), Mat4::from_translation(vec3(0.0, 1.0, 0.0)));
    /// ```
    ///
    /// # Panics
    ///
    /// If a joint's parent isn't an earlier joint, or there are more than
    /// 65536 joints.
    #[must_use]
    pub fn new(joints: Vec<Joint>) -> Self {
        assert!(joints.len() <= 1 << 16, "Skeleton::new: more than 65536 joints");
        for (i, j) in joints.iter().enumerate() {
            if let Some(p) = j.parent {
                assert!(
                    p < i,
                    "Skeleton::new: joint {i} ({}) hangs from joint {p}, which doesn't come before it",
                    j.name
                );
            }
        }
        Self::from_shared(joints.into())
    }

    pub(crate) fn from_shared(joints: Arc<[Joint]>) -> Self {
        let pose = joints.iter().map(|j| j.rest).collect();
        let mut s = Self {
            joints,
            pose,
            matrices: Vec::new(),
        };
        s.update();
        s
    }

    /// The joints, in order (parents before children).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let s = Skeleton::new(vec![Joint { name: "hip".into(), parent: None, rest: Transform3D::default() }]);
    /// assert_eq!(s.joints()[0].name, "hip");
    /// ```
    #[must_use]
    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }

    /// The first joint called `name`.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let s = Skeleton::new(vec![Joint { name: "hip".into(), parent: None, rest: Transform3D::default() }]);
    /// assert_eq!(s.find("hip"), Some(0));
    /// assert_eq!(s.find("tail"), None);
    /// ```
    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }

    /// Puts every joint back in its rest pose.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut s = Skeleton::new(vec![Joint { name: "hip".into(), parent: None, rest: Transform3D::default() }]);
    /// s.pose[0].position.y = 3.0;
    /// s.reset_pose();
    /// assert_eq!(s.pose[0], Transform3D::default());
    /// ```
    pub fn reset_pose(&mut self) {
        self.pose.clear();
        self.pose.extend(self.joints.iter().map(|j| j.rest));
    }

    /// Joint `joint`'s matrix in the entity's own space, from `pose` as it
    /// is now: where a hand is, to put a cup in it. Multiply by the entity's
    /// [`Transform3D::matrix`] for the world.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let root = Joint { name: "root".into(), parent: None, rest: Transform3D::at(vec3(0.0, 1.0, 0.0)) };
    /// let tip = Joint { name: "tip".into(), parent: Some(0), rest: Transform3D::at(vec3(0.0, 2.0, 0.0)) };
    /// let mut s = Skeleton::new(vec![root, tip]);
    /// s.pose[0].rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    /// let at = s.joint_matrix(1).transform_point3(Vec3::ZERO);
    /// assert!((at - vec3(-2.0, 1.0, 0.0)).length() < 1e-5);
    /// ```
    ///
    /// # Panics
    ///
    /// If there's no joint `joint`.
    #[must_use]
    pub fn joint_matrix(&self, joint: usize) -> Mat4 {
        let mut m = self.local(joint);
        let mut at = self.joints[joint].parent;
        while let Some(p) = at {
            m = self.local(p) * m;
            at = self.joints[p].parent;
        }
        m
    }

    fn local(&self, joint: usize) -> Mat4 {
        self.pose.get(joint).unwrap_or(&self.joints[joint].rest).matrix()
    }

    /// Works out every joint's matrix from the pose, for drawing.
    pub(crate) fn update(&mut self) {
        self.matrices.clear();
        for (i, j) in self.joints.iter().enumerate() {
            let local = self.pose.get(i).unwrap_or(&j.rest).matrix();
            let m = match j.parent {
                Some(p) => self.matrices[p] * local,
                None => local,
            };
            self.matrices.push(m);
        }
    }

    /// The joint matrices as of the last [`update`](Self::update).
    pub(crate) fn matrices(&self) -> &[Mat4] {
        &self.matrices
    }

    pub(crate) fn shares_joints(&self, joints: &Arc<[Joint]>) -> bool {
        Arc::ptr_eq(&self.joints, joints)
    }

    pub(crate) fn shared_joints(&self) -> &Arc<[Joint]> {
        &self.joints
    }
}

/// Poses this entity's skinned [`Model`] with another entity's
/// [`Skeleton`], and draws it where that entity is: clothes, hair and
/// swappable parts made for the same skeleton. Joints are matched by name,
/// so a part works on any skeleton that has its joints. The entity needs no
/// [`Transform3D`] of its own.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let body = ctx.load_mesh("people/body.glb");
/// let hat = ctx.load_mesh("people/hat.glb");
/// let person = ctx.world.spawn((Transform3D::default(), Model::new(body), ctx.load_skeleton("people/body.glb")));
/// ctx.world.spawn((Model::new(hat), SkinOf(person)));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinOf(pub Entity);

/// Keeps this entity's [`Transform3D`] on a joint of another entity's
/// [`Skeleton`], moved by `offset` in the joint's space: a gun in a hand, a
/// hat on a head. The engine sets it just before drawing, so systems see
/// last frame's place; [`Skeleton::joint_matrix`] gives this frame's.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx, person: Entity, cup: Mesh) {
/// let hand = ctx.world.get::<Skeleton>(person).unwrap().find("hand_r").unwrap();
/// ctx.world.spawn((
///     Transform3D::default(),
///     Model::new(cup),
///     AttachToJoint { skeleton: person, joint: hand, offset: Transform3D::at(vec3(0.0, 0.05, 0.0)) },
/// ));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttachToJoint {
    /// The entity with the [`Skeleton`].
    pub skeleton: Entity,
    /// Which of its joints.
    pub joint: usize,
    /// Where this entity sits in the joint's space.
    pub offset: Transform3D,
}

/// How a skinned mesh binds to joints: per joint it uses (a "slot", which
/// the corners' `joints` index), the joint's name and index in the joints
/// it was made with, the inverse of where that joint was when the mesh was
/// modelled, and a sphere around the corners it moves.
#[derive(Debug)]
pub(crate) struct MeshSkin {
    pub joints: Arc<[Joint]>,
    /// Per slot: index into `joints`.
    pub slots: Vec<u16>,
    pub inverse_binds: Vec<Mat4>,
    /// Per slot, in the mesh's own (bind) space: center and radius of the
    /// corners it moves; a negative radius when it moves none.
    pub bounds: Vec<(Vec3, f32)>,
}

impl MeshSkin {
    pub fn new(joints: Arc<[Joint]>, slots: Vec<u16>, inverse_binds: Vec<Mat4>, data: &MeshData) -> Self {
        let n = slots.len();
        let mut boxes = vec![(Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)); n];
        for_each_influence(data, |corner, slot| {
            let (lo, hi) = &mut boxes[slot];
            *lo = lo.min(data.positions[corner]);
            *hi = hi.max(data.positions[corner]);
        });
        let centers: Vec<Vec3> = boxes.iter().map(|(lo, hi)| (*lo + *hi) / 2.0).collect();
        let mut radii = vec![-1.0f32; n];
        for_each_influence(data, |corner, slot| {
            radii[slot] = radii[slot].max(data.positions[corner].distance(centers[slot]));
        });
        Self {
            joints,
            slots,
            inverse_binds,
            bounds: centers.into_iter().zip(radii).collect(),
        }
    }

    /// Per slot: the joint of `skeleton` it follows (by index when the
    /// skeleton has the joints the mesh was made with, else by name).
    fn joint_in(&self, skeleton: &Skeleton, slot: usize) -> Option<usize> {
        let own = usize::from(self.slots[slot]);
        if skeleton.shares_joints(&self.joints) {
            return Some(own);
        }
        skeleton.find(&self.joints[own].name)
    }
}

/// Calls `f(corner, slot)` for every joint a corner is weighted to.
fn for_each_influence(data: &MeshData, mut f: impl FnMut(usize, usize)) {
    for (corner, (joints, weights)) in data.joints.iter().zip(&data.weights).enumerate() {
        for k in 0..4 {
            if weights[k] > 0.0 {
                f(corner, usize::from(joints[k]));
            }
        }
    }
}

/// Weights that add up to 1 (all on the first joint when they're all 0).
pub(crate) fn normalized(w: [f32; 4]) -> [f32; 4] {
    let w = w.map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 });
    let sum: f32 = w.iter().sum();
    if sum <= 0.0 {
        [1.0, 0.0, 0.0, 0.0]
    } else {
        w.map(|v| v / sum)
    }
}

/// The matrix that moves a corner weighted `weights` to `joints`, given
/// each slot's skin matrix.
pub(crate) fn blend(skin: &[Mat4], joints: [u16; 4], weights: [f32; 4]) -> Mat4 {
    let mut m = Mat4::ZERO;
    for k in 0..4 {
        if weights[k] > 0.0 {
            m += skin.get(usize::from(joints[k])).copied().unwrap_or(Mat4::IDENTITY) * weights[k];
        }
    }
    m
}

/// One skinned model to draw this frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SkinDraw {
    /// The skinned mesh.
    pub mesh: u32,
    pub instance: Instance3D,
    /// The model matrix, and its largest scale.
    pub matrix: Mat4,
    pub scale: f32,
    /// A sphere around the posed mesh, in model space; negative radius:
    /// nothing to draw.
    pub bounds: (Vec3, f32),
    /// Its first skin matrix in `SkinFrame::matrices` (one per slot).
    pub first: u32,
}

/// This frame's skinned models and their skin matrices: the joints' matrices
/// times each slot's inverse bind matrix. Target cameras draw the same.
#[derive(Default)]
pub(crate) struct SkinFrame {
    pub draws: Vec<SkinDraw>,
    pub matrices: Vec<Mat4>,
    attached: Vec<(Entity, AttachToJoint)>,
    moved: Vec<(Entity, Transform3D)>,
    shared: Vec<(Model, Entity)>,
}

impl SkinFrame {
    /// Poses every skeleton, moves what's attached to joints, and lists the
    /// skinned models.
    pub fn collect(&mut self, world: &mut World, assets: &Assets3D) {
        self.draws.clear();
        self.matrices.clear();
        world.query::<&mut Skeleton>().par_for_each(Skeleton::update);

        // Attachments, in two steps: the query can't look at other entities.
        self.attached.clear();
        world
            .query::<(Entity, &AttachToJoint)>()
            .for_each(|(e, a)| self.attached.push((e, *a)));
        self.moved.clear();
        for &(e, a) in &self.attached {
            let (Some(s), Some(t)) = (world.get::<Skeleton>(a.skeleton), world.get::<Transform3D>(a.skeleton)) else {
                continue;
            };
            let Some(joint) = s.matrices().get(a.joint) else {
                continue;
            };
            let (scale, rotation, position) = (t.matrix() * *joint * a.offset.matrix()).to_scale_rotation_translation();
            self.moved.push((
                e,
                Transform3D {
                    rotation,
                    position,
                    scale,
                },
            ));
        }
        for &(e, t) in &self.moved {
            if let Some(mine) = world.get_mut::<Transform3D>(e) {
                *mine = t;
            }
        }

        let (draws, matrices) = (&mut self.draws, &mut self.matrices);
        world
            .query::<(&Transform3D, &Model, &Skeleton)>()
            .for_each(|(t, m, s)| add(draws, matrices, assets, t, m, s));
        self.shared.clear();
        world
            .query::<(&Model, &SkinOf)>()
            .for_each(|(m, of)| self.shared.push((*m, of.0)));
        for (m, of) in &self.shared {
            if let (Some(t), Some(s)) = (world.get::<Transform3D>(*of), world.get::<Skeleton>(*of)) {
                add(draws, matrices, assets, t, m, s);
            }
        }
    }
}

/// Adds model `m` posed by `s`, if its mesh is skinned and live.
fn add(
    draws: &mut Vec<SkinDraw>,
    matrices: &mut Vec<Mat4>,
    assets: &Assets3D,
    t: &Transform3D,
    m: &Model,
    s: &Skeleton,
) {
    let Some(skin) = assets.skin(m.mesh.id) else {
        return;
    };
    if assets.live_bounds(m.mesh.id).is_none() {
        return;
    }
    let first = matrices.len() as u32;
    let joints = s.matrices();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let mut spheres = 0;
    for slot in 0..skin.slots.len() {
        let skin_matrix = match skin.joint_in(s, slot).and_then(|j| joints.get(j)) {
            Some(joint) => *joint * skin.inverse_binds[slot],
            None => Mat4::IDENTITY, // no such joint: as modelled
        };
        matrices.push(skin_matrix);
        let (center, radius) = skin.bounds[slot];
        if radius >= 0.0 {
            let c = skin_matrix.transform_point3(center);
            let r = radius * max_scale(skin_matrix);
            lo = lo.min(c - r);
            hi = hi.max(c + r);
            spheres += 1;
        }
    }
    let bounds = if spheres == 0 {
        (Vec3::ZERO, -1.0)
    } else {
        ((lo + hi) / 2.0, (hi - lo).length() / 2.0)
    };
    let matrix = t.matrix();
    draws.push(SkinDraw {
        mesh: m.mesh.id,
        instance: Instance3D {
            model: matrix.to_cols_array_2d(),
            color: linear(m.color),
        },
        matrix,
        scale: t.scale.abs().max_element(),
        bounds,
        first,
    });
}

/// The longest of a matrix's three axes.
fn max_scale(m: Mat4) -> f32 {
    m.x_axis
        .truncate()
        .length()
        .max(m.y_axis.truncate().length())
        .max(m.z_axis.truncate().length())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, vec3};

    fn arm() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                name: "root".into(),
                parent: None,
                rest: Transform3D::default(),
            },
            Joint {
                name: "tip".into(),
                parent: Some(0),
                rest: Transform3D::at(vec3(0.0, 1.0, 0.0)),
            },
        ])
    }

    #[test]
    fn update_matches_joint_matrix() {
        let mut s = arm();
        s.pose[0].rotation = Quat::from_rotation_x(0.7);
        s.pose[1].scale = Vec3::splat(2.0);
        s.update();
        for j in 0..2 {
            assert!(s.matrices()[j].abs_diff_eq(s.joint_matrix(j), 1e-6));
        }
    }

    #[test]
    fn weights_are_normalized_and_blended() {
        assert_eq!(normalized([2.0, 2.0, 0.0, 0.0]), [0.5, 0.5, 0.0, 0.0]);
        assert_eq!(normalized([0.0; 4]), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(normalized([f32::NAN, -1.0, 1.0, 0.0]), [0.0, 0.0, 1.0, 0.0]);
        let skin = [Mat4::from_translation(Vec3::X), Mat4::from_translation(Vec3::Y)];
        let m = blend(&skin, [0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0]);
        assert!(m.transform_point3(Vec3::ZERO).abs_diff_eq(vec3(0.5, 0.5, 0.0), 1e-6));
    }

    #[test]
    fn slot_bounds_hold_the_corners_each_joint_moves() {
        let data = MeshData {
            positions: vec![Vec3::ZERO, vec3(0.0, 2.0, 0.0), vec3(0.0, 4.0, 0.0)],
            indices: vec![0, 1, 2],
            joints: vec![[0, 0, 0, 0], [0, 1, 0, 0], [1, 0, 0, 0]],
            weights: vec![[1.0, 0.0, 0.0, 0.0], [0.5, 0.5, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]],
            ..Default::default()
        };
        let s = arm();
        let skin = MeshSkin::new(s.shared_joints().clone(), vec![0, 1, 1], vec![Mat4::IDENTITY; 3], &data);
        assert_eq!(skin.bounds[0], (vec3(0.0, 1.0, 0.0), 1.0));
        assert_eq!(skin.bounds[1], (vec3(0.0, 3.0, 0.0), 1.0));
        assert!(skin.bounds[2].1 < 0.0, "a slot nothing is weighted to");
    }

    /// An assets table with mesh 0: a two-corner "stick" skinned to `arm`
    /// (one corner on each joint).
    fn stick(s: &Skeleton) -> Assets3D {
        let data = MeshData {
            positions: vec![Vec3::ZERO, vec3(0.0, 1.0, 0.0), Vec3::ZERO],
            indices: vec![0, 1, 2],
            joints: vec![[0, 0, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0]],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
            ..Default::default()
        };
        let inverse = (0..2).map(|j| s.joint_matrix(j).inverse()).collect();
        let skin = MeshSkin::new(s.shared_joints().clone(), vec![0, 1], inverse, &data);
        Assets3D {
            bounds: vec![data.bounds()],
            skins: vec![Some(Arc::new(skin))],
            ..Default::default()
        }
    }

    #[test]
    fn skinned_models_are_posed_bound_by_name_and_carry_attachments() {
        let mut world = World::new();
        let mut s = arm();
        let assets = stick(&s);
        s.pose[0].rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let mesh = super::super::Mesh { id: 0 };
        let at = Transform3D::at(vec3(10.0, 0.0, 0.0));
        let body = world.spawn((at, Model::new(mesh), s));
        // The same joint names, made separately: bound by name.
        let mut other = arm();
        other.pose[1].position.y = 3.0;
        let other = world.spawn((Transform3D::default(), other));
        world.spawn((Model::new(mesh), SkinOf(other)));
        let cup = world.spawn((
            Transform3D::default(),
            AttachToJoint {
                skeleton: body,
                joint: 1,
                offset: Transform3D::at(vec3(0.0, 0.5, 0.0)),
            },
        ));
        let mut frame = SkinFrame::default();
        frame.collect(&mut world, &assets);
        assert_eq!(frame.draws.len(), 2);
        assert_eq!(frame.matrices.len(), 4, "two slots each");
        // The tip corner, bent 90 degrees: from (0, 1) to (-1, 0).
        let tip = frame.matrices[1].transform_point3(vec3(0.0, 1.0, 0.0));
        assert!(tip.abs_diff_eq(vec3(-1.0, 0.0, 0.0), 1e-5), "{tip}");
        let (center, radius) = frame.draws[0].bounds;
        assert!(center.distance(tip) <= radius + 1e-5 && center.length() <= radius + 1e-5);
        assert_eq!(frame.draws[0].matrix, at.matrix());
        // The other skeleton stretched its tip to y = 3.
        let far = frame.matrices[3].transform_point3(vec3(0.0, 1.0, 0.0));
        assert!(far.abs_diff_eq(vec3(0.0, 3.0, 0.0), 1e-5));
        // The cup: on the bent tip, then half a unit along the tip's up (now -x).
        let cup_at = world.get::<Transform3D>(cup).unwrap().position;
        assert!(cup_at.abs_diff_eq(vec3(8.5, 0.0, 0.0), 1e-5), "{cup_at}");
    }

    #[test]
    fn posed_models_are_not_also_drawn_as_they_were_modelled() {
        let mut world = World::new();
        let s = arm();
        let assets = stick(&s);
        let mesh = super::super::Mesh { id: 0 };
        world.spawn((Transform3D::default(), Model::new(mesh), s));
        // A skinned mesh without a skeleton draws as modelled.
        world.spawn((Transform3D::default(), Model::new(mesh)));
        let mut skins = SkinFrame::default();
        skins.collect(&mut world, &assets);
        let mut frame = super::super::Frame3D::default();
        let mut camera = super::super::Camera3D::new(glam::Vec2::new(100.0, 100.0));
        camera.position = vec3(0.0, 0.5, 5.0);
        camera.look_at(vec3(0.0, 0.5, 0.0));
        let lighting = super::super::Lighting {
            shadows: false,
            ..Default::default()
        };
        frame.collect(&mut world, &assets, &skins, &camera, &lighting);
        assert_eq!(
            frame.runs,
            [(0, 0, 1), (1, 1, 1)],
            "mesh 0 once, skinned model 0 as mesh 1"
        );
    }

    #[test]
    #[should_panic(expected = "doesn't come before it")]
    fn parents_come_first() {
        let _ = Skeleton::new(vec![Joint {
            name: "loop".into(),
            parent: Some(0),
            rest: Transform3D::default(),
        }]);
    }
}
