//! Native spellings are real Rust declarations.
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
