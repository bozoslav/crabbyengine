//! `#[derive(Component)]` and hand-written `impl Component`.

use std::marker::PhantomData;

use ponko_ecs::*;

#[derive(Component, Debug, PartialEq)]
struct Pos(f32);

#[derive(Component, Debug, PartialEq)]
enum State {
    Idle,
    Running { speed: f32 },
}

#[derive(Component)]
struct Unit;

struct ByHand(u32);
impl Component for ByHand {}

/// A generic type is a component for the arguments that make it one.
#[derive(Component, Debug, PartialEq)]
struct Tagged<T>(T);

/// Existing bounds and where clauses are kept.
#[derive(Component, Debug, PartialEq)]
struct Scaled<T: Copy, const N: usize>(T)
where
    T: Default;

/// Parameters that only appear behind `PhantomData` need no bounds.
#[derive(Component)]
struct Kind<T>(PhantomData<fn() -> T>);

/// A lifetime parameter: a component only for `'static`.
#[derive(Component)]
struct Label<'a>(&'a str);

#[test]
fn derived_and_hand_written_components_behave_the_same() {
    let mut w = World::new();
    let e = w.spawn((Pos(1.0), State::Running { speed: 2.0 }, Unit, ByHand(7)));
    assert_eq!(w.get::<Pos>(e), Some(&Pos(1.0)));
    assert_eq!(w.get::<State>(e), Some(&State::Running { speed: 2.0 }));
    assert!(w.has::<Unit>(e));
    assert_eq!(w.get::<ByHand>(e).map(|b| b.0), Some(7));
    w.insert(e, State::Idle);
    assert_eq!(w.query::<(&Pos, &State)>().iter().count(), 1);
}

#[test]
fn generic_components_are_one_component_per_argument() {
    let mut w = World::new();
    let e = w.spawn((Tagged(1u8), Tagged("name"), Scaled::<i32, 3>(4)));
    w.insert(e, Kind::<String>(PhantomData));
    w.insert(e, Label("static"));
    assert_eq!(w.get::<Tagged<u8>>(e), Some(&Tagged(1)));
    assert_eq!(w.get::<Tagged<&str>>(e), Some(&Tagged("name")));
    assert_eq!(w.get::<Scaled<i32, 3>>(e), Some(&Scaled(4)));
    assert!(w.get::<Scaled<i32, 2>>(e).is_none());
    assert!(w.has::<Kind<String>>(e) && !w.has::<Kind<u8>>(e));
    assert_eq!(w.get::<Label>(e).map(|l| l.0), Some("static"));
}
