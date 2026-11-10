//! The additive native APIs remain available in reduced builds and fail honestly.
#[cfg(not(feature = "physics"))]
#[test]
fn filtered_queries_without_physics_report_the_missing_feature_without_writing_outputs() {
    use ponko_ffi::*;
    let mut found = true;
    let mut hit = CrabbyRayHit::default();
    let mut count = 123;
    // SAFETY: no live context is required on the feature-disabled path; outputs are valid.
    unsafe {
        assert_eq!(
            crabby_ctx_cast_ray_filtered(
                std::ptr::null_mut(),
                Default::default(),
                Default::default(),
                1.0,
                Default::default(),
                &mut found,
                &mut hit
            ),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
        assert!(
            std::ffi::CStr::from_ptr(crabby_last_error())
                .to_str()
                .unwrap()
                .contains("physics")
        );
        assert_eq!(
            crabby_ctx_overlap_ball(
                std::ptr::null_mut(),
                Default::default(),
                1.0,
                Default::default(),
                std::ptr::null_mut(),
                0,
                &mut count
            ),
            CrabbyStatus::CRABBY_INVALID_ARGUMENT
        );
    }
    assert!(found);
    assert_eq!(count, 123);
}
