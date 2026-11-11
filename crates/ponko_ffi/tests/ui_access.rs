//! The UI's accessibility semantics through the C API: names, descriptions
//! and live regions, the snapshot and what changed in it, and requests from
//! assistive technology, in headless runs (no window).
use ponko_ffi::*;
use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::{null, null_mut};

use PonkoStatus::*;

type Ctx = *mut PonkoCtx;

const SCREEN: PonkoVec2 = PonkoVec2 { x: 800.0, y: 600.0 };

struct Scenario {
    body: unsafe fn(Ctx),
    failure: Option<String>,
}

unsafe extern "C" fn trampoline(ctx: Ctx, user: *mut c_void) {
    let scenario = unsafe { &mut *user.cast::<Scenario>() };
    let body = scenario.body;
    if let Err(payload) = catch_unwind(AssertUnwindSafe(|| unsafe { body(ctx) })) {
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()));
        scenario.failure = Some(message.unwrap_or_else(|| "panicked".into()));
    }
}

/// Runs `body` once as a sim system of a headless game and reports its failure.
fn run(body: unsafe fn(Ctx)) {
    let mut scenario = Scenario { body, failure: None };
    unsafe {
        let game = ponko_game_new(c"ui".as_ptr(), 1, 1);
        assert_eq!(
            ponko_game_sim_system(
                game,
                c"s".as_ptr(),
                1,
                Some(trampoline),
                (&mut scenario as *mut Scenario).cast()
            ),
            PONKO_OK
        );
        let mut world = null_mut();
        assert_eq!(ponko_game_run_headless(game, 1, None, null_mut(), &mut world), PONKO_OK);
        ponko_world_free(world);
    }
    if let Some(failure) = scenario.failure {
        panic!("{failure}");
    }
}

fn last_error() -> String {
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}

unsafe fn id_of(status: impl FnOnce(*mut u64) -> PonkoStatus) -> u64 {
    let mut id = PONKO_UI_NONE;
    assert_eq!(status(&mut id), PONKO_OK, "{}", last_error());
    id
}

/// Reads a string the two-call way: the length, then the bytes.
unsafe fn string(read: impl Fn(*mut c_char, usize, *mut usize) -> PonkoStatus) -> String {
    let mut len = 0;
    assert_eq!(read(null_mut(), 0, &mut len), PONKO_OK, "{}", last_error());
    let mut buffer = vec![0u8; len + 1];
    assert_eq!(read(buffer.as_mut_ptr().cast(), buffer.len(), &mut len), PONKO_OK);
    assert_eq!(buffer[len], 0);
    String::from_utf8(buffer[..len].to_vec()).unwrap()
}

unsafe fn frame(ctx: Ctx, input: PonkoUiInput) {
    assert_eq!(
        unsafe { ponko_ctx_ui_frame(ctx, PONKO_UI_SCREEN, SCREEN, &input) },
        PONKO_OK,
        "{}",
        last_error()
    );
}

unsafe fn events(ctx: Ctx) -> Vec<(PonkoUiEventKind, u64)> {
    let mut all = Vec::new();
    loop {
        let mut event = PonkoUiEvent {
            kind: PonkoUiEventKind::PONKO_UI_EVENT_CLICKED,
            id: 0,
        };
        match unsafe { ponko_ctx_ui_next_event(ctx, PONKO_UI_SCREEN, &mut event) } {
            PONKO_OK => all.push((event.kind, event.id)),
            PONKO_NOT_FOUND => return all,
            other => panic!("{other:?}: {}", last_error()),
        }
    }
}

/// The sample menu: title, button, slider, text input, toggle, disabled button.
struct Menu {
    root: u64,
    title: u64,
    play: u64,
    volume: u64,
    name: u64,
    music: u64,
    quit: u64,
}

unsafe fn menu(ctx: Ctx) -> Menu {
    unsafe {
        let fill = ponko_ui_layout_fill();
        let row = ponko_ui_layout_row(30.0);
        let s = PONKO_UI_SCREEN;
        let root = id_of(|out| ponko_ctx_ui_add_panel(ctx, s, PONKO_UI_NONE, &fill, out));
        let title = id_of(|out| ponko_ctx_ui_add_label(ctx, s, root, c"Settings".as_ptr(), &row, out));
        let play = id_of(|out| ponko_ctx_ui_add_button(ctx, s, root, c"Play".as_ptr(), &row, out));
        let volume = id_of(|out| ponko_ctx_ui_add_slider(ctx, s, root, 0.5, &row, out));
        let name = id_of(|out| ponko_ctx_ui_add_text_input(ctx, s, root, c"Ann".as_ptr(), &row, out));
        let music = id_of(|out| ponko_ctx_ui_add_toggle(ctx, s, root, c"Music".as_ptr(), true, &row, out));
        let quit = id_of(|out| ponko_ctx_ui_add_button(ctx, s, root, c"Quit".as_ptr(), &row, out));
        assert_eq!(ponko_ctx_ui_set_enabled(ctx, s, quit, false), PONKO_OK);
        assert_eq!(
            ponko_ctx_ui_set_accessible_name(ctx, s, volume, c"Volume".as_ptr()),
            PONKO_OK
        );
        assert_eq!(
            ponko_ctx_ui_set_accessible_name(ctx, s, name, c"Player name".as_ptr()),
            PONKO_OK
        );
        assert_eq!(ponko_ctx_ui_push_screen(ctx, s, root, false), PONKO_OK);
        frame(ctx, ponko_ui_input_default());
        Menu {
            root,
            title,
            play,
            volume,
            name,
            music,
            quit,
        }
    }
}

unsafe fn update(access: *mut PonkoUiAccess, ctx: Ctx) -> (PonkoUiAccessChanges, Vec<u64>, Vec<u64>) {
    unsafe {
        let mut changes = PonkoUiAccessChanges::default();
        assert_eq!(
            ponko_ui_access_update(access, ctx, PONKO_UI_SCREEN, &mut changes),
            PONKO_OK,
            "{}",
            last_error()
        );
        let changed = (0..changes.changed_count as usize)
            .map(|i| id_of(|out| ponko_ui_access_changed(access, i, out)))
            .collect();
        let removed = (0..changes.removed_count as usize)
            .map(|i| id_of(|out| ponko_ui_access_removed(access, i, out)))
            .collect();
        (changes, changed, removed)
    }
}

unsafe fn node(access: *mut PonkoUiAccess, id: u64) -> PonkoUiAccessNode {
    let mut n = PonkoUiAccessNode::default();
    assert_eq!(
        unsafe { ponko_ui_access_find(access, id, &mut n) },
        PONKO_OK,
        "{}",
        last_error()
    );
    n
}

unsafe fn action(ctx: Ctx, kind: PonkoUiAccessActionKind, id: u64, value: f32, text: *const c_char) {
    unsafe {
        assert_eq!(
            ponko_ctx_ui_access_action(ctx, PONKO_UI_SCREEN, kind as u32, id, value, text),
            PONKO_OK,
            "{}",
            last_error()
        );
    }
}

// ---- names, descriptions, live regions and the text ----------------------------

unsafe fn names_descriptions_and_live_regions(ctx: Ctx) {
    unsafe {
        let m = menu(ctx);
        let s = PONKO_UI_SCREEN;
        let name = |id| string(|b, c, l| ponko_ctx_ui_accessible_name(ctx, s, id, b, c, l));
        let description = |id| string(|b, c, l| ponko_ctx_ui_accessible_description(ctx, s, id, b, c, l));
        assert_eq!(name(m.play), "Play", "a button's name is its text");
        assert_eq!(name(m.volume), "Volume");
        assert_eq!(name(m.name), "Player name", "a text input's text is its value");
        assert_eq!(description(m.play), "");

        assert_eq!(
            ponko_ctx_ui_set_accessible_description(ctx, s, m.quit, c"Closes the game".as_ptr()),
            PONKO_OK
        );
        assert_eq!(description(m.quit), "Closes the game");
        assert_eq!(ponko_ctx_ui_set_live_region(ctx, s, m.title, 2), PONKO_OK);
        let mut live = PonkoUiLive::PONKO_UI_LIVE_OFF;
        assert_eq!(ponko_ctx_ui_live_region(ctx, s, m.title, &mut live), PONKO_OK);
        assert_eq!(live, PonkoUiLive::PONKO_UI_LIVE_ASSERTIVE);

        // The whole tree as text, the same text the Rust snapshot prints.
        let text = string(|b, c, l| ponko_ctx_ui_access_text(ctx, s, b, c, l));
        assert_eq!(
            text,
            "group\n  \
             label \"Settings\" live=assertive\n  \
             button \"Play\" focusable\n  \
             slider \"Volume\" value=0.50 focusable\n  \
             text-input \"Player name\" value=\"Ann\" focusable\n  \
             checkbox \"Music\" checked focusable\n  \
             button \"Quit\" description=\"Closes the game\" disabled\n"
        );

        // Null or empty takes the override off.
        assert_eq!(ponko_ctx_ui_set_accessible_name(ctx, s, m.volume, null()), PONKO_OK);
        assert_eq!(name(m.volume), "");
        assert_eq!(
            ponko_ctx_ui_set_accessible_description(ctx, s, m.quit, c"".as_ptr()),
            PONKO_OK
        );
        assert_eq!(description(m.quit), "");
        let _ = m.root;
    }
}
#[test]
fn names_descriptions_and_live_regions_are_set_read_and_printed() {
    run(names_descriptions_and_live_regions);
}

// ---- the snapshot --------------------------------------------------------------

unsafe fn snapshot_nodes_and_changes(ctx: Ctx) {
    unsafe {
        let m = menu(ctx);
        let s = PONKO_UI_SCREEN;
        let access = ponko_ui_access_new();
        assert!(!access.is_null());

        // The first look reports every node, in tree order.
        let (changes, changed, removed) = update(access, ctx);
        assert!(changes.any && changes.screens_changed);
        assert_eq!(changed, [m.root, m.title, m.play, m.volume, m.name, m.music, m.quit]);
        assert!(removed.is_empty());
        let mut count = 0;
        assert_eq!(ponko_ui_access_count(access, &mut count), PONKO_OK);
        assert_eq!(count, 7);

        // Nodes by index, and by id.
        let mut first = PonkoUiAccessNode::default();
        assert_eq!(ponko_ui_access_node(access, 0, &mut first), PONKO_OK);
        assert_eq!(
            (first.id, first.parent, first.depth, first.child_count),
            (m.root, PONKO_UI_NONE, 0, 6)
        );
        assert_eq!(first.role, PonkoUiRole::PONKO_UI_ROLE_GROUP);
        assert_eq!(id_of(|out| ponko_ui_access_child(access, m.root, 1, out)), m.play);

        let play = node(access, m.play);
        assert_eq!(play.role, PonkoUiRole::PONKO_UI_ROLE_BUTTON);
        assert_eq!((play.parent, play.depth), (m.root, 1));
        assert!(play.focusable && !play.focused && !play.disabled && !play.has_selection);
        assert!(play.rect.max.x > play.rect.min.x && play.rect.max.y > play.rect.min.y);
        assert_eq!(string(|b, c, l| ponko_ui_access_name(access, m.play, b, c, l)), "Play");

        let volume = node(access, m.volume);
        assert_eq!(volume.role, PonkoUiRole::PONKO_UI_ROLE_SLIDER);
        assert_eq!(
            (volume.value_kind, volume.value),
            (PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_RANGE, 0.5)
        );

        let name = node(access, m.name);
        assert_eq!(name.role, PonkoUiRole::PONKO_UI_ROLE_TEXT_INPUT);
        assert_eq!(name.value_kind, PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_TEXT);
        assert!(name.has_selection);
        assert_eq!((name.selection_anchor, name.selection_caret), (3, 3));
        assert_eq!(
            string(|b, c, l| ponko_ui_access_value_text(access, m.name, b, c, l)),
            "Ann"
        );
        assert_eq!(
            string(|b, c, l| ponko_ui_access_value_text(access, m.play, b, c, l)),
            "",
            "a button has no text value"
        );

        let music = node(access, m.music);
        assert_eq!(
            (music.role, music.value_kind, music.value),
            (
                PonkoUiRole::PONKO_UI_ROLE_CHECKBOX,
                PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_CHECKED,
                1.0
            )
        );
        let quit = node(access, m.quit);
        assert!(quit.disabled && !quit.focusable);
        assert_eq!(node(access, m.title).role, PonkoUiRole::PONKO_UI_ROLE_LABEL);
        assert_eq!(id_of(|out| ponko_ui_access_focus(access, out)), PONKO_UI_NONE);

        // Nothing changed: nothing is reported.
        let (changes, changed, removed) = update(access, ctx);
        assert!(!changes.any && changed.is_empty() && removed.is_empty());

        // One value, one description: only those nodes.
        assert_eq!(ponko_ctx_ui_set_value(ctx, s, m.volume, 0.75), PONKO_OK);
        assert_eq!(
            ponko_ctx_ui_set_accessible_description(ctx, s, m.play, c"Starts a run".as_ptr()),
            PONKO_OK
        );
        let (changes, changed, _) = update(access, ctx);
        assert!(changes.any && !changes.screens_changed && !changes.focus_changed);
        assert_eq!(changed, [m.play, m.volume]);
        assert_eq!(node(access, m.volume).value, 0.75);
        assert_eq!(
            string(|b, c, l| ponko_ui_access_description(access, m.play, b, c, l)),
            "Starts a run"
        );

        // Focus (Tab), then a modal dialog over the menu.
        let mut tab = ponko_ui_input_default();
        tab.next = true;
        frame(ctx, tab);
        let (changes, changed, _) = update(access, ctx);
        assert!(changes.focus_changed);
        assert_eq!(changed, [m.play]);
        assert_eq!(id_of(|out| ponko_ui_access_focus(access, out)), m.play);
        assert!(node(access, m.play).focused);

        let centered = ponko_ui_layout_centered(300.0, 120.0);
        let dialog = id_of(|out| ponko_ctx_ui_add_panel(ctx, s, PONKO_UI_NONE, &centered, out));
        let row = ponko_ui_layout_row(30.0);
        let yes = id_of(|out| ponko_ctx_ui_add_button(ctx, s, dialog, c"Yes".as_ptr(), &row, out));
        assert_eq!(ponko_ctx_ui_push_screen(ctx, s, dialog, true), PONKO_OK);
        frame(ctx, ponko_ui_input_default());
        let (changes, changed, removed) = update(access, ctx);
        assert!(changes.screens_changed);
        assert_eq!(changed, [dialog, yes]);
        assert_eq!(removed.len(), 7, "the modal dialog covers the whole menu");
        let shown = node(access, dialog);
        assert!(shown.modal);
        assert_eq!(shown.role, PonkoUiRole::PONKO_UI_ROLE_DIALOG);
        let mut hidden = PonkoUiAccessNode::default();
        assert_eq!(ponko_ui_access_find(access, m.play, &mut hidden), PONKO_NOT_FOUND);
        assert_eq!(
            ponko_ui_access_name(access, m.play, null_mut(), 0, &mut 0),
            PONKO_NOT_FOUND
        );

        // A reset reports everything again.
        assert_eq!(ponko_ui_access_reset(access), PONKO_OK);
        let (changes, changed, removed) = update(access, ctx);
        assert!(changes.any);
        assert_eq!((changed, removed.len()), (vec![dialog, yes], 0));

        ponko_ui_access_free(access);
    }
}
#[test]
fn the_snapshot_exposes_nodes_and_reports_only_what_changed() {
    run(snapshot_nodes_and_changes);
}

// ---- requests ------------------------------------------------------------------

unsafe fn requests(ctx: Ctx) {
    use PonkoUiAccessActionKind::*;
    use PonkoUiEventKind::*;
    unsafe {
        let m = menu(ctx);
        let s = PONKO_UI_SCREEN;
        let idle = || frame(ctx, ponko_ui_input_default());

        action(ctx, PONKO_UI_ACCESS_ACTION_ACTIVATE, m.play, 0.0, null());
        idle();
        assert_eq!(events(ctx), [(PONKO_UI_EVENT_CLICKED, m.play)]);
        assert_eq!(id_of(|out| ponko_ctx_ui_focus(ctx, s, out)), m.play);

        action(ctx, PONKO_UI_ACCESS_ACTION_FOCUS, m.name, 0.0, null());
        idle();
        assert!(events(ctx).is_empty(), "focusing queues nothing");
        assert_eq!(id_of(|out| ponko_ctx_ui_focus(ctx, s, out)), m.name);

        action(ctx, PONKO_UI_ACCESS_ACTION_INCREMENT, m.volume, 0.0, null());
        action(ctx, PONKO_UI_ACCESS_ACTION_INCREMENT, m.volume, 0.0, null());
        action(ctx, PONKO_UI_ACCESS_ACTION_DECREMENT, m.volume, 0.0, null());
        idle();
        let mut value = 0.0;
        assert_eq!(ponko_ctx_ui_value(ctx, s, m.volume, &mut value), PONKO_OK);
        assert!((value - 0.55).abs() < 1e-6, "{value}");
        assert_eq!(events(ctx).len(), 3);

        action(ctx, PONKO_UI_ACCESS_ACTION_SET_VALUE, m.volume, 0.2, null());
        action(ctx, PONKO_UI_ACCESS_ACTION_SET_VALUE, m.music, 0.0, null());
        action(ctx, PONKO_UI_ACCESS_ACTION_SET_TEXT, m.name, 0.0, c"Bea".as_ptr());
        idle();
        assert_eq!(
            events(ctx),
            [
                (PONKO_UI_EVENT_CHANGED, m.volume),
                (PONKO_UI_EVENT_CHANGED, m.music),
                (PONKO_UI_EVENT_CHANGED, m.name)
            ]
        );
        assert_eq!(string(|b, c, l| ponko_ctx_ui_text(ctx, s, m.name, b, c, l)), "Bea");

        // A disabled button and a label take no request; it is not an error.
        action(ctx, PONKO_UI_ACCESS_ACTION_ACTIVATE, m.quit, 0.0, null());
        action(ctx, PONKO_UI_ACCESS_ACTION_ACTIVATE, m.title, 0.0, null());
        idle();
        assert!(events(ctx).is_empty());
    }
}
#[test]
fn requests_do_what_keys_do_and_queue_the_same_events() {
    run(requests);
}

// ---- errors --------------------------------------------------------------------

unsafe fn bad_arguments(ctx: Ctx) {
    unsafe {
        let m = menu(ctx);
        let s = PONKO_UI_SCREEN;
        let stale = id_of(|out| ponko_ctx_ui_add_button(ctx, s, m.root, c"Gone".as_ptr(), null(), out));
        assert_eq!(ponko_ctx_ui_remove(ctx, s, stale), PONKO_OK);

        // Stale ids.
        assert_eq!(
            ponko_ctx_ui_set_accessible_name(ctx, s, stale, c"x".as_ptr()),
            PONKO_NOT_FOUND
        );
        assert_eq!(
            ponko_ctx_ui_set_accessible_description(ctx, s, stale, c"x".as_ptr()),
            PONKO_NOT_FOUND
        );
        assert_eq!(ponko_ctx_ui_set_live_region(ctx, s, stale, 1), PONKO_NOT_FOUND);
        assert_eq!(
            ponko_ctx_ui_access_action(ctx, s, 1, stale, 0.0, null()),
            PONKO_NOT_FOUND
        );
        assert!(last_error().contains("not found"), "{}", last_error());

        // Unknown enum values, null outputs.
        assert_eq!(ponko_ctx_ui_set_live_region(ctx, s, m.title, 9), PONKO_INVALID_ARGUMENT);
        assert_eq!(
            ponko_ctx_ui_access_action(ctx, s, 99, m.play, 0.0, null()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_ui_live_region(ctx, s, m.title, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_ui_accessible_name(ctx, s, m.play, null_mut(), 4, &mut 0),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_ui_access_text(ctx, s, null_mut(), 0, null_mut()),
            PONKO_INVALID_ARGUMENT
        );

        // A buffer that is too small gets the length and an empty string.
        let mut small = [1u8; 3];
        let mut len = 0;
        assert_eq!(
            ponko_ctx_ui_accessible_name(ctx, s, m.play, small.as_mut_ptr().cast(), small.len(), &mut len),
            PONKO_OK
        );
        assert_eq!((len, small[0]), (4, 0));

        // The snapshot handle.
        let access = ponko_ui_access_new();
        let mut changes = PonkoUiAccessChanges::default();
        assert_eq!(
            ponko_ui_access_update(null_mut(), ctx, s, &mut changes),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ui_access_update(access, ctx, s, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(ponko_ui_access_update(access, ctx, s, &mut changes), PONKO_OK);
        let mut n = PonkoUiAccessNode::default();
        assert_eq!(ponko_ui_access_node(access, 99, &mut n), PONKO_NOT_FOUND);
        assert_eq!(ponko_ui_access_node(access, 0, null_mut()), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ui_access_find(access, stale, &mut n), PONKO_NOT_FOUND);
        assert_eq!(ponko_ui_access_child(access, m.root, 99, &mut 0), PONKO_NOT_FOUND);
        assert_eq!(ponko_ui_access_child(access, m.play, 0, &mut 0), PONKO_NOT_FOUND);
        assert_eq!(ponko_ui_access_changed(access, 99, &mut 0), PONKO_NOT_FOUND);
        assert_eq!(ponko_ui_access_removed(access, 0, &mut 0), PONKO_NOT_FOUND);
        assert_eq!(ponko_ui_access_count(access, null_mut()), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ui_access_focus(null_mut(), &mut 0), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ui_access_reset(null_mut()), PONKO_INVALID_ARGUMENT);
        assert_eq!(
            ponko_ui_access_description(access, m.play, null_mut(), 2, &mut 0),
            PONKO_INVALID_ARGUMENT
        );
        // A panel's UI is reached by its entity; one that isn't there fails.
        assert_ne!(ponko_ui_access_update(access, ctx, 12345, &mut changes), PONKO_OK);
        ponko_ui_access_free(access);
        ponko_ui_access_free(null_mut());
    }
}
#[test]
fn stale_ids_unknown_values_and_null_outputs_fail_cleanly() {
    run(bad_arguments);
}
