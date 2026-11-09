//! Nav grids and flow fields through the C API.
use ponko_ffi::*;
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

fn v(x: f32, y: f32) -> CrabbyVec2 {
    CrabbyVec2 { x, y }
}

fn costs(nav: *mut CrabbyNavGrid) -> &'static mut [u8] {
    let (mut p, mut len) = (null_mut(), 0);
    assert_eq!(unsafe { crabby_nav_costs(nav, &mut p, &mut len) }, CRABBY_OK);
    unsafe { std::slice::from_raw_parts_mut(p, len) }
}

/// The same lookup crabby.hpp's `flow_view::direction_at` does inline.
fn view_direction(view: &CrabbyFlowView, p: CrabbyVec2) -> CrabbyVec2 {
    let cx = ((p.x - view.origin.x) / view.cell_size).floor();
    let cy = ((p.y - view.origin.y) / view.cell_size).floor();
    if view.cells.is_null() || cx < 0.0 || cy < 0.0 || cx >= view.width as f32 || cy >= view.height as f32 {
        return CrabbyVec2::default();
    }
    let i = (cy as usize + 1) * view.stride + cx as usize + 1;
    unsafe { *view.directions.add(*view.cells.add(i) as usize) }
}

#[test]
fn fields_lead_around_walls_written_through_the_cost_pointer() {
    let nav = crabby_nav_new(10, 10, 2.0);
    // A wall across row 5 with a gap at x = 9.
    let c = costs(nav);
    assert_eq!(c.len(), 100);
    for x in 0..9 {
        c[5 * 10 + x] = CRABBY_NAV_WALL;
    }
    let flow = crabby_flow_new();
    let goal = v(1.0, 19.0);
    unsafe {
        assert_eq!(crabby_flow_compute(flow, nav, &goal, 1), CRABBY_OK);
        // Above the wall, everyone heads for the gap on the right.
        assert!(crabby_flow_direction(flow, v(15.0, 7.0)).x > 0.0, "toward the gap");
        assert_eq!(
            crabby_flow_direction(flow, v(1.0, 11.0)),
            CrabbyVec2::default(),
            "on the wall"
        );
        let mut d = 0.0;
        assert_eq!(crabby_flow_distance(flow, goal, &mut d), CRABBY_OK);
        assert_eq!(d, 0.0);
        assert_eq!(crabby_flow_distance(flow, v(1.0, 1.0), &mut d), CRABBY_OK);
        assert!(d > 18.0, "longer than the 18 units straight down: {d}");
        crabby_flow_free(flow);
        crabby_nav_free(nav);
    }
}

#[test]
fn views_match_the_direction_function_everywhere() {
    let nav = crabby_nav_new(16, 9, 4.0);
    unsafe { assert_eq!(crabby_nav_set_origin(nav, v(-32.0, 10.0)), CRABBY_OK) };
    let c = costs(nav);
    for (i, cost) in c.iter_mut().enumerate() {
        *cost = match i % 7 {
            0 => CRABBY_NAV_WALL,
            1 => 5,
            _ => 1,
        };
    }
    let flow = crabby_flow_new();
    let goals = [v(-30.0, 12.0), v(30.0, 44.0)];
    unsafe { assert_eq!(crabby_flow_compute(flow, nav, goals.as_ptr(), 2), CRABBY_OK) };
    let mut view = MaybeUninit::uninit();
    unsafe { assert_eq!(crabby_flow_view(flow, view.as_mut_ptr()), CRABBY_OK) };
    let view = unsafe { view.assume_init() };
    assert_eq!((view.width, view.height, view.cell_size), (16, 9, 4.0));
    let mut moving = 0;
    for y in -2..40 {
        for x in -12..40 {
            let p = v(-32.0 + x as f32 * 2.0 + 0.5, 10.0 + y as f32 * 1.0 + 0.25);
            let d = unsafe { crabby_flow_direction(flow, p) };
            assert_eq!(view_direction(&view, p), d, "at {p:?}");
            moving += (d != CrabbyVec2::default()) as usize;
        }
    }
    assert!(moving > 100, "most points have somewhere to go: {moving}");
    unsafe {
        crabby_flow_free(flow);
        crabby_nav_free(nav);
    }
}

#[test]
fn unreachable_points_report_not_found() {
    let nav = crabby_nav_new(4, 1, 1.0);
    costs(nav)[1] = CRABBY_NAV_WALL;
    let flow = crabby_flow_new();
    let goal = v(3.5, 0.5);
    let mut d = 7.0;
    unsafe {
        assert_eq!(crabby_flow_compute(flow, nav, &goal, 1), CRABBY_OK);
        assert_eq!(crabby_flow_distance(flow, v(0.5, 0.5), &mut d), CRABBY_NOT_FOUND);
        assert_eq!(d, 7.0, "untouched");
        // No goals at all: nothing is reachable.
        assert_eq!(crabby_flow_compute(flow, nav, null(), 0), CRABBY_OK);
        assert_eq!(crabby_flow_distance(flow, v(3.5, 0.5), &mut d), CRABBY_NOT_FOUND);
        crabby_flow_free(flow);
        crabby_nav_free(nav);
    }
}

#[test]
fn bad_arguments_are_rejected() {
    assert!(crabby_nav_new(0, 4, 1.0).is_null());
    assert!(crabby_nav_new(4, 4, -1.0).is_null());
    assert!(crabby_nav_new(4, 4, f32::NAN).is_null());
    assert!(crabby_nav_new(1 << 15, 1 << 14, 1.0).is_null(), "more than 2^28 cells");
    let nav = crabby_nav_new(4, 4, 1.0);
    let flow = crabby_flow_new();
    let mut d = 0.0;
    unsafe {
        assert_eq!(crabby_flow_compute(flow, nav, null(), 3), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_flow_compute(null_mut(), nav, null(), 0), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_flow_compute(flow, null(), null(), 0), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_nav_costs(nav, null_mut(), null_mut()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_flow_view(null(), null_mut()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_flow_distance(null(), v(0.0, 0.0), &mut d),
            CRABBY_INVALID_ARGUMENT
        );
        crabby_flow_free(flow);
        crabby_nav_free(nav);
    }
}
