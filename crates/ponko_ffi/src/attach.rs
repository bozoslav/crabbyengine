//! Attachments from C: `CrabbyAttached` and the system that places them.
use ponko::Attached;

use crate::types::{CrabbyEntity, CrabbyVec2};
use crate::{CrabbyGame, CrabbyStatus, CrabbyWorld, builtin};

/// Keeps an entity's `CrabbyTransform2D` position and rotation at `offset`
/// (turned and stretched with it) from entity `to`'s; the built-in
/// `Attached` component, same layout as in Rust, so it works as a query
/// column. Placed by the system `crabby_game_follow_attachments` adds; when
/// `to` is despawned, the attached entity is too.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyAttached>(), 20);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyAttached {
    /// The entity to follow.
    pub to: CrabbyEntity,
    /// Where, in the followed entity's space.
    pub offset: CrabbyVec2,
    /// Added to the followed entity's rotation (radians).
    pub rotation: f32,
}

const _: () = {
    assert!(size_of::<CrabbyAttached>() == size_of::<Attached>());
    assert!(align_of::<CrabbyAttached>() == align_of::<Attached>());
    assert!(std::mem::offset_of!(CrabbyAttached, to) == std::mem::offset_of!(Attached, to));
    assert!(std::mem::offset_of!(CrabbyAttached, offset) == std::mem::offset_of!(Attached, offset));
    assert!(std::mem::offset_of!(CrabbyAttached, rotation) == std::mem::offset_of!(Attached, rotation));
};

/// The id of the built-in `Attached` component (`CrabbyAttached` in C),
/// registering it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_attached_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_attached_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<Attached>(world, out_id) }
}

/// Adds the built-in system that moves attached entities next to theirs, as
/// a simulation system at this point in the order: add it after the
/// systems that move things.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_game_follow_attachments(std::ptr::null_mut()) } as u32, 1);
/// ```
/// # Safety
/// `game` must be null or a live game.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_follow_attachments(game: *mut CrabbyGame) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        crate::game::build(game, |g| {
            crate::game::ok(g.sim_system(ponko::systems::follow_attachments))
        })
    }
}
