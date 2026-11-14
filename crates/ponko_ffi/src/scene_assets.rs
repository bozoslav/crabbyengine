//! Asset names in scene fields from C (Rust `Assets::scene_asset_field`):
//! what to load before a scene or prefab that names it is spawned.
use crate::ctx::{ctx_access, text, with_ctx};
use crate::{PonkoCtx, PonkoStatus, boundary, invalid};
use ponko::prelude::{Error, ErrorKind};
use std::ffi::{CString, c_char, c_void};

/// Loads the asset `name` for a scene field (`ponko_ctx_scene_asset_field`):
/// returns `PONKO_OK` (0) when it is loaded, another `PonkoStatus` to fail
/// the scene load. `ctx` is valid until it returns; use it, not a ctx of
/// your own system.
pub type PonkoSceneAssetFn =
    Option<unsafe extern "C" fn(ctx: *mut PonkoCtx, name: *const c_char, user: *mut c_void) -> u32>;

/// Says that field `field` of the reflected component registered as
/// `component` names an asset, and how to load it (Rust
/// `Assets::scene_asset_field`): before a scene or prefab is spawned
/// (`ponko_ctx_load_scene`, `ponko_ctx_spawn_prefab`, the editor and the
/// control endpoint), `load` is called with every name found there, the
/// way the textures of sprites and meshes of models are loaded first. The
/// component keeps the name (a string field, or a list of them), not a
/// handle: a system gets the handle by loading the name again, which
/// returns the cached one.
///
/// A `load` that returns anything but `PONKO_OK` fails the scene load
/// before any entity changes, with the component, field and name in the
/// message. Calling this again for the same component and field replaces
/// its loader. `load` and `user` must stay valid while the game runs.
/// ```
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_scene_asset_field(std::ptr::null_mut(), c"ShopItem".as_ptr(), c"icon".as_ptr(), None, std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `component` and `field` NUL-terminated; `load` and
/// `user` valid while the game runs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_scene_asset_field(
    ctx: *mut PonkoCtx,
    component: *const c_char,
    field: *const c_char,
    load: PonkoSceneAssetFn,
    user: *mut c_void,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let component = text(component)?;
        let field = text(field)?;
        let load = load.ok_or_else(|| invalid("load is null"))?;
        if component.is_empty() || field.is_empty() {
            return Err(invalid("component and field need names"));
        }
        ctx_access(ctx, |c| {
            c.assets.scene_asset_field_named(component, field, move |ctx, name| {
                let c_name = CString::new(name)
                    .map_err(|_| Error::new(ErrorKind::Invalid, "asset name contains NUL").with_path(name))?;
                // SAFETY: the caller of ponko_ctx_scene_asset_field promised
                // `load` and `user` stay valid while the game runs; the ctx
                // and the name live for the call.
                let status = with_ctx(ctx, |c| load(c, c_name.as_ptr(), user));
                if status == PonkoStatus::PONKO_OK as u32 {
                    return Ok(());
                }
                let kind = if status == PonkoStatus::PONKO_NOT_FOUND as u32 {
                    ErrorKind::NotFound
                } else {
                    ErrorKind::Decode
                };
                Err(Error::new(kind, format!("the loader returned status {status}")).with_path(name))
            });
            Ok(())
        })
    })
}
