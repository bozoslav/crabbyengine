//! Events from C: ponko_ctx_send_event and ponko_ctx_read_events.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hit {
    tick: u64,
    damage: f32,
}

const HIT: u32 = 7;

#[derive(Default)]
struct Log {
    seen: Vec<(u64, Hit)>,
    statuses: Vec<PonkoStatus>,
    health: u32,
}

fn user<T>(t: &mut T) -> *mut c_void {
    (t as *mut T).cast()
}

unsafe fn run(game: *mut PonkoGame, ticks: u64, setup: PonkoSystemFn, user: *mut c_void) {
    let mut world = null_mut();
    unsafe {
        assert_eq!(ponko_game_run_headless(game, ticks, setup, user, &mut world), PONKO_OK);
        ponko_world_free(world);
    }
}

unsafe extern "C" fn send_two(ctx: *mut PonkoCtx, _: *mut c_void) {
    unsafe {
        let tick = ponko_ctx_sim_tick(ctx);
        for damage in [1.0, 2.0] {
            let hit = Hit { tick, damage };
            let st = ponko_ctx_send_event(ctx, HIT, (&raw const hit).cast(), size_of::<Hit>());
            assert_eq!(st, PONKO_OK);
        }
    }
}

unsafe extern "C" fn read_hits(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let log = &mut *user.cast::<Log>();
        let (mut data, mut count) = (null(), 0);
        let st = ponko_ctx_read_events(ctx, HIT, size_of::<Hit>(), &mut data, &mut count);
        assert_eq!(st, PONKO_OK);
        assert_eq!(data as usize % align_of::<Hit>(), 0);
        let hits = std::slice::from_raw_parts(data.cast::<Hit>(), count);
        let tick = ponko_ctx_sim_tick(ctx);
        log.seen.extend(hits.iter().map(|h| (tick, *h)));
    }
}

#[test]
fn c_systems_send_and_read_events_once_each() {
    let mut log = Log::default();
    unsafe {
        let g = ponko_game_new(c"events".as_ptr(), 1, 1);
        ponko_game_sim_system(g, c"before".as_ptr(), 1, Some(read_hits), user(&mut log));
        ponko_game_sim_system(g, c"send".as_ptr(), 1, Some(send_two), null_mut());
        run(g, 3, None, null_mut());
    }
    // The reader runs before the sender, so it sees each tick's hits on the next tick.
    let seen: Vec<(u64, u64, f32)> = log.seen.iter().map(|(t, h)| (*t, h.tick, h.damage)).collect();
    assert_eq!(seen, [(1, 0, 1.0), (1, 0, 2.0), (2, 1, 1.0), (2, 1, 2.0)]);
}

unsafe extern "C" fn wrong_sizes(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let log = &mut *user.cast::<Log>();
        let hit = Hit { tick: 0, damage: 0.0 };
        let p = (&raw const hit).cast();
        log.statuses.push(ponko_ctx_send_event(ctx, HIT, p, size_of::<Hit>()));
        log.statuses.push(ponko_ctx_send_event(ctx, HIT, p, 4));
        let (mut data, mut count) = (null(), 0);
        log.statuses
            .push(ponko_ctx_read_events(ctx, HIT, 4, &mut data, &mut count));
        log.statuses
            .push(ponko_ctx_send_event(ctx, HIT, null(), size_of::<Hit>()));
        log.statuses.push(ponko_ctx_read_events(
            ctx,
            HIT,
            size_of::<Hit>(),
            null_mut(),
            &mut count,
        ));
    }
}

#[test]
fn mismatched_sizes_and_null_pointers_are_refused() {
    let mut log = Log::default();
    unsafe {
        let g = ponko_game_new(c"events".as_ptr(), 1, 1);
        run(g, 0, Some(wrong_sizes), user(&mut log));
    }
    assert_eq!(
        log.statuses,
        [
            PONKO_OK,
            PONKO_INVALID_ARGUMENT,
            PONKO_INVALID_ARGUMENT,
            PONKO_INVALID_ARGUMENT,
            PONKO_INVALID_ARGUMENT
        ]
    );
}

struct QueryUser {
    ctx: *mut PonkoCtx,
    statuses: Vec<PonkoStatus>,
}

unsafe extern "C" fn send_per_entity(chunk: *const PonkoChunk, user: *mut c_void) {
    unsafe {
        let u = &mut *user.cast::<QueryUser>();
        for _ in 0..(*chunk).count {
            let hit = Hit { tick: 0, damage: 5.0 };
            u.statuses.push(ponko_ctx_send_event(
                u.ctx,
                HIT,
                (&raw const hit).cast(),
                size_of::<Hit>(),
            ));
        }
    }
}

unsafe extern "C" fn spawn_three(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let world = ponko_ctx_world(ctx);
        let log = &mut *user.cast::<Log>();
        assert_eq!(
            ponko_component_register(world, c"Health".as_ptr(), 4, 4, &mut log.health),
            PONKO_OK
        );
        for hp in [1u32, 2, 3] {
            let v = PonkoValue {
                component: log.health,
                data: (&raw const hp).cast(),
                size: 4,
            };
            let mut e = 0;
            assert_eq!(ponko_spawn(world, &v, 1, &mut e), PONKO_OK);
        }
    }
}

unsafe extern "C" fn query_sends(ctx: *mut PonkoCtx, log: *mut c_void) {
    unsafe {
        let log = &mut *log.cast::<Log>();
        let world = ponko_ctx_world(ctx);
        let mut q = QueryUser {
            ctx,
            statuses: Vec::new(),
        };
        let ids = [log.health];
        assert_eq!(
            ponko_query(world, ids.as_ptr(), 1, Some(send_per_entity), user(&mut q)),
            PONKO_OK
        );
        assert_eq!(
            ponko_query_par(world, ids.as_ptr(), 1, Some(send_per_entity), user(&mut q)),
            PONKO_OK
        );
        log.statuses = q.statuses;
    }
}

#[test]
fn events_can_be_sent_inside_queries_but_not_parallel_ones() {
    let mut log = Log::default();
    unsafe {
        let g = ponko_game_new(c"events".as_ptr(), 1, 1);
        ponko_game_sim_system(g, c"send".as_ptr(), 1, Some(query_sends), user(&mut log));
        ponko_game_sim_system(g, c"read".as_ptr(), 1, Some(read_hits), user(&mut log));
        run(g, 1, Some(spawn_three), user(&mut log));
    }
    assert_eq!(&log.statuses[..3], [PONKO_OK; 3]);
    assert!(log.statuses[3..].iter().all(|s| *s == PONKO_BUSY));
    assert_eq!(log.seen.len(), 3);
}
