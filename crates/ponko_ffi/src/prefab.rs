//! Prefabs with children from C: the parts a prefab spawned with its root,
//! and despawning them together (see `PrefabParts` and `Ctx::despawn_prefab`
//! in Rust). `ponko_ctx_spawn_prefab` returns the root of a tree like any
//! other prefab.
use crate::ctx::{PonkoCtx, ctx_access};
use crate::{PonkoStatus, boundary};

/// Writes the parts `ponko_ctx_spawn_prefab` spawned with prefab root `root`
/// (its `PrefabParts`) to `out_entities`, up to `capacity` of them, and how
/// many there are to `*out_count` (0 for an entity that isn't a prefab tree's
/// root). Call it with capacity 0 to size a buffer.
/// # Safety
/// Live ctx; `out_entities` writable for `capacity` ids (null with 0);
/// `out_count` writable.
/// ```
/// let mut n = 0;
/// let status = unsafe { ponko_ffi::ponko_ctx_prefab_parts(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0, &mut n) };
/// assert_eq!(status as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_prefab_parts(
    ctx: *mut PonkoCtx,
    root: u64,
    out_entities: *mut u64,
    capacity: usize,
    out_count: *mut usize,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract; `collect` checks and caps every write.
    boundary(|| unsafe {
        let parts = ctx_access(ctx, |c| {
            Ok(c.world
                .get::<ponko::prelude::PrefabParts>(ponko_ecs::Entity::from_bits(root))
                .map(|p| p.0.clone())
                .unwrap_or_default())
        })?;
        let mut n = 0;
        crate::ctx::collect(out_entities, capacity, out_count, &mut n, |sink| {
            for e in parts {
                sink.push(e.to_bits());
            }
        })
    })
}

/// Despawns prefab root `root` and the parts spawned with it, writing how
/// many entities that was to `*out_count` (which may be null). An entity
/// without parts is despawned alone.
/// # Safety
/// Live ctx; `out_count` writable or null.
/// ```
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_despawn_prefab(std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_despawn_prefab(ctx: *mut PonkoCtx, root: u64, out_count: *mut usize) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let n = ctx_access(ctx, |c| Ok(c.despawn_prefab(ponko_ecs::Entity::from_bits(root))))?;
        if !out_count.is_null() {
            out_count.write(n);
        }
        Ok(())
    })
}
