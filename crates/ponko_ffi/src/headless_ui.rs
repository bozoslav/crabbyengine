//! Headless runs with frames, and input scenarios that act on the retained UI
//! by node name, from C (Rust's `Game::headless_frames`,
//! `InputScenario::click`, `InputScenario::type_text`, `ScenarioCheck::ui`
//! and `Ui::is_shown`).
use std::ffi::c_char;

use ponko::scenario::{InputScenario, ScenarioCheck};

use crate::ctx::{out, text};
use crate::game::build;
use crate::scenario::{PonkoInputScenario, edit, expectation};
use crate::ui_retained::{existing, optional_text, with_ui};
use crate::{PonkoCtx, PonkoGame, PonkoStatus, boundary};

/// Makes `ponko_game_run_headless` (and the profiled run) run whole frames
/// instead of simulation ticks alone (Rust's `Game::headless_frames`): the
/// retained UI takes each frame's input, then one tick, then the frame,
/// animate, late and draw systems. The window is virtual, of the size given
/// to `ponko_game_new`; nothing is drawn and no GPU or window is opened.
/// An input scenario's input for a tick is fed before the UI takes it, and
/// its checks run after the whole frame. Off by default.
/// ```
/// assert_eq!(unsafe { ponko_ffi::ponko_game_headless_frames(std::ptr::null_mut(), true) } as u32, 1);
/// ```
/// # Safety
/// `game` null or live, never run.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_game_headless_frames(game: *mut PonkoGame, on: bool) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe { build(game, |g| (g.headless_frames(on), Ok(()))) }
}

/// Adds a click on the retained UI's node named `node` at tick `tick`
/// (Rust's `InputScenario::click`): pressed and released at its centre in
/// the frame of that tick, through the UI's hit testing. The run fails at
/// that tick when no node has the name, it is not shown, or something else
/// is under its centre. One UI step is used per frame.
/// ```
/// let s = ponko_ffi::ponko_input_scenario_new();
/// assert_eq!(unsafe { ponko_ffi::ponko_input_scenario_click(s, 3, c"menu.play".as_ptr()) } as u32, 0);
/// unsafe { ponko_ffi::ponko_input_scenario_free(s) };
/// ```
/// # Safety
/// `scenario` null or live; `node` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_input_scenario_click(
    scenario: *mut PonkoInputScenario,
    tick: u64,
    node: *const c_char,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        edit(scenario, || {
            let node = text(node)?.to_owned();
            Ok(move |s: InputScenario| s.click(tick, node))
        })
    }
}

/// Adds typing `typed` into the retained UI's text input named `node` at
/// tick `tick` (Rust's `InputScenario::type_text`): a click on it, then the
/// text at the end of what it holds, in one frame. Fails the run as
/// `ponko_input_scenario_click` does, and when the node is not a text input.
/// ```
/// let s = ponko_ffi::ponko_input_scenario_new();
/// assert_eq!(unsafe { ponko_ffi::ponko_input_scenario_type_text(s, 3, c"chat".as_ptr(), c"hi".as_ptr()) } as u32, 0);
/// unsafe { ponko_ffi::ponko_input_scenario_free(s) };
/// ```
/// # Safety
/// `scenario` null or live; `node` and `typed` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_input_scenario_type_text(
    scenario: *mut PonkoInputScenario,
    tick: u64,
    node: *const c_char,
    typed: *const c_char,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        edit(scenario, || {
            let node = text(node)?.to_owned();
            let typed = text(typed)?.to_owned();
            Ok(move |s: InputScenario| s.type_text(tick, node, typed))
        })
    }
}

/// Adds a check of the retained UI's node named `node` at tick `tick`
/// (Rust's `ScenarioCheck::ui`). `exists`, `shown` and `focused` are each 1
/// (expected true), 0 (expected false) or -1 (not checked); `expected_text`
/// is the node's text, or null for not checked. Every check but `exists`
/// needs the node to exist.
/// ```
/// let s = ponko_ffi::ponko_input_scenario_new();
/// assert_eq!(unsafe { ponko_ffi::ponko_input_scenario_expect_ui(s, 3, c"menu.play".as_ptr(), 1, 1, -1, c"Play".as_ptr()) } as u32, 0);
/// unsafe { ponko_ffi::ponko_input_scenario_free(s) };
/// ```
/// # Safety
/// `scenario` null or live; `node` NUL-terminated; `expected_text` null or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_input_scenario_expect_ui(
    scenario: *mut PonkoInputScenario,
    tick: u64,
    node: *const c_char,
    exists: i32,
    shown: i32,
    focused: i32,
    expected_text: *const c_char,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    unsafe {
        edit(scenario, || {
            let mut check = ScenarioCheck::ui(text(node)?);
            if let Some(want) = expectation(exists)? {
                check = check.exists(want);
            }
            if let Some(want) = expectation(shown)? {
                check = check.shown(want);
            }
            if let Some(want) = expectation(focused)? {
                check = check.focused(want);
            }
            if !expected_text.is_null() {
                check = check.text(optional_text(expected_text)?);
            }
            Ok(move |s: InputScenario| s.expect(tick, check))
        })
    }
}

/// Writes whether node `id` of the UI `target` is shown (Rust's
/// `Ui::is_shown`): on a screen of the stack and neither it nor a node
/// above it hidden. `PONKO_NOT_FOUND` for a node that doesn't exist.
/// ```
/// let mut shown = false;
/// assert_eq!(unsafe { ponko_ffi::ponko_ctx_ui_is_shown(std::ptr::null_mut(), ponko_ffi::PONKO_UI_SCREEN, 0, &mut shown) } as u32, 1);
/// ```
/// # Safety
/// `ctx` null or live; `out_shown` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_ctx_ui_is_shown(
    ctx: *mut PonkoCtx,
    target: u64,
    id: u64,
    out_shown: *mut bool,
) -> PonkoStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        with_ui(ctx, target, |ui| {
            let id = existing(ui, id)?;
            out(out_shown, ui.is_shown(id))
        })
    })
}
