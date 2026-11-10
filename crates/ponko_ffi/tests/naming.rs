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
    assert_eq!(CRABBY_OK as u32, PonkoStatus::PONKO_OK as u32);
    let world: *mut CrabbyWorld = ponko_world_new();
    unsafe {
        assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK);
    }
    let world: *mut PonkoWorld = crabby_world_new();
    unsafe {
        assert_eq!(ponko_world_free(world), PonkoStatus::PONKO_OK);
    }
}

#[cfg(feature = "legacy-names")]
#[test]
#[allow(deprecated)]
fn explicit_legacy_variant_imports_and_enum_fields_stay_usable() {
    use ponko_ffi::{
        CrabbyAudioRolloff::*, CrabbyJointMotionKind::CRABBY_JOINT_LOCKED, CrabbyStatus::CRABBY_OK,
        crabby_audio_settings3d_default, crabby_joint3d_fixed, crabby_world_free, crabby_world_new,
    };
    let world = crabby_world_new();
    unsafe {
        assert_eq!(crabby_world_free(world), CRABBY_OK);
    }
    let mut audio = crabby_audio_settings3d_default();
    assert_eq!(audio.rolloff, CRABBY_ROLLOFF_LOGARITHMIC);
    audio.rolloff = CRABBY_ROLLOFF_LINEAR;
    assert_eq!(audio.rolloff, CRABBY_ROLLOFF_LINEAR);
    let joint = crabby_joint3d_fixed(0, 0);
    assert_eq!(joint.axes[0].kind, CRABBY_JOINT_LOCKED);
}
