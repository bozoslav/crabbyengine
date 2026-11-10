//! Shared helpers and conventions for the offscreen GPU suites
//! (`src/render/*_render_test.rs`).
//!
//! # Conventions
//!
//! - **Adapters.** Get the device with [`gpu`] or [`gpu_with`], never by
//!   calling `request_adapter` directly. When no adapter or device exists the
//!   test returns early and logs `render test SKIPPED [<test>]: <reason>`;
//!   when it runs it logs `render test ran [<test>] on <adapter> (<backend>,
//!   ...)`. A test that needs a capability the adapter lacks calls [`skip`]
//!   with the reason. A skip is not a pass: it proves nothing about the
//!   picture. To see what ran, use `--nocapture` and look for those lines:
//!
//!   ```sh
//!   cargo test -p ponko --lib --features 3d render_test -- --nocapture 2>&1 | grep "render test"
//!   ```
//!
//!   Set `PONKO_REQUIRE_GPU=1` on a machine (or CI runner) that has a GPU to
//!   turn a missing adapter or device into a failure instead of a skip.
//!   Capability skips stay skips: some adapters really lack a feature.
//! - **GPU tiers.** Set `PONKO_GPU_TIER` to `webgl2` or `baseline` to run
//!   every suite on a device opened with that tier's limits and without
//!   timestamp queries ([`forced`]; `desktop` runs as usual), as CI does
//!   one tier at a time:
//!
//!   ```sh
//!   PONKO_GPU_TIER=webgl2 cargo test -p ponko --lib --features 3d render_test
//!   ```
//!
//!   A test that needs a feature the forced tier lacks calls [`skip`] with
//!   the reason ([`needs`] does both); every other test must pass as on the
//!   real device. Only the capability suite opens its own forced devices.
//! - **Assertions.** Prefer semantic checks that say what must hold ("the box
//!   is lit", "the background behind it stays black", "this mode costs more
//!   than that one") with a negative control showing the check can fail.
//!   Compare images with [`diff`] and a [`Tolerance`] (per-channel slack plus
//!   a fraction of pixels allowed beyond it), not byte equality, unless the
//!   same device must produce identical output (a fallback drawing exactly
//!   like the mode it falls back to). Print the measured numbers with
//!   `eprintln!` so a failure on other hardware can be read.
//! - **Reference images.** A whole-image baseline goes through
//!   [`check_reference`]: a small PNG in `tests/fixtures/render/<name>.png`
//!   and its metadata in `<name>.json` (size, format, tolerance, the scene and
//!   the adapter it was made on). A missing or mismatching reference fails,
//!   and the actual and difference images are written to
//!   `target/render-test-output/` for inspection. Nothing is ever written to
//!   the fixtures unless asked: `PONKO_UPDATE_REFERENCES=1` regenerates them
//!   locally, and is refused when `CI` is set. Changing a tolerance in code
//!   without regenerating fails too, so a looser baseline always shows up in
//!   review. Commit regenerated images only with the before/after images in
//!   the pull request, and keep them small (64x64 is plenty).
//! - **Readback.** [`read_texture`] copies any texture back with row padding
//!   handled; [`target`] makes an offscreen colour target to draw into, and
//!   [`white_textures`] the one-white-texture set most rigs draw with.
//!
//! The TAA, light shaft and decal suites use these helpers; the older suites
//! still carry their own copies and move over as they are touched.
use super::gpu::GpuTexture;
use std::path::PathBuf;

/// A device for one test, and what it runs on.
pub(crate) struct TestGpu {
    pub info: wgpu::AdapterInfo,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// The running test's name (libtest names each test's thread after it).
fn test_name() -> String {
    std::thread::current().name().unwrap_or("unnamed test").to_owned()
}

fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty() && v != "0")
}

/// Logs that the running test skips because the adapter can't do something
/// it needs. The caller returns early.
pub(super) fn skip(reason: impl std::fmt::Display) {
    eprintln!("render test SKIPPED [{}]: {reason}", test_name());
}

/// A missing adapter or device: a skip, or a failure under `PONKO_REQUIRE_GPU`.
/// Every suite reports a failed adapter or device request through this.
pub(super) fn no_gpu(reason: impl std::fmt::Display) {
    if env_set("PONKO_REQUIRE_GPU") {
        panic!("{reason}, and PONKO_REQUIRE_GPU is set");
    }
    skip(reason);
}

/// The tier `PONKO_GPU_TIER` forces on the suites, if any.
pub(super) fn forced_tier() -> Option<crate::GpuTier> {
    crate::GpuTier::forced()
}

/// `descriptor` as the forced tier opens it (unchanged without one): that
/// tier's limits, and no timestamp queries below `desktop`. Every suite
/// opens its device through this.
pub(super) fn forced<'a>(descriptor: &wgpu::DeviceDescriptor<'a>) -> wgpu::DeviceDescriptor<'a> {
    let mut d = descriptor.clone();
    if let Some(tier) = forced_tier() {
        // CI checks for these lines: a run where nothing opened a device on
        // the tier must not pass.
        eprintln!("render test opens a device [{}] as the {tier} tier", test_name());
        d.required_limits = tier.device_limits(d.required_limits);
        d.required_features = tier.device_features(d.required_features);
    }
    d
}

/// Whether `device` has `feature` (as the renderer decides it); if not,
/// logs a skip naming the forced tier (the caller returns early).
pub(super) fn needs(device: &wgpu::Device, feature: crate::GpuFeature) -> bool {
    let caps = super::capabilities::report(
        Default::default(),
        super::capabilities::GpuLimits::of(device),
        device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        (4, 4),
        true,
    );
    if caps.available(feature) {
        return true;
    }
    let tier = forced_tier().map_or_else(|| "this device".to_owned(), |t| format!("the forced {t} tier"));
    skip(format_args!("needs {feature:?}, which {tier} lacks"));
    false
}

/// A device with default limits and no optional features, or `None` (the
/// test returns early; see the module docs).
pub(crate) fn gpu() -> Option<TestGpu> {
    gpu_with(|_| Ok(wgpu::DeviceDescriptor::default()))
}

/// A device made from the descriptor `describe` picks for the adapter.
/// `describe` returns `Err(reason)` when the adapter can't run the test (a
/// capability skip, logged with that reason).
pub(super) fn gpu_with(
    describe: impl FnOnce(&wgpu::Adapter) -> Result<wgpu::DeviceDescriptor<'static>, String>,
) -> Option<TestGpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
        Ok(adapter) => adapter,
        Err(e) => {
            no_gpu(format_args!("no GPU adapter ({e})"));
            return None;
        }
    };
    let info = adapter.get_info();
    let descriptor = match describe(&adapter) {
        Ok(descriptor) => descriptor,
        Err(reason) => {
            skip(format_args!("{reason} (on {})", info.name));
            return None;
        }
    };
    let (device, queue) = match pollster::block_on(adapter.request_device(&forced(&descriptor))) {
        Ok(pair) => pair,
        Err(e) => {
            no_gpu(format_args!("no GPU device on {} ({e})", info.name));
            return None;
        }
    };
    match forced_tier() {
        Some(tier) => eprintln!(
            "render test ran [{}] on {} as the {tier} tier",
            test_name(),
            describe_adapter(&info)
        ),
        None => eprintln!("render test ran [{}] on {}", test_name(), describe_adapter(&info)),
    }
    Some(TestGpu { info, device, queue })
}

/// An adapter's name, backend, type and driver, for logs and metadata.
pub(super) fn describe_adapter(info: &wgpu::AdapterInfo) -> String {
    format!(
        "{} ({:?}, {:?}, driver {})",
        info.name,
        info.backend,
        info.device_type,
        driver(info)
    )
}

/// The adapter's driver and its version, or "unknown" (Metal reports none).
fn driver(info: &wgpu::AdapterInfo) -> String {
    let driver = format!("{} {}", info.driver, info.driver_info);
    match driver.trim() {
        "" => "unknown".to_owned(),
        d => d.to_owned(),
    }
}

/// The texture set most suites draw with: a bind group layout for one
/// filtered texture, a 1x1 white sRGB texture (also the renderer's default),
/// and that texture as texture 0.
pub(super) fn white_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (wgpu::BindGroupLayout, wgpu::Texture, Vec<GpuTexture>) {
    let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty,
        count: None,
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            entry(
                0,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            ),
            entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
        ],
    });
    let white = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });
    queue.write_texture(
        white.as_image_copy(),
        &[255; 4],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: None,
        },
        white.size(),
    );
    let sampler = device.create_sampler(&Default::default());
    let view = white.create_view(&Default::default());
    let textures = vec![GpuTexture {
        texture: white.clone(),
        nearest: false,
        mips: None,
        bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        }),
    }];
    (layout, white, textures)
}

/// An offscreen colour target of `size` that can be drawn into and read back.
pub(super) fn target(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// The first mip and layer of `texture` (made with `COPY_SRC`), row by row
/// from the top with no padding. Waits for the GPU.
pub(super) fn read_texture(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let (width, height) = (texture.width(), texture.height());
    let texel = texture
        .format()
        .block_copy_size(None)
        .expect("a format with one copyable aspect");
    let row = width * texel;
    let padded = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(padded * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    readback.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let mapped = readback.get_mapped_range(..).unwrap();
    if padded == row {
        return mapped.to_vec();
    }
    mapped
        .chunks_exact(padded as usize)
        .flat_map(|r| &r[..row as usize])
        .copied()
        .collect()
}

/// An RGBA8 image read back from the GPU, row by row from the top.
#[derive(Clone, PartialEq)]
pub(super) struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image({}x{})", self.width, self.height)
    }
}

impl Image {
    /// Reads back an RGBA8 (or BGRA8, swizzled to RGBA) texture.
    pub fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Self {
        let mut rgba = read_texture(device, queue, texture);
        match texture.format() {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {}
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                rgba.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p.swap(0, 2));
            }
            other => panic!("not an 8-bit RGBA format: {other:?}"),
        }
        Self {
            width: texture.width(),
            height: texture.height(),
            rgba,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.width + x) * 4) as usize;
        self.rgba[at..at + 4].try_into().unwrap()
    }

    /// The fraction of the pixels in the rectangle `x` by `y` for which
    /// `test` holds.
    pub fn fraction(&self, x: std::ops::Range<u32>, y: std::ops::Range<u32>, test: impl Fn([u8; 4]) -> bool) -> f32 {
        let (mut hits, mut total) = (0usize, 0usize);
        for py in y {
            for px in x.clone() {
                total += 1;
                hits += usize::from(test(self.pixel(px, py)));
            }
        }
        assert!(total > 0, "an empty region");
        hits as f32 / total as f32
    }
}

/// Asserts that `test` holds for at least `at_least` (0..=1) of the pixels
/// in the rectangle `x` by `y`: a semantic check ("the box is lit").
pub(super) fn assert_region(
    image: &Image,
    x: std::ops::Range<u32>,
    y: std::ops::Range<u32>,
    at_least: f32,
    what: &str,
    test: impl Fn([u8; 4]) -> bool,
) {
    let (xs, ys) = (x.clone(), y.clone());
    let got = image.fraction(x, y, test);
    assert!(
        got >= at_least,
        "{what}: holds for {:.1}% of pixels {xs:?} x {ys:?}, wanted at least {:.1}%",
        got * 100.0,
        at_least * 100.0
    );
}

/// How far an image may stray from what it's compared with: each channel by
/// up to `channel`, and beyond that on at most `outliers` (0..=1) of the
/// pixels (edges rasterise slightly differently across GPUs).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Tolerance {
    pub channel: u8,
    pub outliers: f32,
}

/// How two images of the same size differ.
#[derive(Clone, Copy, Debug)]
pub(super) struct Diff {
    /// The largest difference of any channel.
    pub max_channel: u8,
    /// The mean absolute difference over every channel, 0..=255.
    pub mean: f32,
    /// Pixels with a channel differing by more than the tolerance's.
    pub outliers: usize,
    pub pixels: usize,
}

impl Diff {
    pub fn within(&self, tolerance: Tolerance) -> bool {
        self.outliers as f32 <= tolerance.outliers * self.pixels as f32
    }
}

impl std::fmt::Display for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "max channel difference {}, mean {:.3}, {} of {} pixels ({:.2}%) beyond the tolerance",
            self.max_channel,
            self.mean,
            self.outliers,
            self.pixels,
            self.outliers as f32 * 100.0 / self.pixels.max(1) as f32
        )
    }
}

/// How `a` differs from `b`, counting pixels with a channel off by more than
/// `channel`.
pub(super) fn diff(a: &Image, b: &Image, channel: u8) -> Diff {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "comparing images of different sizes"
    );
    let (mut max_channel, mut sum, mut outliers) = (0u8, 0u64, 0usize);
    for (p, q) in a.rgba.as_chunks::<4>().0.iter().zip(b.rgba.as_chunks::<4>().0) {
        let worst = p.iter().zip(q).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        sum += p.iter().zip(q).map(|(x, y)| u64::from(x.abs_diff(*y))).sum::<u64>();
        max_channel = max_channel.max(worst);
        outliers += usize::from(worst > channel);
    }
    let pixels = a.rgba.len() / 4;
    Diff {
        max_channel,
        mean: sum as f32 / (pixels * 4).max(1) as f32,
        outliers,
        pixels,
    }
}

/// A reference image's metadata, stored next to it as `<name>.json`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct ReferenceMeta {
    pub name: String,
    /// What the picture shows and how it's set up, for reviewers.
    pub scene: String,
    pub width: u32,
    pub height: u32,
    /// Always 8-bit RGBA, sRGB-encoded.
    pub format: String,
    pub tolerance: Tolerance,
    /// The adapter the reference was made on.
    pub adapter: String,
    pub backend: String,
    pub driver: String,
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/render")
}

/// Where failing comparisons leave their images.
pub(super) fn output_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"))
        .join("render-test-output")
}

fn save_png(path: &std::path::Path, image: &Image) {
    image::save_buffer(path, &image.rgba, image.width, image.height, image::ColorType::Rgba8)
        .unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
}

/// Whether reference images may be rewritten in this run (see the module docs).
fn updating_references() -> bool {
    if !env_set("PONKO_UPDATE_REFERENCES") {
        return false;
    }
    assert!(
        !env_set("CI"),
        "PONKO_UPDATE_REFERENCES is refused in CI: regenerate references locally and commit them for review"
    );
    true
}

/// Compares `image` with the reference `tests/fixtures/render/<name>.png`
/// within `tolerance` (see the module docs). `scene` describes the setup for
/// the metadata; `gpu` is the adapter that drew `image`.
pub(super) fn check_reference(name: &str, image: &Image, tolerance: Tolerance, scene: &str, gpu: &wgpu::AdapterInfo) {
    let (png, json) = (
        fixtures().join(format!("{name}.png")),
        fixtures().join(format!("{name}.json")),
    );
    if updating_references() {
        std::fs::create_dir_all(fixtures()).unwrap();
        save_png(&png, image);
        let meta = ReferenceMeta {
            name: name.to_owned(),
            scene: scene.to_owned(),
            width: image.width,
            height: image.height,
            format: "rgba8-srgb".to_owned(),
            tolerance,
            adapter: gpu.name.clone(),
            backend: format!("{:?}", gpu.backend),
            driver: driver(gpu),
        };
        std::fs::write(&json, serde_json::to_string_pretty(&meta).unwrap() + "\n").unwrap();
        eprintln!(
            "updated reference {}: review it and include before/after images in the pull request",
            png.display()
        );
        return;
    }
    let how = "regenerate locally with PONKO_UPDATE_REFERENCES=1 and review the new image";
    let meta: ReferenceMeta = match std::fs::read_to_string(&json) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", json.display())),
        Err(e) => panic!("no reference metadata {} ({e}): {how}", json.display()),
    };
    let reference = image::open(&png)
        .unwrap_or_else(|e| panic!("no reference image {} ({e}): {how}", png.display()))
        .into_rgba8();
    let reference = Image {
        width: reference.width(),
        height: reference.height(),
        rgba: reference.into_raw(),
    };
    assert_eq!(
        (meta.width, meta.height),
        (reference.width, reference.height),
        "{}: the metadata's size doesn't match the image",
        json.display()
    );
    assert_eq!(
        meta.tolerance, tolerance,
        "{name}: the tolerance in code differs from the reference's metadata; {how}"
    );
    assert_eq!(
        (image.width, image.height),
        (reference.width, reference.height),
        "{name}: drawn at a different size than its reference; {how}"
    );
    let d = diff(image, &reference, tolerance.channel);
    eprintln!(
        "reference {name}: {d} (made on {}, {}; drawn on {}, {:?})",
        meta.adapter, meta.backend, gpu.name, gpu.backend
    );
    if d.within(tolerance) {
        return;
    }
    let out = output_dir();
    std::fs::create_dir_all(&out).unwrap();
    let actual = out.join(format!("{name}.actual.png"));
    let difference = out.join(format!("{name}.diff.png"));
    save_png(&actual, image);
    // Pixels beyond the tolerance in red, the rest as dim grey.
    let mut shown = image.clone();
    for (p, q) in shown
        .rgba
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(reference.rgba.as_chunks::<4>().0)
    {
        let worst = p.iter().zip(q).take(3).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        let grey = (p[0] as u16 + p[1] as u16 + p[2] as u16) / 12;
        p.copy_from_slice(&if worst > tolerance.channel {
            [255, 0, 0, 255]
        } else {
            [grey as u8, grey as u8, grey as u8, 255]
        });
    }
    save_png(&difference, &shown);
    panic!(
        "{name} differs from its reference: {d}, allowed {tolerance:?}. Reference made on {} ({}, driver {}), drawn on {}. \
         See {} and {}; if the change is intended, {how}",
        meta.adapter,
        meta.backend,
        meta.driver,
        describe_adapter(gpu),
        actual.display(),
        difference.display()
    );
}

#[test]
fn diff_counts_pixels_beyond_the_channel_tolerance() {
    let a = Image {
        width: 2,
        height: 2,
        rgba: vec![0, 0, 0, 255, 10, 10, 10, 255, 100, 100, 100, 255, 255, 255, 255, 255],
    };
    let mut b = a.clone();
    b.rgba[4] = 13; // within 3
    b.rgba[8] = 120; // beyond
    let d = diff(&a, &b, 3);
    assert_eq!((d.max_channel, d.outliers, d.pixels), (20, 1, 4));
    assert!(d.within(Tolerance {
        channel: 3,
        outliers: 0.25
    }));
    assert!(!d.within(Tolerance {
        channel: 3,
        outliers: 0.2
    }));
    assert!((d.mean - 23.0 / 16.0).abs() < 1e-6);
}

#[test]
fn semantic_regions_report_the_fraction_that_holds() {
    let image = Image {
        width: 2,
        height: 1,
        rgba: vec![255, 0, 0, 255, 0, 0, 0, 255],
    };
    assert_eq!(image.fraction(0..2, 0..1, |p| p[0] > 128), 0.5);
    assert_region(&image, 0..1, 0..1, 1.0, "the left pixel is red", |p| p[0] > 128);
}
