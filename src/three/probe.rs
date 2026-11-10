//! Local, axis-aligned reflection volumes. Captures are explicit, never per-frame.
use crate::gfx::Graphics;
use crate::{Entity, Texture, Transform3D, Vec3, World};

/// A local environment at an entity's [`Transform3D::position`]. Rotation and
/// scale are ignored. Four baked environments can be resident; zero content
/// falls back to sky lighting. Capture excludes transparent objects.
/// ```
/// use ponko::*;
/// let probe = ReflectionProbe3D { size: vec3(12.0, 4.0, 8.0), ..Default::default() };
/// assert_eq!(probe.intensity, 1.0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ReflectionProbe3D {
    /// Full axis-aligned box size in world units. Default 10 on each axis.
    pub size: Vec3,
    /// Fade width inside the box. Default 1; zero gives a hard boundary.
    pub blend_distance: f32,
    /// Multiplier on captured linear radiance. Default 1.
    pub intensity: f32,
    /// Opaque environment index assigned by the context; zero means unbaked.
    pub content: u32,
    /// Correct reflections for the box's walls. Default true.
    pub box_projection: bool,
}
impl Default for ReflectionProbe3D {
    fn default() -> Self {
        Self {
            size: Vec3::splat(10.0),
            blend_distance: 1.0,
            intensity: 1.0,
            content: 0,
            box_projection: true,
        }
    }
}
impl ReflectionProbe3D {
    /// Influence at `point`, fading to zero at the boundary.
    /// ```
    /// use ponko::*;
    /// assert_eq!(ReflectionProbe3D::default().weight(Vec3::ZERO, Vec3::ZERO), 1.0);
    /// ```
    pub fn weight(&self, center: Vec3, point: Vec3) -> f32 {
        if !self.size.is_finite()
            || self.size.min_element() <= 0.0
            || !self.blend_distance.is_finite()
            || !self.intensity.is_finite()
            || self.intensity <= 0.0
            || !center.is_finite()
            || !point.is_finite()
        {
            return 0.0;
        }
        let edge = (self.size * 0.5 - (point - center).abs()).min_element();
        if edge <= 0.0 {
            0.0
        } else if self.blend_distance <= 0.0 {
            1.0
        } else {
            (edge / self.blend_distance).min(1.0)
        }
    }
    /// Direction from the capture center to the ray's exit from this box.
    /// Returns the input direction outside the box or when correction is off.
    /// ```
    /// use ponko::*;
    /// let p = ReflectionProbe3D::default();
    /// assert_eq!(p.project(Vec3::ZERO, Vec3::ZERO, Vec3::Z), Vec3::Z * 5.0);
    /// ```
    pub fn project(&self, center: Vec3, point: Vec3, direction: Vec3) -> Vec3 {
        if !self.box_projection || self.weight(center, point) == 0.0 || !direction.is_finite() {
            return direction;
        }
        let local = point - center;
        let mut distance = f32::INFINITY;
        for axis in 0..3 {
            let d = direction[axis];
            if d.abs() > 1e-7 {
                distance = distance.min((self.size[axis] * 0.5 * d.signum() - local[axis]) / d);
            }
        }
        if distance.is_finite() {
            local + direction * distance
        } else {
            direction
        }
    }
}

#[derive(Default)]
pub(crate) struct ProbeAssets {
    pub captures: Vec<[Texture; 6]>,
    pub pending: Vec<(u32, Vec3)>,
    pub uploads: Vec<(u32, std::sync::Arc<Vec<u16>>)>,
    /// What the frame being drawn took from the two lists above, kept until
    /// it is known to have been drawn.
    pub in_flight: Vec<(u32, Vec3)>,
    pub in_flight_uploads: Vec<(u32, std::sync::Arc<Vec<u16>>)>,
}

impl ProbeAssets {
    /// Takes this frame's requests (see `Ctx::collect_views`).
    pub fn take_requests(&mut self) {
        self.in_flight.clear();
        self.in_flight_uploads.clear();
        self.in_flight.append(&mut self.pending);
        self.in_flight_uploads.append(&mut self.uploads);
    }
    /// After a frame: if it was drawn the requests are done; if not (hidden
    /// window), ask again next frame, unless the game has asked for that
    /// environment again since.
    pub fn settle(&mut self, drawn: bool) {
        if drawn {
            self.in_flight.clear();
            self.in_flight_uploads.clear();
            return;
        }
        for (slot, center) in std::mem::take(&mut self.in_flight) {
            let asked = self.pending.iter().any(|&(s, _)| s == slot) || self.uploads.iter().any(|(s, _)| *s == slot);
            if !asked {
                self.pending.push((slot, center));
            }
        }
        for (slot, data) in std::mem::take(&mut self.in_flight_uploads) {
            let asked = self.pending.iter().any(|&(s, _)| s == slot) || self.uploads.iter().any(|(s, _)| *s == slot);
            if !asked {
                self.uploads.push((slot, data));
            }
        }
    }
}

impl Graphics {
    /// Queue six 128² HDR scene views and GGX filtering on the next rendered
    /// frame, for the probe on `entity` of `world` (the game's is `ctx.world`). Calling again refreshes the same content. At most four captures
    /// can be allocated in a context. Headless simulation queues no GPU work.
    /// ```
    /// use ponko::*;
    /// Game::new("probe", 1, 1).run_headless(0, |ctx| {
    ///     let e = ctx.world.spawn((Transform3D::default(), ReflectionProbe3D::default()));
    ///     ctx.gfx.bake_reflection_probe(&mut ctx.world, e).unwrap();
    /// });
    /// ```
    pub fn bake_reflection_probe(&mut self, world: &mut World, entity: Entity) -> Result<(), String> {
        let probe = *world
            .get::<ReflectionProbe3D>(entity)
            .ok_or("entity has no reflection probe")?;
        let center = world
            .get::<Transform3D>(entity)
            .ok_or("probe has no transform")?
            .position;
        if probe.weight(center, center) == 0.0 {
            return Err("probe has invalid volume or intensity".into());
        }
        let slot = if probe.content == 0 {
            if self.assets3d.probes.captures.len() == 4 {
                return Err("four reflection environments are already allocated".into());
            }
            let faces = std::array::from_fn(|_| {
                self.make_render_target(128, 128)
                    .unwrap_or_else(|e| panic!("ponko: {e}"))
            });
            self.assets3d.probes.captures.push(faces);
            let slot = self.assets3d.probes.captures.len() as u32;
            world.get_mut::<ReflectionProbe3D>(entity).unwrap().content = slot;
            slot
        } else {
            if probe.content as usize > self.assets3d.probes.captures.len() {
                return Err("invalid reflection content".into());
            }
            probe.content
        };
        self.assets3d.probes.pending.retain(|&(s, _)| s != slot);
        self.assets3d.probes.pending.push((slot, center));
        Ok(())
    }
}

/// Packed frame metadata; also the shader uniform layout.
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ProbeUniform {
    pub center: [f32; 4],
    pub size: [f32; 4],
    pub params: [f32; 4],
}

impl Graphics {
    /// Load an HDR/EXR latitude-longitude image, from the [added asset
    /// sources](crate::Assets::add_source) or `assets/`, as the baked content
    /// of the probe on `entity` of `world` (the game's is `ctx.world`).
    /// Pixels are linear radiance, with +Y at the top and +X at the center.
    /// The file must have a 2:1 aspect ratio. Filtering runs next rendered frame.
    /// ```no_run
    /// # use ponko::*;
    /// # fn load(ctx: &mut Ctx, probe: Entity) -> Result<(), String> {
    /// ctx.gfx.load_reflection_probe(&mut ctx.world, probe, "interior.exr")?;
    /// # Ok(()) }
    /// ```
    pub fn load_reflection_probe(&mut self, world: &mut World, entity: Entity, name: &str) -> Result<(), String> {
        let found = self.asset_sources.find(name)?;
        let origin = found.origin.clone();
        let bytes = found.into_bytes()?;
        let image = image::load_from_memory(&bytes)
            .map_err(|e| format!("{} is not a valid image: {e}", origin.describe(name)))?
            .into_rgb32f();
        let (width, height) = image.dimensions();
        if width != height.saturating_mul(2) || height == 0 {
            return Err("reflection image must have 2:1 aspect ratio".into());
        }
        let mut pixels = Vec::with_capacity(768 * 128 * 4);
        for y in 0..128 {
            for x in 0..768 {
                let d = cube_direction(
                    x / 128,
                    (x % 128) as f32 / 128.0 + 0.5 / 128.0,
                    y as f32 / 128.0 + 0.5 / 128.0,
                );
                let u = (d.z.atan2(d.x) / std::f32::consts::TAU + 0.5) * width as f32 - 0.5;
                let v = (0.5 - d.y.asin() / std::f32::consts::PI) * height as f32 - 0.5;
                let ix = u.floor() as i32;
                let iy = v.floor() as i32;
                let sample = |x: i32, y: i32| {
                    Vec3::from_array(
                        image
                            .get_pixel(x.rem_euclid(width as i32) as u32, y.clamp(0, height as i32 - 1) as u32)
                            .0,
                    )
                };
                let c = sample(ix, iy).lerp(sample(ix + 1, iy), u.fract().rem_euclid(1.0)).lerp(
                    sample(ix, iy + 1).lerp(sample(ix + 1, iy + 1), u.fract().rem_euclid(1.0)),
                    v.fract().rem_euclid(1.0),
                );
                if !c.is_finite() {
                    return Err("reflection image contains non-finite radiance".into());
                }
                pixels.extend(
                    c.clamp(Vec3::ZERO, Vec3::splat(65504.0))
                        .to_array()
                        .map(|v| half::f16::from_f32(v).to_bits()),
                );
                pixels.push(half::f16::ONE.to_bits());
            }
        }
        self.bake_reflection_probe(world, entity)?;
        let slot = world.get::<ReflectionProbe3D>(entity).unwrap().content;
        self.assets3d.probes.pending.retain(|&(s, _)| s != slot);
        self.assets3d.probes.uploads.retain(|(s, _)| *s != slot);
        self.assets3d.probes.uploads.push((slot, std::sync::Arc::new(pixels)));
        Ok(())
    }
}

pub(crate) fn cube_direction(face: u32, u: f32, v: f32) -> Vec3 {
    let x = u * 2.0 - 1.0;
    let y = v * 2.0 - 1.0;
    match face {
        0 => Vec3::new(1.0, -y, -x),
        1 => Vec3::new(-1.0, -y, x),
        2 => Vec3::new(x, 1.0, y),
        3 => Vec3::new(x, -1.0, -y),
        4 => Vec3::new(x, -y, 1.0),
        _ => Vec3::new(-x, -y, -1.0),
    }
    .normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_that_was_not_drawn_hands_its_requests_to_the_next() {
        let mut probes = ProbeAssets::default();
        probes.pending.push((1, Vec3::X));
        probes.uploads.push((2, std::sync::Arc::new(vec![0])));
        probes.take_requests();
        assert!(probes.pending.is_empty() && probes.uploads.is_empty());
        probes.pending.push((1, Vec3::Y)); // asked again meanwhile
        probes.settle(false);
        assert_eq!(probes.pending, vec![(1, Vec3::Y)]);
        assert_eq!(probes.uploads.len(), 1);
        probes.take_requests();
        probes.settle(true);
        assert!(probes.in_flight.is_empty() && probes.in_flight_uploads.is_empty());
        probes.settle(false);
        assert!(probes.pending.is_empty());
    }
}
