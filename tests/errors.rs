//! `ponko::Error`: every loader's `try_` twin reports what went wrong as an
//! [`ErrorKind`], and the panicking twin panics with the same message.

use ponko::prelude::*;
#[allow(unused_imports)]
use ponko::reflect::ReflectError;
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::render::{Material, MaterialDesc, MeshData};

/// Writes `bytes` to a temporary file and returns its absolute path, which
/// the asset loaders accept as is.
fn temp_file(name: &str, bytes: &[u8]) -> String {
    let path = std::env::temp_dir().join(format!("ponko-errors-{}-{name}", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    path.to_string_lossy().into_owned()
}

fn in_tick(f: impl FnOnce(&mut Ctx) + 'static) {
    Game::new("errors", 1, 1).run_headless(0, f);
}

#[test]
fn missing_assets_are_not_found_for_every_loader() {
    in_tick(|ctx| {
        let missing = "no/such/asset.bin";
        assert_eq!(
            ctx.gfx.try_load_texture(missing).unwrap_err().kind(),
            ErrorKind::NotFound
        );
        assert_eq!(ctx.gfx.try_load_font(missing).unwrap_err().kind(), ErrorKind::NotFound);
        assert_eq!(ctx.audio.try_load(missing).unwrap_err().kind(), ErrorKind::NotFound);
        let err = ctx.gfx.try_load_texture(missing).unwrap_err();
        assert_eq!(err.path(), Some(missing));
    });
}

#[test]
fn files_that_are_not_images_fail_to_decode() {
    let path = temp_file("bad.png", b"not a png");
    in_tick(move |ctx| {
        let err = ctx.gfx.try_load_texture(&path).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Decode);
        assert_eq!(err.path(), Some(path.as_str()));
    });
}

#[test]
fn files_that_are_not_fonts_fail_to_decode() {
    let path = temp_file("bad.ttf", b"not a font");
    in_tick(move |ctx| {
        assert_eq!(ctx.gfx.try_load_font(&path).unwrap_err().kind(), ErrorKind::Decode);
    });
}

#[cfg(feature = "audio")]
#[test]
fn files_that_are_not_sounds_fail_to_decode() {
    let path = temp_file("bad.ogg", b"not a sound");
    in_tick(move |ctx| {
        assert_eq!(ctx.audio.try_load(&path).unwrap_err().kind(), ErrorKind::Decode);
    });
}

#[test]
fn a_failed_load_leaves_later_loads_working() {
    in_tick(|ctx| {
        assert!(ctx.gfx.try_load_texture("nope.png").is_err());
        let a = ctx.gfx.try_load_texture("crab.png").unwrap();
        assert_eq!(ctx.gfx.try_load_texture("crab.png").unwrap(), a);
    });
}

#[test]
fn errors_convert_with_question_mark_into_boxed_errors() {
    fn load(ctx: &mut Ctx) -> Result<Texture, Box<dyn std::error::Error>> {
        Ok(ctx.gfx.try_load_texture("nope.png")?)
    }
    in_tick(|ctx| {
        let err = load(ctx).unwrap_err();
        let err = err.downcast_ref::<Error>().expect("a ponko::Error");
        assert_eq!(err.kind(), ErrorKind::NotFound);
    });
}

#[test]
#[should_panic(expected = "ponko: \"nope.png\" not found in assets/")]
fn panicking_loaders_panic_with_the_error_message_once_prefixed() {
    in_tick(|ctx| {
        ctx.load_texture("nope.png");
    });
}

#[test]
fn panics_and_errors_read_the_same() {
    in_tick(|ctx| {
        let err = ctx.gfx.try_load_font("nope.ttf").unwrap_err();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.gfx.load_font("nope.ttf");
        }))
        .unwrap_err();
        let message = panic.downcast_ref::<String>().unwrap();
        assert_eq!(message, &err.to_string());
        assert!(!message.starts_with("ponko: ponko:"));
    });
}

#[test]
fn reflection_errors_convert_into_error() {
    let err: Error = ReflectError::UnknownResource("Nope".into()).into();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[cfg(feature = "3d")]
mod three {
    use super::*;

    #[test]
    fn missing_models_are_not_found() {
        in_tick(|ctx| {
            assert_eq!(
                ctx.gfx.try_load_mesh("no/such.glb").unwrap_err().kind(),
                ErrorKind::NotFound
            );
            assert_eq!(
                ctx.gfx.try_load_skeleton("no/such.glb").unwrap_err().kind(),
                ErrorKind::NotFound
            );
        });
    }

    #[test]
    fn files_that_are_not_models_fail_to_decode() {
        let path = temp_file("bad.glb", b"not a model");
        in_tick(move |ctx| {
            assert_eq!(ctx.gfx.try_load_mesh(&path).unwrap_err().kind(), ErrorKind::Decode);
            assert_eq!(ctx.gfx.try_load_skeleton(&path).unwrap_err().kind(), ErrorKind::Decode);
        });
    }

    #[test]
    fn out_of_range_materials_are_invalid() {
        in_tick(|ctx| {
            let desc = MaterialDesc {
                metallic: -1.0,
                ..Default::default()
            };
            assert_eq!(
                ctx.gfx.try_create_material(desc).unwrap_err().kind(),
                ErrorKind::Invalid
            );
            assert!(ctx.gfx.try_create_material(MaterialDesc::default()).is_ok());
        });
    }

    #[test]
    fn meshes_check_their_data_and_material() {
        // A material from another game, which made more materials than this one.
        let other = std::rc::Rc::new(std::cell::Cell::new(None));
        let out = other.clone();
        in_tick(move |ctx| {
            for _ in 0..4 {
                out.set(Some(ctx.gfx.create_material(MaterialDesc::default())));
            }
        });
        let foreign: Material = other.get().unwrap();
        in_tick(move |ctx| {
            let stone = ctx.gfx.create_material(MaterialDesc::default());
            let mut broken = MeshData::cube(1.0);
            broken.indices.pop();
            assert_eq!(
                ctx.gfx.try_create_mesh(&broken, stone).unwrap_err().kind(),
                ErrorKind::Invalid
            );
            let err = ctx.gfx.try_create_mesh(&MeshData::cube(1.0), foreign).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::BadHandle);
            assert!(ctx.gfx.try_create_mesh(&MeshData::cube(1.0), stone).is_ok());
        });
    }
}
