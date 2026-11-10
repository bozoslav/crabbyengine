//! Skinned meshes: a [`Skeleton`] of joints, posed every frame, bends the
//! meshes bound to it (characters, animals, hands).

use std::sync::Arc;

use glam::{Mat4, Vec3};

use super::collect::Instance3D;
use super::{Assets3D, MeshData, Model, ModelMaterial, ModelMaterials, Transform3D, linear};
use crate::{Entity, World};

/// One joint of a [`Skeleton`]: its name, the joint it hangs from, and
/// where it sits relative to that joint when nothing moves it (its rest
/// pose).
///
/// ```
/// # use ponko::*;
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
/// [`Graphics::load_skeleton`](crate::Graphics::load_skeleton), or made in code with
/// [`Skeleton::new`]. Cloning one is cheap: the joints are shared.
///
/// A skinned model without a skeleton draws in the pose it was modelled in.
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// let body = ctx.gfx.load_mesh("models/fox.glb");
/// let skeleton = ctx.gfx.load_skeleton("models/fox.glb");
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
    pub(crate) morphs: MorphPose,
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
    /// # use ponko::*;
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
            morphs: MorphPose::default(),
            pose,
            matrices: Vec::new(),
        };
        s.update();
        s
    }

    /// The joints, in order (parents before children).
    ///
    /// ```
    /// # use ponko::*;
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
    /// # use ponko::*;
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
    /// # use ponko::*;
    /// let mut s = Skeleton::new(vec![Joint { name: "hip".into(), parent: None, rest: Transform3D::default() }]);
    /// s.pose[0].position.y = 3.0;
    /// s.reset_pose();
    /// assert_eq!(s.pose[0], Transform3D::default());
    /// ```
    pub fn reset_pose(&mut self) {
        self.morphs.clear();
        self.pose.clear();
        self.pose.extend(self.joints.iter().map(|j| j.rest));
    }

    /// Writes sampled morph channels by name, preserving all unrelated weights.
    /// Call after manual clip sampling or Animator::update. Automatic animators
    /// write the model's component themselves, including SkinOf attachments.
    /// ```
    /// use ponko::*;
    /// let s = Skeleton::new(vec![]);
    /// let mut weights = MorphWeights(vec![0.5]);
    /// s.write_morph_weights(&["smile".into()], &mut weights);
    /// assert_eq!(weights.get(0), 0.5);
    /// ```
    pub fn write_morph_weights(&self, names: &[String], weights: &mut MorphWeights) {
        self.morphs.write(names, weights);
    }

    /// Joint `joint`'s matrix in the entity's own space, from `pose` as it
    /// is now: where a hand is, to put a cup in it. Multiply by the entity's
    /// [`Transform3D::matrix`] for the world.
    ///
    /// ```
    /// # use ponko::*;
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
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// let body = ctx.gfx.load_mesh("people/body.glb");
/// let hat = ctx.gfx.load_mesh("people/hat.glb");
/// let skeleton = ctx.gfx.load_skeleton("people/body.glb");
/// let person = ctx.world.spawn((Transform3D::default(), Model::new(body), skeleton));
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
/// # use ponko::*;
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

/// How much of each of its mesh's [morph targets](crate::MeshData::morph_targets)
/// a [`Model`] shows: 0 none, 1 all of it (more or less than that
/// exaggerates or inverts). By target index; missing ones are 0. A model
/// without this component shows its mesh's own weights (from its glTF
/// file, else none). Animation clips with weight tracks set it.
///
/// ```
/// # use ponko::*;
/// Game::new("t", 1, 1).run_headless(0, |ctx| {
///     let mut quad = MeshData::plane(1.0);
///     let mut up = vec![Vec3::ZERO; quad.positions.len()];
///     up[0] = vec3(0.0, 0.5, 0.0);
///     quad.morph_targets.push(MorphTarget { name: "raise".into(), positions: up, normals: Vec::new() });
///     let material = ctx.gfx.default_material();
///     let mesh = ctx.gfx.create_mesh(&quad, material);
///     let raise = ctx.gfx.morph_target(mesh, "raise").unwrap();
///     let mut weights = MorphWeights::default();
///     weights.set(raise, 0.5); // halfway there
///     ctx.world.spawn((Transform3D::default(), Model::new(mesh), weights));
/// });
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MorphWeights(pub Vec<f32>);

impl MorphWeights {
    /// Target `target`'s weight (0 when not set).
    /// ```
    /// # use ponko::*;
    /// assert_eq!(MorphWeights(vec![0.25]).get(0), 0.25);
    /// assert_eq!(MorphWeights::default().get(3), 0.0);
    /// ```
    #[must_use]
    pub fn get(&self, target: usize) -> f32 {
        self.0.get(target).copied().unwrap_or(0.0)
    }

    /// Sets target `target`'s weight, growing the list as needed.
    /// ```
    /// # use ponko::*;
    /// let mut w = MorphWeights::default();
    /// w.set(2, 1.0);
    /// assert_eq!(w.0, [0.0, 0.0, 1.0]);
    /// ```
    pub fn set(&mut self, target: usize, weight: f32) {
        if self.0.len() <= target {
            self.0.resize(target + 1, 0.0);
        }
        self.0[target] = weight;
    }
}

/// What the CPU keeps about a mesh's morph targets: their names, the
/// weights it shows by default, and the farthest all of them together
/// move a corner (for culling).
#[derive(Debug)]
pub(crate) struct MeshMorph {
    pub names: Vec<String>,
    pub defaults: Vec<f32>,
    pub reach: f32,
    pub frames: Vec<super::MorphFrame>,
}

impl MeshMorph {
    pub fn new(data: &MeshData, defaults: Vec<f32>) -> Option<Self> {
        if data.morph_targets.is_empty() {
            return None;
        }
        let reach = (0..data.positions.len())
            .map(|i| data.morph_targets.iter().map(|t| t.positions[i].length()).sum::<f32>())
            .fold(0.0, f32::max);
        Some(Self {
            names: data.morph_targets.iter().map(|t| t.name.clone()).collect(),
            defaults,
            reach,
            frames: data.morph_frames.clone(),
        })
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
    /// The model's entity (bits), to find its motion from last frame.
    pub entity: u64,
    /// The model matrix, and its largest scale.
    pub matrix: Mat4,
    pub scale: f32,
    /// A sphere around the posed mesh, in model space; negative radius:
    /// nothing to draw.
    pub bounds: (Vec3, f32),
    /// Its first skin matrix in `SkinFrame::matrices` (one per slot).
    pub first: u32,
    /// Its first morph weight in `SkinFrame::weights`, and how many (its
    /// mesh's morph targets; 0 for none).
    pub morph_first: u32,
    pub morph_count: u32,
    /// The material every part is drawn with (a `ModelMaterial`), or
    /// `u32::MAX`: each part's own.
    pub material: u32,
    /// Its per-part materials (a `ModelMaterials`), which win over
    /// `material`: where they start in `SkinFrame::part_materials`, and
    /// how many.
    pub parts: (u32, u32),
}

/// This frame's skinned models and their skin matrices: the joints' matrices
/// times each slot's inverse bind matrix. Target cameras draw the same.
#[derive(Default)]
pub(crate) struct SkinFrame {
    pub draws: Vec<SkinDraw>,
    pub matrices: Vec<Mat4>,
    /// This frame's morph weights, `morph_count` per draw.
    pub weights: Vec<f32>,
    /// `(part, material)` lists of the draws with per-part materials, each
    /// sorted by part.
    pub part_materials: Vec<(u32, u32)>,
    attached: Vec<(Entity, AttachToJoint)>,
    moved: Vec<(Entity, Transform3D)>,
    shared: Vec<(Model, Entity, Entity)>,
}

impl SkinFrame {
    /// Poses every skeleton, moves what's attached to joints, and lists the
    /// skinned models.
    pub fn collect(&mut self, world: &mut World, assets: &Assets3D) {
        self.draws.clear();
        self.matrices.clear();
        self.weights.clear();
        self.part_materials.clear();
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

        let out = &mut Out {
            draws: &mut self.draws,
            matrices: &mut self.matrices,
            weights: &mut self.weights,
            part_materials: &mut self.part_materials,
        };
        world
            .query::<(
                Entity,
                &Transform3D,
                &Model,
                &Skeleton,
                Option<&MorphWeights>,
                Option<&super::ReceivesDecals>,
                Option<&ModelMaterial>,
                Option<&ModelMaterials>,
            )>()
            .for_each(|(e, t, m, s, w, r, material, parts)| {
                // Unskinned meshes go with the next query, skeleton or not.
                if assets.skin(m.mesh.id).is_some() {
                    add(out, assets, e, t, m, Some(s), w, r, (material, parts));
                }
            });
        // Meshes that change shape but don't bend: one identity matrix.
        world
            .query::<(
                Entity,
                &Transform3D,
                &Model,
                Option<&MorphWeights>,
                Option<&super::ReceivesDecals>,
                Option<&ModelMaterial>,
                Option<&ModelMaterials>,
            )>()
            .without::<SkinOf>()
            .for_each(|(e, t, m, w, r, material, parts)| {
                if assets.skin(m.mesh.id).is_none() && assets.morph(m.mesh.id).is_some() {
                    add(out, assets, e, t, m, None, w, r, (material, parts));
                }
            });
        self.shared.clear();
        world
            .query::<(Entity, &Model, &SkinOf)>()
            .for_each(|(e, m, of)| self.shared.push((*m, of.0, e)));
        for (m, of, e) in &self.shared {
            if let (Some(t), Some(s)) = (world.get::<Transform3D>(*of), world.get::<Skeleton>(*of)) {
                add(
                    out,
                    assets,
                    *e,
                    t,
                    m,
                    Some(s),
                    world.get::<MorphWeights>(*e),
                    world.get::<super::ReceivesDecals>(*e),
                    (world.get::<ModelMaterial>(*e), world.get::<ModelMaterials>(*e)),
                );
            }
        }
    }
}

/// Where [`add`] puts a model.
struct Out<'a> {
    draws: &'a mut Vec<SkinDraw>,
    matrices: &'a mut Vec<Mat4>,
    weights: &'a mut Vec<f32>,
    part_materials: &'a mut Vec<(u32, u32)>,
}

/// Adds model `m`, posed by `s` if its mesh is skinned, shaped by `w` if
/// it has morph targets, when its mesh is live.
#[allow(clippy::too_many_arguments)]
fn add(
    out: &mut Out,
    assets: &Assets3D,
    entity: Entity,
    t: &Transform3D,
    m: &Model,
    s: Option<&Skeleton>,
    w: Option<&MorphWeights>,
    receives: Option<&super::ReceivesDecals>,
    (material, parts): (Option<&ModelMaterial>, Option<&ModelMaterials>),
) {
    let Some(mesh_bounds) = assets.live_bounds(m.mesh.id) else {
        return;
    };
    let first = out.matrices.len() as u32;
    let morph = assets.morph(m.mesh.id);
    // How far the shape can move corners this frame.
    let (morph_first, morph_count, reach) = match morph {
        Some(morph) => {
            let at = out.weights.len();
            let own = w.map_or(&morph.defaults[..], |w| &w.0[..]);
            let n = morph.names.len();
            out.weights
                .extend((0..n).map(|k| own.get(k).copied().filter(|v| v.is_finite()).unwrap_or(0.0)));
            expand_frames(&morph.frames, &mut out.weights[at..]);
            let most = out.weights[at..].iter().fold(0.0f32, |a, v| a.max(v.abs()));
            (at as u32, n as u32, morph.reach * most)
        }
        None => (0, 0, 0.0),
    };
    let bounds = match (assets.skin(m.mesh.id), s) {
        (Some(skin), Some(s)) => {
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            let mut spheres = 0;
            let joints = s.matrices();
            for slot in 0..skin.slots.len() {
                let skin_matrix = match skin.joint_in(s, slot).and_then(|j| joints.get(j)) {
                    Some(joint) => *joint * skin.inverse_binds[slot],
                    None => Mat4::IDENTITY, // no such joint: as modelled
                };
                out.matrices.push(skin_matrix);
                let (center, radius) = skin.bounds[slot];
                if radius >= 0.0 {
                    let c = skin_matrix.transform_point3(center);
                    let r = (radius + reach) * max_scale(skin_matrix);
                    lo = lo.min(c - r);
                    hi = hi.max(c + r);
                    spheres += 1;
                }
            }
            if spheres == 0 {
                (Vec3::ZERO, -1.0)
            } else {
                ((lo + hi) / 2.0, (hi - lo).length() / 2.0)
            }
        }
        (None, _) if morph.is_some() => {
            out.matrices.push(Mat4::IDENTITY);
            (mesh_bounds.0, mesh_bounds.1 + reach)
        }
        _ => return,
    };
    let matrix = t.matrix();
    let mut model = matrix.to_cols_array_2d();
    model[0][3] = super::projected::receiver_mask(receives);
    out.draws.push(SkinDraw {
        mesh: m.mesh.id,
        instance: Instance3D {
            model,
            color: linear(m.color),
        },
        entity: entity.to_bits(),
        matrix,
        scale: t.scale.abs().max_element(),
        bounds,
        first,
        morph_first,
        morph_count,
        material: material.map_or(u32::MAX, |m| m.0.id),
        parts: match parts.map(ModelMaterials::raw) {
            Some(list) if !list.is_empty() => {
                let start = out.part_materials.len() as u32;
                out.part_materials.extend(list.iter().map(|&(part, m)| (part, m.id)));
                (start, list.len() as u32)
            }
            _ => (0, 0),
        },
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
    fn skinning_scratch_follows_a_same_id_world_load() {
        use crate::WorldReflect;
        let mut world = World::new();
        world.reflect::<Transform3D>();
        let mut bent = arm();
        let assets = stick(&bent);
        bent.pose[0].rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let mesh = super::super::Mesh { id: 0 };
        let body = world.spawn((Transform3D::default(), Model::new(mesh), bent));
        let cup = world.spawn((
            Transform3D::default(),
            AttachToJoint {
                skeleton: body,
                joint: 1,
                offset: Transform3D::default(),
            },
        ));
        let mut frame = SkinFrame::default();
        frame.collect(&mut world, &assets);
        assert_eq!(frame.draws.len(), 1);
        // The next scene: the same ids, the body moved, skeletons and joint
        // attachments not saved (they aren't reflected).
        world.get_mut::<Transform3D>(body).unwrap().position = vec3(5.0, 0.0, 0.0);
        let next = world.save();
        world.load(&next).unwrap();
        frame.collect(&mut world, &assets);
        assert!(
            frame.draws.is_empty() && frame.matrices.is_empty(),
            "nothing of the old pose is drawn"
        );
        // The game adds an unbent skeleton to the same id.
        world.insert(body, Model::new(mesh));
        world.insert(body, arm());
        world.insert(
            cup,
            AttachToJoint {
                skeleton: body,
                joint: 1,
                offset: Transform3D::default(),
            },
        );
        frame.collect(&mut world, &assets);
        assert_eq!(frame.draws.len(), 1);
        assert_eq!(frame.draws[0].matrix, Transform3D::at(vec3(5.0, 0.0, 0.0)).matrix());
        let tip = frame.matrices[1].transform_point3(vec3(0.0, 1.0, 0.0));
        assert!(
            tip.abs_diff_eq(vec3(0.0, 1.0, 0.0), 1e-5),
            "the new pose, not the old one: {tip}"
        );
        let cup_at = world.get::<Transform3D>(cup).unwrap().position;
        assert!(
            cup_at.abs_diff_eq(vec3(5.0, 1.0, 0.0), 1e-5),
            "on the new skeleton's tip: {cup_at}"
        );
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

    /// An assets table with mesh 0: an unskinned triangle with one morph
    /// target that lifts its first corner by 2, shown at 0.25 by default.
    fn lifting_triangle() -> Assets3D {
        let mut data = MeshData {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            indices: vec![0, 1, 2],
            ..Default::default()
        };
        data.morph_targets.push(crate::MorphTarget {
            name: "lift".into(),
            positions: vec![vec3(0.0, 0.0, 2.0), Vec3::ZERO, Vec3::ZERO],
            normals: Vec::new(),
        });
        Assets3D {
            bounds: vec![data.bounds()],
            morphs: vec![MeshMorph::new(&data, vec![0.25]).map(Arc::new)],
            ..Default::default()
        }
    }

    #[test]
    fn morph_reach_is_the_farthest_a_corner_moves() {
        assert_eq!(lifting_triangle().morph(0).unwrap().reach, 2.0);
        assert!(MeshMorph::new(&MeshData::cube(1.0), Vec::new()).is_none());
    }

    #[test]
    fn morphing_models_take_their_weights_or_their_meshes_defaults() {
        let mut world = World::new();
        let assets = lifting_triangle();
        let mesh = super::super::Mesh { id: 0 };
        world.spawn((Transform3D::default(), Model::new(mesh)));
        world.spawn((Transform3D::default(), Model::new(mesh), MorphWeights(vec![1.0, 7.0])));
        world.spawn((Transform3D::default(), Model::new(mesh), MorphWeights(vec![f32::NAN])));
        let mut frame = SkinFrame::default();
        frame.collect(&mut world, &assets);
        assert_eq!(frame.draws.len(), 3);
        assert_eq!(frame.matrices, [Mat4::IDENTITY; 3], "one still matrix each");
        // One weight per target: extra weights are dropped, bad ones are 0.
        assert_eq!(frame.weights, [0.25, 1.0, 0.0]);
        for (k, d) in frame.draws.iter().enumerate() {
            assert_eq!((d.first, d.morph_first, d.morph_count), (k as u32, k as u32, 1));
        }
        // Bounds grow by how far this model's weights can move a corner.
        let (rest_center, rest_radius) = assets.live_bounds(0).unwrap();
        let radius = |k: usize| {
            let (c, r) = frame.draws[k].bounds;
            assert!(c.abs_diff_eq(rest_center, 1e-6));
            r
        };
        let lifted = Vec3::Z * 2.0;
        assert!(
            rest_center.distance(lifted) <= radius(1) + 1e-5,
            "fully lifted corner inside"
        );
        assert!((radius(0) - (rest_radius + 0.5)).abs() < 1e-6);
        assert!((radius(1) - (rest_radius + 2.0)).abs() < 1e-6);
        assert!((radius(2) - rest_radius).abs() < 1e-6, "no weight, no growth");
    }

    #[test]
    fn a_freed_morphing_mesh_draws_nothing() {
        let mut world = World::new();
        let mut assets = lifting_triangle();
        assets.bounds[0] = (Vec3::ZERO, -1.0);
        world.spawn((Transform3D::default(), Model::new(super::super::Mesh { id: 0 })));
        let mut frame = SkinFrame::default();
        frame.collect(&mut world, &assets);
        assert!(frame.draws.is_empty() && frame.weights.is_empty());
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

/// Sparse affine pose: value + (1 - coverage) * current entity weight.
#[derive(Debug, Clone, Default)]
pub(crate) struct MorphPose(Vec<(Arc<str>, f32, f32, bool)>);
impl MorphPose {
    pub fn clear(&mut self) {
        for (_, v, coverage, active) in &mut self.0 {
            *v = 0.;
            *coverage = 0.;
            *active = false;
        }
    }
    pub fn set(&mut self, name: &str, value: f32, weight: f32) {
        if let Some((_, v, c, active)) = self.0.iter_mut().find(|(n, _, _, _)| &**n == name) {
            *v += (value - *v) * weight;
            *c += (1. - *c) * weight;
            *active = true;
        } else {
            self.0.push((Arc::from(name), value * weight, weight, true));
        }
    }
    pub fn mix(&mut self, source: &Self, weight: f32) {
        for (_, v, c, active) in &mut self.0 {
            if *active {
                *v *= 1. - weight;
                *c *= 1. - weight;
            }
        }
        for (name, value, coverage, active) in &source.0 {
            if !active {
                continue;
            }
            if let Some((_, v, c, a)) = self.0.iter_mut().find(|(n, _, _, _)| n == name) {
                *v += value * weight;
                *c += coverage * weight;
                *a = true;
            } else {
                self.0.push((name.clone(), value * weight, coverage * weight, true));
            }
        }
    }
    pub fn layer(&mut self, source: &Self, weight: f32, additive: bool) {
        for (name, value, coverage, active) in &source.0 {
            if !active {
                continue;
            }
            if let Some((_, v, c, a)) = self.0.iter_mut().find(|(n, _, _, _)| n == name) {
                if additive {
                    *v += value * weight;
                } else {
                    *v = *v * (1. - coverage * weight) + value * weight;
                    *c += (1. - *c) * coverage * weight;
                }
                *a = true;
            } else {
                self.0.push((
                    name.clone(),
                    value * weight,
                    if additive { 0. } else { coverage * weight },
                    true,
                ));
            }
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.0.iter().any(|(_, _, _, active)| *active)
    }
    pub fn write_base(&self, names: &[String], base: &MorphWeights, weights: &mut MorphWeights) {
        for (name, value, coverage, active) in &self.0 {
            if *active && let Some(i) = names.iter().position(|n| n == &**name) {
                weights.set(i, value + base.get(i) * (1. - coverage));
            }
        }
    }
    pub fn write(&self, names: &[String], weights: &mut MorphWeights) {
        for (name, value, coverage, active) in &self.0 {
            if *active && let Some(i) = names.iter().position(|n| n == &**name) {
                weights.set(i, value + weights.get(i) * (1. - coverage));
            }
        }
    }
}

/// Expand a channel to at most two active frame targets; extrapolate outside
/// the authored range, including negative weights, using the end segment.
fn expand_frames(frames: &[super::MorphFrame], weights: &mut [f32]) {
    for (i, frame) in frames.iter().enumerate() {
        if frames[..i].iter().any(|f| f.channel == frame.channel) {
            continue;
        }
        let value = weights[frame.channel];
        let first = frames
            .iter()
            .filter(|f| f.channel == frame.channel)
            .min_by(|a, b| a.weight.total_cmp(&b.weight))
            .unwrap();
        let last = frames
            .iter()
            .filter(|f| f.channel == frame.channel)
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
            .unwrap();
        let hi = frames
            .iter()
            .filter(|f| f.channel == frame.channel && f.weight >= value)
            .min_by(|a, b| a.weight.total_cmp(&b.weight))
            .unwrap_or(last);
        let lo = frames
            .iter()
            .filter(|f| f.channel == frame.channel && f.weight < hi.weight)
            .max_by(|a, b| a.weight.total_cmp(&b.weight));
        weights[frame.channel] = 0.;
        for f in frames.iter().filter(|f| f.channel == frame.channel) {
            weights[f.target] = 0.;
        }
        if let Some(lo) = lo.filter(|_| value > first.weight) {
            let t = (value - lo.weight) / (hi.weight - lo.weight);
            weights[lo.target] = 1. - t;
            weights[hi.target] = t;
        } else {
            weights[first.target] = value / first.weight;
        }
    }
}

#[cfg(test)]
mod morph_frame_tests {
    use super::*;
    use crate::MorphFrame;
    #[test]
    fn frames_interpolate_from_zero_and_between_authored_shapes() {
        let frames = [
            MorphFrame {
                channel: 0,
                target: 1,
                weight: 0.5,
            },
            MorphFrame {
                channel: 0,
                target: 0,
                weight: 1.,
            },
        ];
        for (weight, expected) in [
            (0., [0., 0.]),
            (0.25, [0., 0.5]),
            (0.5, [0., 1.]),
            (0.75, [0.5, 0.5]),
            (1., [1., 0.]),
            (1.5, [2., -1.]),
            (-0.5, [0., -1.]),
        ] {
            let mut weights = [weight, 99., 0.3];
            expand_frames(&frames, &mut weights);
            assert_eq!(weights, [expected[0], expected[1], 0.3]);
        }
    }
}
