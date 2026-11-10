//! Lockstep from C: two games over UDP on localhost see the same inputs.
use ponko_ffi::*;
use std::ffi::{CString, c_void};
use std::ptr::null_mut;

use PonkoStatus::*;

#[derive(Default)]
struct Run {
    player: u32,
    players: u32,
    /// Sum of every (player + 1) * tick input seen: the same on both sides.
    sum: u64,
    others: u32,
    state: u32,
    desync: bool,
}

unsafe extern "C" fn tick(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        assert_eq!(ponko_ctx_net_player(ctx, &mut run.player, &mut run.players), PONKO_OK);
        let mine = (u64::from(run.player) + 1) * ponko_ctx_sim_tick(ctx);
        assert_eq!(ponko_ctx_set_input(ctx, (&raw const mine).cast(), 8), PONKO_OK);
        for p in 0..run.players {
            let (mut data, mut size) = (std::ptr::null(), 0);
            assert_eq!(ponko_ctx_input(ctx, p, &mut data, &mut size), PONKO_OK);
            if size == 8 {
                run.sum += data.cast::<u64>().read_unaligned();
                if p != run.player {
                    run.others += 1;
                }
            }
        }
        assert_eq!(ponko_ctx_net_state(ctx, &mut run.state), PONKO_OK);
        let (mut tick, mut who) = (0, 0);
        assert_eq!(
            ponko_ctx_net_desync(ctx, &mut run.desync, &mut tick, &mut who),
            PONKO_OK
        );
    }
}

fn play(game: *mut PonkoGame) -> Run {
    let mut run = Run::default();
    let user = (&raw mut run).cast::<c_void>();
    unsafe {
        assert_eq!(
            ponko_game_sim_system(game, c"tick".as_ptr(), 1, Some(tick), user),
            PONKO_OK
        );
        let mut world = null_mut();
        assert_eq!(
            ponko_game_run_headless(game, 300, None, null_mut(), &mut world),
            PONKO_OK
        );
        ponko_world_free(world);
    }
    run
}

#[test]
fn c_games_play_in_lockstep_over_udp() {
    let mut port = 0;
    let host = unsafe {
        let g = ponko_game_new(c"host".as_ptr(), 1, 1);
        assert_eq!(
            ponko_game_lockstep_host(g, c"127.0.0.1:0".as_ptr(), 2, 2, 30, &mut port),
            PONKO_OK
        );
        g as usize
    };
    assert_ne!(port, 0);
    let host = std::thread::spawn(move || play(host as *mut PonkoGame));
    let addr = CString::new(format!("127.0.0.1:{port}")).unwrap();
    let guest = unsafe {
        let g = ponko_game_new(c"guest".as_ptr(), 1, 1);
        assert_eq!(ponko_game_lockstep_join(g, addr.as_ptr()), PONKO_OK);
        play(g)
    };
    let host = host.join().unwrap();
    assert_eq!((host.player, guest.player, guest.players), (0, 1, 2));
    assert_eq!(host.state, PonkoNetState::PONKO_NET_RUNNING as u32);
    assert_eq!(host.sum, guest.sum, "same inputs on both sides");
    assert!(
        host.others > 250 && guest.others > 250,
        "{} {}",
        host.others,
        guest.others
    );
    assert!(!host.desync && !guest.desync);
}

#[test]
fn bad_sessions_are_errors() {
    unsafe {
        let g = ponko_game_new(c"t".as_ptr(), 1, 1);
        let host =
            |players, delay| ponko_game_lockstep_host(g, c"127.0.0.1:0".as_ptr(), players, delay, 60, null_mut());
        assert_eq!(host(1, 3), PONKO_INVALID_ARGUMENT, "at least 2 players");
        assert_eq!(host(2, 65), PONKO_INVALID_ARGUMENT, "delay at most 64");
        assert_eq!(
            ponko_game_lockstep_join(g, c"not an address".as_ptr()),
            PONKO_INVALID_ARGUMENT
        );
        ponko_game_free(g);
    }
}

#[cfg(not(feature = "steam"))]
#[test]
fn steam_calls_say_the_feature_is_off() {
    unsafe {
        let g = ponko_game_new(c"t".as_ptr(), 1, 1);
        assert_eq!(ponko_game_lockstep_steam_host(g, 480, 2, 3, 60), PONKO_INVALID_ARGUMENT);
        let message = std::ffi::CStr::from_ptr(ponko_last_error())
            .to_string_lossy()
            .into_owned();
        assert!(message.contains("`steam` feature"), "{message}");
        let mut lobby = 1;
        assert_eq!(ponko_steam_lobby_from_args(&mut lobby), PONKO_INVALID_ARGUMENT);
        ponko_game_free(g);
    }
}
