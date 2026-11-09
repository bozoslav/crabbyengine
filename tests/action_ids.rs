//! `ActionId`: names resolved once, queried by index.
use ponko::*;

actions! {
    /// Test actions: names declared once.
    struct Act { jump = "jump", throttle = "throttle", steer = "steer" }
}

fn bindings() -> ActionMap {
    let mut map = ActionMap::default();
    map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
    map.set(
        "throttle",
        Action::Axis(vec![AxisBinding::Keys {
            negative: Key::S,
            positive: Key::W,
        }]),
    );
    map.set(
        "steer",
        Action::Axis2(vec![Axis2Binding::Keys {
            left: Key::A,
            right: Key::D,
            up: Key::Q,
            down: Key::E,
        }]),
    );
    map
}

fn headless(f: impl FnOnce(&mut Ctx)) {
    Game::new("action ids", 1, 1).run_headless(0, |ctx| {
        ctx.set_actions(bindings());
        f(ctx)
    });
}

#[test]
fn id_queries_match_the_name_queries() {
    headless(|ctx| {
        let [jump, throttle, steer] = ctx.action_ids(&["jump", "throttle", "steer"]);
        ctx.feed_input(InputEvent::Key(Key::Space, true));
        ctx.feed_input(InputEvent::Key(Key::W, true));
        ctx.feed_input(InputEvent::Key(Key::D, true));
        assert!(ctx.action_down(jump) && ctx.action_down("jump"));
        assert!(ctx.action_pressed(jump) && !ctx.action_released(jump));
        assert_eq!(ctx.action_axis(throttle), 1.0);
        assert_eq!(ctx.action_axis2(steer), Vec2::X);
        ctx.feed_input(InputEvent::Key(Key::Space, false));
        assert!(ctx.action_released(jump) && ctx.action_released("jump"));
        assert!(!ctx.action_down(jump));
    });
}

#[test]
fn ids_are_copy_and_distinct_per_action() {
    headless(|ctx| {
        let a = ctx.action_id("jump");
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, ctx.action_id("throttle"));
        assert!(!a.is_none());
    });
}

#[test]
fn unknown_names_fail_at_resolution_with_a_helpful_message() {
    headless(|ctx| {
        let error = ctx.try_action_id("jmup").unwrap_err();
        assert_eq!(error.name(), "jmup");
        assert_eq!(
            error.to_string(),
            "unknown action \"jmup\" (known: jump, steer, throttle)"
        );
        // The logging form gives the same error to stderr and an id that reads false/zero.
        let none = ctx.action_id("jmup");
        assert!(none.is_none());
        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(!ctx.action_down(none) && ctx.action_axis(none) == 0.0 && ctx.action_axis2(none) == Vec2::ZERO);
        assert_eq!(ActionId::default(), ActionId::NONE);
    });
}

#[test]
fn resolving_with_no_actions_installed_says_so() {
    let error = ActionMap::default().resolve("jump").unwrap_err();
    assert_eq!(
        error.to_string(),
        "unknown action \"jump\" (no actions are installed; see Ctx::set_actions)"
    );
}

#[test]
fn ids_survive_rebinding_and_loading() {
    headless(|ctx| {
        let jump = ctx.action_id("jump");
        let throttle = ctx.action_id("throttle");
        // Rebind jump to Enter, add a new action, drop `steer`.
        let mut changed = ctx.actions().clone();
        changed.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Enter)]));
        changed.set("fire", Action::Button(vec![ButtonBinding::Key(Key::F)]));
        changed.remove("steer");
        ctx.set_actions(changed);
        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(!ctx.action_down(jump));
        ctx.feed_input(InputEvent::Key(Key::Enter, true));
        assert!(ctx.action_down(jump), "the id still names `jump`");
        ctx.feed_input(InputEvent::Key(Key::W, true));
        assert_eq!(ctx.action_axis(throttle), 1.0);
        // A map loaded from JSON keeps the slots too.
        let json = ctx.actions().to_json().unwrap();
        ctx.set_actions(ActionMap::from_json(&json).unwrap());
        assert!(ctx.action_down(jump));
        assert_eq!(ctx.action_axis(throttle), 1.0);
        // New names resolve, removed ones don't.
        let fire = ctx.action_id("fire");
        ctx.feed_input(InputEvent::Key(Key::F, true));
        assert!(ctx.action_down(fire));
        assert!(ctx.try_action_id("steer").is_err());
    });
}

#[test]
fn a_removed_actions_id_reads_zero_and_rebinding_the_name_reuses_its_slot() {
    headless(|ctx| {
        let steer = ctx.action_id("steer");
        ctx.feed_input(InputEvent::Key(Key::D, true));
        assert_eq!(ctx.action_axis2(steer), Vec2::X);
        let mut changed = ctx.actions().clone();
        changed.remove("steer");
        ctx.set_actions(changed);
        assert_eq!(ctx.action_axis2(steer), Vec2::ZERO);
        // Binding the name again reuses its slot.
        let mut again = ctx.actions().clone();
        again.set(
            "steer",
            Action::Axis2(vec![Axis2Binding::Keys {
                left: Key::A,
                right: Key::D,
                up: Key::Q,
                down: Key::E,
            }]),
        );
        ctx.set_actions(again);
        assert_eq!(ctx.action_id("steer"), steer);
        assert_eq!(ctx.action_axis2(steer), Vec2::X);
    });
}

#[test]
fn the_actions_macro_resolves_every_name_once() {
    headless(|ctx| {
        assert_eq!(Act::NAMES, ["jump", "throttle", "steer"]);
        let act = Act::resolve(ctx);
        let copy = act; // Copy
        assert_eq!(copy.jump, ctx.action_id("jump"));
        assert_eq!(act.throttle, ctx.action_id("throttle"));
        assert_eq!(act.steer, ctx.action_id("steer"));
        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(ctx.action_pressed(act.jump));
    });
}

#[test]
fn string_queries_still_work_for_owned_names() {
    headless(|ctx| {
        let name = String::from("jump");
        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(ctx.action_down(&name));
        assert!(!ctx.action_down("absent"));
    });
}
