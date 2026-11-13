//! Bundles: tuples of components spawned together.

use std::mem::ManuallyDrop;

use crate::seal::Seal;
use crate::storage::ComponentId;
use crate::world::World;

/// Any `Send + Sync + 'static` type can be a component. No derive needed.
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

/// A tuple of components that are spawned together: `(Pos, Vel)`, `(Pos,)`.
///
/// Implemented for tuples of up to 12 components, by this crate only: the
/// trait is sealed.
///
/// # Safety
/// `write` must call `f` exactly once per element, in the order of
/// `component_ids`, with a pointer to a value whose ownership the callee takes.
pub unsafe trait Bundle: Send + Sync + 'static {
    #[doc(hidden)]
    const SEAL: Seal;
    #[doc(hidden)]
    fn component_ids(world: &mut World, out: &mut Vec<ComponentId>);
    #[doc(hidden)]
    unsafe fn write(self, f: &mut impl FnMut(usize, *const u8));
}

macro_rules! bundle_tuple {
    ($($t:ident $i:tt),*) => {
        unsafe impl<$($t: Component),*> Bundle for ($($t,)*) {
            const SEAL: Seal = Seal;
            fn component_ids(world: &mut World, out: &mut Vec<ComponentId>) {
                $(out.push(world.register::<$t>());)*
            }
            unsafe fn write(self, f: &mut impl FnMut(usize, *const u8)) {
                let me = ManuallyDrop::new(self);
                $(f($i, (&me.$i as *const $t).cast());)*
            }
        }
    };
}
bundle_tuple!(A 0);
bundle_tuple!(A 0, B 1);
bundle_tuple!(A 0, B 1, C 2);
bundle_tuple!(A 0, B 1, C 2, D 3);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10);
bundle_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11);
