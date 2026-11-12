//! Asset sources from C: callback sources, memory sources and
//! `ponko_ctx_load_asset_with`.
mod common;
use common::*;
use ponko_ffi::*;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering};

use PonkoErrorKind::*;
use PonkoStatus::*;

/// What a callback source serves, and how often it was asked and freed.
struct Pak {
    files: HashMap<String, Vec<u8>>,
    reads: usize,
    /// Every name read, in order.
    read_names: Vec<String>,
    /// Every name asked through the `contains` callback, in order.
    contains_names: Vec<String>,
    frees: &'static AtomicUsize,
}
unsafe extern "C" fn pak_read(user: *mut c_void, name: *const c_char, data: *mut *const u8, len: *mut usize) -> u32 {
    let pak = unsafe { &mut *user.cast::<Pak>() };
    pak.reads += 1;
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    pak.read_names.push(name.to_owned());
    if name == "broken.bin" {
        return PONKO_PANIC as u32; // any other status: a read error
    }
    match pak.files.get(name) {
        Some(bytes) => unsafe {
            data.write(bytes.as_ptr());
            len.write(bytes.len());
            PONKO_OK as u32
        },
        None => PONKO_NOT_FOUND as u32,
    }
}
unsafe extern "C" fn pak_contains(user: *mut c_void, name: *const c_char) -> bool {
    let pak = unsafe { &mut *user.cast::<Pak>() };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    pak.contains_names.push(name.to_owned());
    pak.files.contains_key(name)
}
unsafe extern "C" fn pak_free(user: *mut c_void) {
    let pak = unsafe { Box::from_raw(user.cast::<Pak>()) };
    pak.frees.fetch_add(1, Ordering::SeqCst);
}
fn pak(frees: &'static AtomicUsize, files: &[(&str, &[u8])]) -> *mut c_void {
    let files = files.iter().map(|(n, b)| (n.to_string(), b.to_vec())).collect();
    Box::into_raw(Box::new(Pak {
        files,
        reads: 0,
        read_names: Vec::new(),
        contains_names: Vec::new(),
        frees,
    }))
    .cast()
}

/// Loads `name` through `ponko_ctx_load_asset_with` into a byte vector.
unsafe fn read(ctx: *mut PonkoCtx, name: &CStr) -> Result<Vec<u8>, PonkoStatus> {
    unsafe extern "C" fn keep(user: *mut c_void, data: *const u8, len: usize) -> u32 {
        let out = unsafe { &mut *user.cast::<Vec<u8>>() };
        if len > 0 {
            out.extend_from_slice(unsafe { std::slice::from_raw_parts(data, len) });
        }
        PONKO_OK as u32
    }
    let mut out = Vec::new();
    match unsafe { ponko_ctx_load_asset_with(ctx, name.as_ptr(), Some(keep), (&raw mut out).cast()) } {
        PONKO_OK => Ok(out),
        s => Err(s),
    }
}

#[test]
fn a_callback_source_is_asked_first_and_freed_once() {
    static FREES: AtomicUsize = AtomicUsize::new(0);
    let user = pak(&FREES, &[("levels/one.txt", b"##.\n"), ("red.png", &PIXEL_PNG)]);
    run_ctx(|ctx| unsafe {
        let status = ponko_ctx_add_asset_source(
            ctx,
            c"pak".as_ptr(),
            Some(pak_read),
            Some(pak_contains),
            Some(pak_free),
            user,
        );
        assert_eq!(status, PONKO_OK);
        assert_eq!(read(ctx, c"levels/one.txt"), Ok(b"##.\n".to_vec()));
        let mut red = PonkoTexture {
            id: 0,
            width: 0,
            height: 0,
        };
        assert_eq!(
            ponko_ctx_try_load_texture(ctx, c"red.png".as_ptr(), &mut red),
            PONKO_OK,
            "{}",
            last_error()
        );
        assert_eq!((red.width, red.height), (1, 1), "a texture from the source");
        let reads = (*user.cast::<Pak>()).reads;
        let mut again = PonkoTexture {
            id: 0,
            width: 0,
            height: 0,
        };
        assert_eq!(
            ponko_ctx_try_load_texture(ctx, c"red.png".as_ptr(), &mut again),
            PONKO_OK
        );
        assert_eq!(again.id, red.id, "cached by source and name");
        assert_eq!((*user.cast::<Pak>()).reads, reads, "not read again");

        assert_eq!(read(ctx, c"missing.txt"), Err(PONKO_NOT_FOUND));
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_NOT_FOUND);
        assert_eq!(last_path(), "missing.txt");
        assert_eq!(read(ctx, c"broken.bin"), Err(PONKO_INVALID_ARGUMENT));
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_IO);
        assert!(last_error().contains("pak"), "{}", last_error());
        assert_eq!(FREES.load(Ordering::SeqCst), 0, "alive while the game runs");
    });
    assert_eq!(FREES.load(Ordering::SeqCst), 1, "freed once when the game ends");
}

#[test]
fn a_memory_source_holds_copies() {
    run_ctx(|ctx| unsafe {
        let source = ponko_memory_source_new(c"embedded".as_ptr());
        let rules = b"{\"speed\": 30}";
        assert_eq!(
            ponko_memory_source_insert(source, c"rules.json".as_ptr(), rules.as_ptr(), rules.len()),
            PONKO_OK
        );
        assert_eq!(
            ponko_memory_source_insert(source, c"empty".as_ptr(), std::ptr::null(), 0),
            PONKO_OK
        );
        assert_eq!(
            ponko_memory_source_insert(source, c"x".as_ptr(), std::ptr::null(), 3),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(ponko_ctx_add_memory_source(ctx, source), PONKO_OK);
        // Changing the handle afterwards doesn't change what the game has.
        assert_eq!(
            ponko_memory_source_insert(source, c"rules.json".as_ptr(), b"{}".as_ptr(), 2),
            PONKO_OK
        );
        ponko_memory_source_free(source);
        assert_eq!(read(ctx, c"rules.json"), Ok(rules.to_vec()));
        assert_eq!(read(ctx, c"empty"), Ok(Vec::new()));
    });
}

#[test]
fn rejected_sources_leave_their_user_to_the_caller() {
    static FREES: AtomicUsize = AtomicUsize::new(0);
    let first = pak(&FREES, &[]);
    let second = pak(&FREES, &[]);
    run_ctx(|ctx| unsafe {
        let add =
            |id: &CStr, user| ponko_ctx_add_asset_source(ctx, id.as_ptr(), Some(pak_read), None, Some(pak_free), user);
        assert_eq!(add(c"", second), PONKO_INVALID_ARGUMENT, "empty id");
        assert_eq!(
            ponko_ctx_add_asset_source(ctx, c"pak".as_ptr(), None, None, Some(pak_free), second),
            PONKO_INVALID_ARGUMENT,
            "no read callback"
        );
        assert_eq!(add(c"pak", first), PONKO_OK);
        assert_eq!(add(c"pak", second), PONKO_INVALID_ARGUMENT, "a duplicate id");
        assert!(last_error().contains("already added"), "{}", last_error());
        let memory = ponko_memory_source_new(c"pak".as_ptr());
        assert_eq!(
            ponko_ctx_add_memory_source(ctx, memory),
            PONKO_INVALID_ARGUMENT,
            "the same id from memory"
        );
        ponko_memory_source_free(memory);
        assert!(ponko_memory_source_new(c"".as_ptr()).is_null());
        assert_eq!(
            ponko_ctx_add_memory_source(ctx, std::ptr::null()),
            PONKO_INVALID_ARGUMENT
        );
    });
    assert_eq!(FREES.load(Ordering::SeqCst), 1, "only the source the game took");
    unsafe { pak_free(second) };
}

#[test]
fn decoders_report_failures_and_may_use_the_context() {
    run_ctx(|ctx| unsafe {
        let source = ponko_memory_source_new(c"m".as_ptr());
        let pixels = [255u8, 0, 0, 255];
        assert_eq!(
            ponko_memory_source_insert(source, c"pixel.raw".as_ptr(), pixels.as_ptr(), 4),
            PONKO_OK
        );
        assert_eq!(ponko_ctx_add_memory_source(ctx, source), PONKO_OK);
        ponko_memory_source_free(source);

        unsafe extern "C" fn refuse(_: *mut c_void, _: *const u8, _: usize) -> u32 {
            PONKO_INVALID_ARGUMENT as u32
        }
        let status = ponko_ctx_load_asset_with(ctx, c"pixel.raw".as_ptr(), Some(refuse), null_mut());
        assert_eq!(status, PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_last_error_kind(), PONKO_ERROR_DECODE);
        assert_eq!(last_path(), "pixel.raw");

        // A decoder that makes a texture from the bytes: the context is free.
        struct Make {
            ctx: *mut PonkoCtx,
            texture: PonkoTexture,
        }
        unsafe extern "C" fn make(user: *mut c_void, data: *const u8, len: usize) -> u32 {
            let m = unsafe { &mut *user.cast::<Make>() };
            unsafe { ponko_ctx_create_texture(m.ctx, 1, 1, data, len, &mut m.texture) as u32 }
        }
        let mut m = Make {
            ctx,
            texture: PonkoTexture {
                id: 0,
                width: 0,
                height: 0,
            },
        };
        let status = ponko_ctx_load_asset_with(ctx, c"pixel.raw".as_ptr(), Some(make), (&raw mut m).cast());
        assert_eq!(status, PONKO_OK, "{}", last_error());
        assert_eq!(m.texture.width, 1);
        assert_eq!(
            ponko_ctx_load_asset_with(ctx, c"pixel.raw".as_ptr(), None, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
    });
}

/// A one-triangle `.gltf` with its geometry in `tri.bin` and a material
/// showing image file `tex.png`, both next to it.
const TRIANGLE_GLTF: &str = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
    "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"indices":2,"material":0}]}],
    "materials":[{"name":"skin","pbrMetallicRoughness":{"baseColorTexture":{"index":0}}}],
    "textures":[{"source":0}],"images":[{"uri":"tex.png"}],
    "buffers":[{"uri":"tri.bin","byteLength":66}],
    "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24},
        {"buffer":0,"byteOffset":60,"byteLength":6}],
    "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},
        {"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"},
        {"bufferView":2,"componentType":5123,"count":3,"type":"SCALAR"}]}"#;

/// `tri.bin` for [`TRIANGLE_GLTF`]: three corners, their UVs, three indices.
fn triangle_bin() -> Vec<u8> {
    let mut bin = Vec::new();
    for v in [
        0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
    ] {
        bin.extend_from_slice(&v.to_le_bytes());
    }
    for i in [0u16, 1, 2] {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    bin
}

/// Loads [`TRIANGLE_GLTF`] from a callback source, with or without a
/// `contains` callback, and returns what the source was asked.
fn load_gltf_through_callbacks(contains: PonkoAssetContainsFn) -> (Vec<String>, Vec<String>) {
    static FREES: AtomicUsize = AtomicUsize::new(0);
    let bin = triangle_bin();
    let user = pak(
        &FREES,
        &[
            ("tri.gltf", TRIANGLE_GLTF.as_bytes()),
            ("tri.bin", &bin),
            ("tex.png", &PIXEL_PNG),
        ],
    );
    let mut asked = (Vec::new(), Vec::new());
    run_ctx(|ctx| unsafe {
        assert_eq!(
            ponko_ctx_add_asset_source(ctx, c"pak".as_ptr(), Some(pak_read), contains, None, user),
            PONKO_OK
        );
        let mut mesh = std::mem::zeroed::<PonkoMesh>();
        assert_eq!(
            ponko_ctx_try_load_mesh(ctx, c"tri.gltf".as_ptr(), &mut mesh),
            PONKO_OK,
            "{}",
            last_error()
        );
        let pak = &*user.cast::<Pak>();
        asked = (pak.read_names.clone(), pak.contains_names.clone());
    });
    drop(unsafe { Box::from_raw(user.cast::<Pak>()) });
    asked
}

#[test]
fn a_contains_callback_locates_gltf_images_without_reading_them() {
    let (reads, contains) = load_gltf_through_callbacks(Some(pak_contains));
    assert!(contains.iter().any(|n| n == "tex.png"), "contains asked: {contains:?}");
    let image_reads = reads.iter().filter(|n| *n == "tex.png").count();
    assert_eq!(image_reads, 1, "only the image load reads it: {reads:?}");
    assert!(reads.iter().any(|n| n == "tri.bin"), "{reads:?}");
}

#[test]
fn without_a_contains_callback_a_read_locates_gltf_images() {
    let (reads, contains) = load_gltf_through_callbacks(None);
    assert!(contains.is_empty());
    let image_reads = reads.iter().filter(|n| *n == "tex.png").count();
    assert!(image_reads >= 2, "located by reading, then loaded: {reads:?}");
}
