//! A regression check for the steady-state action input path.
//!
//! One test per binary, and a thread-local counter: only the thread running
//! the game counts, so other threads in the process can't disturb the
//! zero-allocation assertion.
use ponko::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}
fn record_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCS.try_with(|n| n.set(n.get() + 1));
    }
}
// SAFETY: all allocation and deallocation operations delegate unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: the allocator caller supplies a valid layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: the allocator caller supplies a valid layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        // SAFETY: the allocation and layout come from the allocator caller.
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the allocation and layout come from the allocator caller.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn input_events_and_action_queries_allocate_nothing_after_installation() {
    let mut frames = 0;
    let mut ids = None;
    Game::new("allocations", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            // Resolved once (it may allocate), before tracking starts.
            let [jump, throttle, movement] =
                *ids.get_or_insert_with(|| ctx.input.action_ids(&["jump", "throttle", "move"]));
            if frames == 0 {
                ALLOCS.set(0);
                TRACK.set(true);
            }
            // Tracking remains enabled between systems, including frame resets.
            let pad = Gamepad::from_slot(0).unwrap();
            for i in 0..128 {
                ctx.input.feed(InputEvent::Key(Key::Space, i % 2 == 0));
                ctx.input
                    .feed(InputEvent::GamepadAxis(pad, GamepadAxis::LeftStickX, 0.6));
                std::hint::black_box(ctx.input.action_down("jump"));
                std::hint::black_box(ctx.input.action_pressed("jump"));
                std::hint::black_box(ctx.input.action_released("jump"));
                std::hint::black_box(ctx.input.action_axis("throttle"));
                std::hint::black_box(ctx.input.action_axis2("move"));
                std::hint::black_box(ctx.input.action_down(jump));
                std::hint::black_box(ctx.input.action_pressed(jump));
                std::hint::black_box(ctx.input.action_released(jump));
                std::hint::black_box(ctx.input.action_axis(throttle));
                std::hint::black_box(ctx.input.action_axis2(movement));
            }
            frames += 1;
            if frames == 3 {
                TRACK.set(false);
                assert_eq!(ALLOCS.get(), 0);
            }
        })
        .run_headless(3, |ctx| {
            let mut map = ActionMap::default();
            let pad = Gamepad::from_slot(0).unwrap();
            map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
            map.set(
                "throttle",
                Action::Axis(vec![AxisBinding::Gamepad {
                    pad,
                    axis: GamepadAxis::RightTrigger,
                    deadzone: 0.2,
                }]),
            );
            map.set(
                "move",
                Action::Axis2(vec![Axis2Binding::Gamepad {
                    pad,
                    stick: GamepadStick::Left,
                    deadzone: 0.2,
                }]),
            );
            ctx.input.set_actions(map);
        });
}
