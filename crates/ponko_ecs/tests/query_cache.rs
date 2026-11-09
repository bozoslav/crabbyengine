use ponko_ecs::{Entity, World};
struct Value(usize);
struct Marker<const N: usize>;

fn fragmented(w: &mut World) {
    for bits in 0..128 {
        let e = w.spawn((Value(bits),));
        macro_rules! m {
            ($n:literal) => {
                if bits & (1 << $n) != 0 {
                    w.insert(e, Marker::<$n>);
                }
            };
        }
        m!(0);
        m!(1);
        m!(2);
        m!(3);
        m!(4);
        m!(5);
        m!(6);
    }
}

#[test]
fn cached_queries_follow_registration_growth_moves_and_empty_layouts() {
    let mut w = World::new();
    fragmented(&mut w);
    assert_eq!(
        w.query::<&Value>()
            .cached()
            .with::<Marker<0>>()
            .without::<Marker<1>>()
            .count(),
        32
    );
    // Register a previously unknown optional component after warming the cache.
    assert_eq!(w.query::<(&Value, Option<&Marker<9>>)>().cached().count(), 128);
    let e = w.spawn((Value(500), Marker::<0>, Marker::<9>));
    for i in 0..512 {
        w.spawn((Value(i), Marker::<0>, Marker::<9>));
    }
    assert_eq!(
        w.query::<&Value>()
            .cached()
            .with::<Marker<0>>()
            .without::<Marker<1>>()
            .count(),
        545
    );
    w.insert(e, Marker::<1>);
    w.remove::<Marker<1>>(e);
    w.query::<&mut Value>()
        .cached()
        .with::<Marker<9>>()
        .for_each(|v| v.0 += 1);
    assert_eq!(w.get::<Value>(e).unwrap().0, 501);
    let ids: Vec<_> = w
        .query::<(Entity, &Value)>()
        .cached()
        .with::<Marker<9>>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    for e in ids {
        w.despawn(e);
    }
    assert_eq!(w.query::<&Value>().cached().with::<Marker<9>>().count(), 0);
    w.spawn((Value(1), Marker::<0>, Marker::<9>));
    assert_eq!(w.query::<&Value>().cached().with::<Marker<9>>().count(), 1);
}

#[test]
fn signatures_optional_filters_and_worlds_remain_independent() {
    let mut w = World::new();
    fragmented(&mut w);
    assert_eq!(w.query::<&Value>().cached().with::<Marker<0>>().count(), 64);
    assert_eq!(w.query::<&Value>().cached().without::<Marker<0>>().count(), 64);
    assert_eq!(
        w.query::<&Value>()
            .cached()
            .with::<Marker<0>>()
            .without::<Marker<0>>()
            .count(),
        0
    );
    let expected: Vec<_> = w
        .query::<(&Value, Option<&Marker<0>>)>()
        .cached()
        .iter()
        .map(|(v, m)| (v.0, m.is_some()))
        .collect();
    let mut actual = Vec::new();
    w.query::<(&Value, Option<&Marker<0>>)>()
        .cached()
        .for_each(|(v, m)| actual.push((v.0, m.is_some())));
    assert_eq!(actual, expected);
    let mut other = World::new();
    other.register::<Marker<0>>();
    fragmented(&mut other);
    assert_eq!(other.query::<&Value>().cached().with::<Marker<0>>().count(), 64);
}

#[cfg(feature = "parallel")]
#[test]
fn cached_parallel_order_matches_sequential() {
    let mut w = World::new();
    fragmented(&mut w);
    let expected: Vec<_> = w
        .query::<&Value>()
        .cached()
        .with::<Marker<0>>()
        .iter()
        .map(|v| v.0)
        .collect();
    let mut actual = Vec::new();
    w.query::<&Value>()
        .cached()
        .with::<Marker<0>>()
        .par_collect_into(&mut actual, |v| v.0);
    assert_eq!(actual, expected);
}
