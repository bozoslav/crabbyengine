//! Primary native spellings are real Rust declarations, with compatible layouts.
use ponko_ffi::*;

#[test]
fn primary_names_are_real_types_and_functions() {
    assert_eq!(ponko_abi_version(), 2);
    assert!(std::any::type_name::<PonkoWorld>().ends_with("PonkoWorld"));
    let world = ponko_world_new();
    assert!(!world.is_null());
    unsafe {
        assert_eq!(ponko_world_free(world), PonkoStatus::PONKO_OK);
    }
}

#[cfg(feature = "legacy-names")]
#[test]
#[allow(deprecated)]
fn legacy_aliases_and_forwarders_interoperate() {
    use CrabbyStatus::*;
    assert_eq!(crabby_abi_version(), ponko_abi_version());
    assert_eq!(std::mem::size_of::<CrabbyChunk>(), std::mem::size_of::<PonkoChunk>());
    assert_eq!(CRABBY_OK, PONKO_OK);
    let world: *mut CrabbyWorld = ponko_world_new();
    unsafe {
        assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
    }
    let world: *mut PonkoWorld = crabby_world_new();
    unsafe {
        assert_eq!(ponko_world_free(world), PonkoStatus::PONKO_OK);
    }
}
