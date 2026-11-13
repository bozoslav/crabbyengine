//! `#[derive(Component)]` in a crate that depends on `ponko` only (this
//! crate's tests have no `ponko_ecs` dependency): the generated impl must
//! name the trait through `ponko`.

use ponko::prelude::*;

#[derive(Component, Debug, PartialEq)]
struct Speed(f32);

#[derive(ponko::Component)]
struct Tagged<T>(T);

#[test]
fn a_crate_that_depends_only_on_ponko_derives_components() {
    let mut world = World::new();
    let e = world.spawn((Speed(2.0), Tagged(1u8)));
    assert_eq!(world.get::<Speed>(e), Some(&Speed(2.0)));
    assert_eq!(world.get::<Tagged<u8>>(e).map(|t| t.0), Some(1));
    assert_eq!(world.query::<&Speed>().iter().count(), 1);
}
