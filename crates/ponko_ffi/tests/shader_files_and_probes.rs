//! Shader, effect and compute shader files, and reflection probes, from C.
//! Shaders are validated at load time without a GPU, and probes bake into
//! headless render targets, so both run here.
use ponko_ffi::*;
use std::ffi::{CString, c_void};
use std::path::PathBuf;
use std::ptr::null_mut;

use CrabbyStatus::*;

const BAD: CrabbyStatus = CRABBY_INVALID_ARGUMENT;

/// A scratch folder of this test binary, removed when dropped.
struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ponko-ffi-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn file(&self, name: &str, bytes: impl AsRef<[u8]>) -> CString {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        CString::new(path.to_str().unwrap()).unwrap()
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A `width` x 1 RGB PNG of mid grey, uncompressed (stored deflate blocks).
fn png(width: u32, height: u32) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc32(&body).to_be_bytes());
    }
    let mut raw = Vec::new();
    for _ in 0..height {
        raw.push(0); // no filter
        raw.extend(std::iter::repeat_n(128u8, width as usize * 3));
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    let mut z = vec![0x78, 0x01, 0x01];
    z.extend((raw.len() as u16).to_le_bytes());
    z.extend((!(raw.len() as u16)).to_le_bytes());
    z.extend(&raw);
    z.extend(((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Runs `body` in a one-frame headless game, hot reload off (so a broken
/// shader file is an error, in debug builds too).
fn with_ctx(body: impl FnOnce(*mut CrabbyCtx)) {
    run_ctx(false, body);
}

fn run_ctx(hot_reload: bool, body: impl FnOnce(*mut CrabbyCtx)) {
    type Body = Box<dyn FnOnce(*mut CrabbyCtx)>;
    struct Slot {
        body: Option<Body>,
        panic: Option<Box<dyn std::any::Any + Send>>,
    }
    unsafe extern "C" fn trampoline(ctx: *mut CrabbyCtx, user: *mut c_void) {
        // SAFETY: `user` is the `Slot` below, alive for the run.
        let slot = unsafe { &mut *user.cast::<Slot>() };
        let body = slot.body.take().expect("setup runs once");
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(ctx))) {
            slot.panic = Some(p);
        }
    }
    let body: Box<dyn FnOnce(*mut CrabbyCtx) + '_> = Box::new(body);
    // SAFETY: the box is consumed inside the headless run, before this returns.
    let body: Body = unsafe { std::mem::transmute(body) };
    let mut slot = Slot {
        body: Some(body),
        panic: None,
    };
    unsafe {
        let game = crabby_game_new(c"files".as_ptr(), 1, 1);
        assert_eq!(crabby_game_hot_reload(game, hot_reload), CRABBY_OK);
        let mut world = null_mut();
        let status = crabby_game_run_headless(game, 1, Some(trampoline), (&raw mut slot).cast(), &mut world);
        if let Some(p) = slot.panic.take() {
            std::panic::resume_unwind(p);
        }
        assert_eq!(status, CRABBY_OK);
        crabby_world_free(world);
    }
}

#[test]
fn shader_effect_and_compute_files_load_once_and_broken_ones_fail() {
    let dir = Scratch::new("shaders");
    let surface = dir.file(
        "wind.wgsl",
        "fn vertex(v: Vertex) -> Vertex { var o = v; o.position.x += material.custom[0].x; return o; }",
    );
    let effect = dir.file(
        "grey.wgsl",
        "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(dot(p.color, vec3<f32>(0.3, 0.6, 0.1))); }",
    );
    let compute = dir.file(
        "fade.wgsl",
        "fn compute(c: Cell) -> vec4<f32> { return c.value * 0.9; }",
    );
    let broken = dir.file("broken.wgsl", "fn surface(s: Surface) -> Surface { return 1; }");
    with_ctx(|ctx| unsafe {
        let (mut a, mut b) = (CrabbyShader::default(), CrabbyShader::default());
        assert_eq!(crabby_ctx_load_shader(ctx, surface.as_ptr(), &mut a), CRABBY_OK);
        assert_eq!(crabby_ctx_load_shader(ctx, surface.as_ptr(), &mut b), CRABBY_OK);
        assert_eq!(a, b, "loading the same file again returns the same shader");
        // The loaded shader works in a material.
        let mut desc = crabby_material_desc(CrabbyColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        desc.has_shader = true;
        desc.shader = a;
        let mut material = CrabbyMaterial::default();
        assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut material), CRABBY_OK);

        let (mut e1, mut e2) = (CrabbyEffect::default(), CrabbyEffect::default());
        assert_eq!(crabby_ctx_load_effect(ctx, effect.as_ptr(), &mut e1), CRABBY_OK);
        assert_eq!(crabby_ctx_load_effect(ctx, effect.as_ptr(), &mut e2), CRABBY_OK);
        assert_eq!(e1, e2);
        let list = [CrabbyScreenEffect {
            effect: e1,
            has_material: false,
            material: CrabbyMaterial::default(),
        }];
        assert_eq!(crabby_ctx_set_effects(ctx, list.as_ptr(), 1), CRABBY_OK);

        let (mut c1, mut c2) = (CrabbyCompute::default(), CrabbyCompute::default());
        assert_eq!(crabby_ctx_load_compute(ctx, compute.as_ptr(), &mut c1), CRABBY_OK);
        assert_eq!(crabby_ctx_load_compute(ctx, compute.as_ptr(), &mut c2), CRABBY_OK);
        assert_eq!(c1, c2);

        // A shader that doesn't compile, a missing file, a null path or output.
        let mut other = CrabbyShader { id: 999 };
        assert_eq!(crabby_ctx_load_shader(ctx, broken.as_ptr(), &mut other), BAD);
        assert_eq!(other.id, 999, "a failed load leaves the output alone");
        assert_eq!(
            crabby_ctx_load_shader(ctx, c"no-such-shader.wgsl".as_ptr(), &mut other),
            BAD
        );
        assert_eq!(crabby_ctx_load_shader(ctx, std::ptr::null(), &mut other), BAD);
        assert_eq!(crabby_ctx_load_shader(ctx, surface.as_ptr(), null_mut()), BAD);
        assert_eq!(crabby_ctx_load_shader(null_mut(), surface.as_ptr(), &mut other), BAD);
        let mut other_effect = CrabbyEffect::default();
        assert_eq!(
            crabby_ctx_load_effect(ctx, broken.as_ptr(), &mut other_effect),
            BAD,
            "a surface isn't an effect"
        );
        assert_eq!(
            crabby_ctx_load_effect(ctx, c"missing.wgsl".as_ptr(), &mut other_effect),
            BAD
        );
        assert_eq!(crabby_ctx_load_effect(ctx, effect.as_ptr(), null_mut()), BAD);
        let mut other_compute = CrabbyCompute::default();
        assert_eq!(crabby_ctx_load_compute(ctx, broken.as_ptr(), &mut other_compute), BAD);
        assert_eq!(
            crabby_ctx_load_compute(ctx, c"missing.wgsl".as_ptr(), &mut other_compute),
            BAD
        );
        assert_eq!(crabby_ctx_load_compute(ctx, compute.as_ptr(), null_mut()), BAD);
    });
}

/// With hot reload on (the default in debug builds), a shader file that
/// doesn't compile still loads: a stand-in, reported as failed, watched
/// until a fix is saved.
#[test]
fn with_hot_reload_on_broken_shader_files_load_as_failed_stand_ins() {
    let dir = Scratch::new("stand-ins");
    let broken = dir.file("broken.wgsl", "fn surface(s: Surface) -> Surface { return 1; }");
    run_ctx(true, |ctx| unsafe {
        let mut shader = CrabbyShader { id: 999 };
        assert_eq!(crabby_ctx_load_shader(ctx, broken.as_ptr(), &mut shader), CRABBY_OK);
        assert_ne!(shader.id, 999);
        let mut status = CrabbyShaderStatus::default();
        assert_eq!(crabby_ctx_shader_status(ctx, 0, &mut status), CRABBY_OK);
        assert_eq!(
            (status.outcome, status.ok, status.line),
            (CRABBY_SHADER_FAILED, false, 1)
        );
        let mut missing = CrabbyShader::default();
        assert_ne!(
            crabby_ctx_load_shader(ctx, c"no-such-shader.wgsl".as_ptr(), &mut missing),
            CRABBY_OK,
            "a missing file is still an error"
        );
    });
}

struct Probe {
    transform: u32,
    probe: u32,
}

unsafe fn spawn_probe(ctx: *mut CrabbyCtx, ids: &Probe, at: f32) -> u64 {
    unsafe {
        let world = crabby_ctx_world(ctx);
        let t = crabby_transform3d_at(CrabbyVec3 { x: at, y: 0.0, z: 0.0 });
        let p = crabby_reflection_probe3d_default();
        let parts = [
            CrabbyValue {
                component: ids.transform,
                data: (&t as *const CrabbyTransform3D).cast(),
                size: size_of::<CrabbyTransform3D>(),
            },
            CrabbyValue {
                component: ids.probe,
                data: (&p as *const CrabbyReflectionProbe3D).cast(),
                size: size_of::<CrabbyReflectionProbe3D>(),
            },
        ];
        let mut entity = 0;
        assert_eq!(crabby_spawn(world, parts.as_ptr(), 2, &mut entity), CRABBY_OK);
        entity
    }
}

unsafe fn content(ctx: *mut CrabbyCtx, ids: &Probe, entity: u64) -> u32 {
    unsafe {
        let mut probe = crabby_reflection_probe3d_default();
        assert_eq!(
            crabby_get(
                crabby_ctx_world(ctx),
                entity,
                ids.probe,
                (&raw mut probe).cast(),
                size_of::<CrabbyReflectionProbe3D>()
            ),
            CRABBY_OK
        );
        probe.content
    }
}

#[test]
fn baking_a_probe_allocates_one_environment_and_only_four_exist() {
    with_ctx(|ctx| unsafe {
        let world = crabby_ctx_world(ctx);
        let mut ids = Probe { transform: 0, probe: 0 };
        assert_eq!(crabby_transform3d_component(world, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_reflection_probe3d_component(world, &mut ids.probe), CRABBY_OK);
        let first = spawn_probe(ctx, &ids, 0.0);
        assert_eq!(content(ctx, &ids, first), 0, "unbaked probes fall back to the sky");
        assert_eq!(crabby_ctx_bake_reflection_probe(ctx, first), CRABBY_OK);
        let slot = content(ctx, &ids, first);
        assert_ne!(slot, 0);
        assert_eq!(crabby_ctx_bake_reflection_probe(ctx, first), CRABBY_OK);
        assert_eq!(content(ctx, &ids, first), slot, "baking again reuses the environment");

        // Four environments in all: the fifth probe cannot be baked.
        for i in 1..4 {
            let probe = spawn_probe(ctx, &ids, i as f32 * 20.0);
            assert_eq!(crabby_ctx_bake_reflection_probe(ctx, probe), CRABBY_OK);
            assert_eq!(content(ctx, &ids, probe), slot + i);
        }
        let fifth = spawn_probe(ctx, &ids, 100.0);
        assert_eq!(crabby_ctx_bake_reflection_probe(ctx, fifth), BAD);
        assert_eq!(content(ctx, &ids, fifth), 0);

        // An entity without a probe, and a bad context.
        let mut plain = 0;
        assert_eq!(crabby_spawn(world, std::ptr::null(), 0, &mut plain), CRABBY_OK);
        assert_eq!(crabby_ctx_bake_reflection_probe(ctx, plain), BAD);
        assert_eq!(crabby_ctx_bake_reflection_probe(null_mut(), first), BAD);
    });
}

#[test]
fn a_probe_loads_a_two_to_one_environment_image_and_refuses_other_shapes() {
    let dir = Scratch::new("probe");
    let good = dir.file("sky.png", png(8, 4));
    let square = dir.file("square.png", png(4, 4));
    let garbage = dir.file("garbage.png", b"not an image");
    with_ctx(|ctx| unsafe {
        let world = crabby_ctx_world(ctx);
        let mut ids = Probe { transform: 0, probe: 0 };
        assert_eq!(crabby_transform3d_component(world, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_reflection_probe3d_component(world, &mut ids.probe), CRABBY_OK);
        let probe = spawn_probe(ctx, &ids, 0.0);
        for refused in [&square, &garbage] {
            assert_eq!(crabby_ctx_load_reflection_probe(ctx, probe, refused.as_ptr()), BAD);
        }
        assert_eq!(
            crabby_ctx_load_reflection_probe(ctx, probe, c"missing-sky.png".as_ptr()),
            BAD
        );
        assert_eq!(crabby_ctx_load_reflection_probe(ctx, probe, std::ptr::null()), BAD);
        assert_eq!(content(ctx, &ids, probe), 0, "refused loads bake nothing");

        assert_eq!(crabby_ctx_load_reflection_probe(ctx, probe, good.as_ptr()), CRABBY_OK);
        assert_ne!(content(ctx, &ids, probe), 0, "the image baked into an environment");
        assert_eq!(crabby_ctx_load_reflection_probe(null_mut(), probe, good.as_ptr()), BAD);
        let mut plain = 0;
        assert_eq!(crabby_spawn(world, std::ptr::null(), 0, &mut plain), CRABBY_OK);
        assert_eq!(
            crabby_ctx_load_reflection_probe(ctx, plain, good.as_ptr()),
            BAD,
            "no probe on it"
        );
    });
}
