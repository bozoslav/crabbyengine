//! Runtime filters (`DynFilter`): what the C and C++ bindings use. Each one
//! must visit the same entities as the typed filter it mirrors, and fail
//! where the typed filter panics.

use ponko_ecs::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Component)]
struct Player;
#[derive(Component)]
struct Enemy;
#[derive(Component)]
struct Dead;
#[derive(Component)]
struct Ghost;
#[derive(Component)]
struct Tag;
#[derive(Component)]
struct Health(#[allow(dead_code)] i32);

/// One entity per combination of the four markers (bit 0 `Player`, 1 `Enemy`,
/// 2 `Dead`, 3 `Ghost`), twice: with and without a `Tag`.
fn world_of_combinations() -> World {
    let mut w = World::new();
    for tagged in [false, true] {
        for bits in 0..16 {
            let e = w.spawn((Health(bits),));
            if bits & 1 != 0 {
                w.insert(e, Player);
            }
            if bits & 2 != 0 {
                w.insert(e, Enemy);
            }
            if bits & 4 != 0 {
                w.insert(e, Dead);
            }
            if bits & 8 != 0 {
                w.insert(e, Ghost);
            }
            if tagged {
                w.insert(e, Tag);
            }
        }
    }
    w
}

fn typed<F: QueryFilter>(w: &mut World) -> Vec<Entity> {
    let mut out: Vec<Entity> = w.query::<Entity>().filter::<F>().iter().collect();
    out.sort();
    out
}

fn dynamic(w: &mut World, filter: &DynFilter) -> Result<Vec<Entity>, FilterError> {
    let compiled = filter.compile()?;
    let mut out = Vec::new();
    w.raw().query_chunks(ChunkQuery::new(&[]).filter(&compiled), |c| {
        out.extend_from_slice(c.entities)
    });
    out.sort();
    Ok(out)
}

struct Ids {
    player: ComponentId,
    enemy: ComponentId,
    dead: ComponentId,
    ghost: ComponentId,
    tag: ComponentId,
}

fn ids(w: &mut World) -> Ids {
    Ids {
        player: w.register::<Player>(),
        enemy: w.register::<Enemy>(),
        dead: w.register::<Dead>(),
        ghost: w.register::<Ghost>(),
        tag: w.register::<Tag>(),
    }
}

/// The runtime filter and the typed one return the same entities, and some.
fn same<F: QueryFilter>(w: &mut World, filter: DynFilter) -> usize {
    let want = typed::<F>(w);
    assert_eq!(dynamic(w, &filter).unwrap(), want, "{filter:?}");
    want.len()
}

use DynFilter::{All, Or as Any, With as Has, Without as Lacks};

#[test]
fn runtime_filters_match_the_typed_ones() {
    let mut w = world_of_combinations();
    let i = ids(&mut w);
    assert_eq!(same::<With<Player>>(&mut w, Has(i.player)), 16);
    assert_eq!(same::<Without<Dead>>(&mut w, Lacks(i.dead)), 16);
    assert_eq!(same::<()>(&mut w, All(vec![])), 32);
    assert_eq!(
        same::<(With<Player>, Without<Dead>)>(&mut w, All(vec![Has(i.player), Lacks(i.dead)])),
        8
    );
    assert_eq!(
        same::<Or<(With<Player>, With<Enemy>)>>(&mut w, Any(vec![Has(i.player), Has(i.enemy)])),
        24
    );
    assert_eq!(
        same::<(Or<(With<Player>, With<Enemy>)>, Without<Dead>)>(
            &mut w,
            All(vec![Any(vec![Has(i.player), Has(i.enemy)]), Lacks(i.dead)])
        ),
        12
    );
    // AND groups inside an OR, as a tuple arm.
    assert_eq!(
        same::<Or<((With<Player>, With<Enemy>), Without<Dead>)>>(
            &mut w,
            Any(vec![All(vec![Has(i.player), Has(i.enemy)]), Lacks(i.dead)])
        ),
        20
    );
    assert_eq!(
        same::<Or<((With<Player>, Without<Ghost>), (With<Enemy>, With<Tag>))>>(
            &mut w,
            Any(vec![
                All(vec![Has(i.player), Lacks(i.ghost)]),
                All(vec![Has(i.enemy), Has(i.tag)])
            ])
        ),
        14
    );
    // Nesting: an `Or` inside an arm, and a tuple inside that.
    assert_eq!(
        same::<Or<((With<Player>, Or<(With<Dead>, (With<Ghost>, With<Tag>))>), With<Enemy>)>>(
            &mut w,
            Any(vec![
                All(vec![
                    Has(i.player),
                    Any(vec![Has(i.dead), All(vec![Has(i.ghost), Has(i.tag)])])
                ]),
                Has(i.enemy)
            ])
        ),
        21
    );
    // Several filters at the top level combine like several `.filter()` calls.
    let want = {
        let mut out: Vec<Entity> = w
            .query::<Entity>()
            .filter::<Or<(With<Player>, With<Enemy>)>>()
            .filter::<Or<(With<Dead>, With<Ghost>)>>()
            .iter()
            .collect();
        out.sort();
        out
    };
    let both = All(vec![
        Any(vec![Has(i.player), Has(i.enemy)]),
        Any(vec![Has(i.dead), Has(i.ghost)]),
    ]);
    assert_eq!(dynamic(&mut w, &both).unwrap(), want);
}

#[test]
fn empty_and_contradictory_filters_follow_the_typed_rules() {
    let mut w = world_of_combinations();
    let i = ids(&mut w);
    assert_eq!(same::<Or<()>>(&mut w, Any(vec![])), 0);
    assert_eq!(
        same::<Or<((), With<Player>)>>(&mut w, Any(vec![All(vec![]), Has(i.player)])),
        32
    );
    assert_eq!(
        same::<(Or<()>, With<Player>)>(&mut w, All(vec![Any(vec![]), Has(i.player)])),
        0
    );
    assert_eq!(
        same::<(With<Player>, Without<Player>)>(&mut w, All(vec![Has(i.player), Lacks(i.player)])),
        0
    );
    assert_eq!(
        same::<(With<Player>, With<Player>)>(&mut w, All(vec![Has(i.player), Has(i.player)])),
        16
    );
}

#[test]
fn filters_reach_the_same_entities_with_columns_and_empty_archetypes() {
    let mut w = world_of_combinations();
    let i = ids(&mut w);
    let health = w.register::<Health>();
    // The filtered components needn't be columns, and columns narrow it too.
    let filter = Any(vec![Has(i.player), Has(i.enemy)]).compile().unwrap();
    let mut rows = 0;
    w.raw().query_chunks(ChunkQuery::new(&[health]).filter(&filter), |c| {
        assert_eq!(c.columns.len(), 1);
        rows += c.entities.len();
    });
    assert_eq!(rows, 24);
    // An archetype that emptied is skipped, as in the typed path.
    for e in typed::<With<Player>>(&mut w) {
        w.despawn(e);
    }
    assert_eq!(same::<With<Player>>(&mut w, Has(i.player)), 0);
}

#[test]
fn compile_fails_where_the_typed_filters_panic() {
    #[derive(Component)]
    struct C0;
    #[derive(Component)]
    struct C1;
    #[derive(Component)]
    struct C2;
    #[derive(Component)]
    struct C3;
    #[derive(Component)]
    struct C4;
    #[derive(Component)]
    struct C5;
    #[derive(Component)]
    struct C6;
    #[derive(Component)]
    struct C7;
    #[derive(Component)]
    struct C8;
    let mut w = World::new();
    let i = ids(&mut w);
    let c = [
        w.register::<C0>(),
        w.register::<C1>(),
        w.register::<C2>(),
        w.register::<C3>(),
        w.register::<C4>(),
        w.register::<C5>(),
        w.register::<C6>(),
        w.register::<C7>(),
        w.register::<C8>(),
    ];

    // 8 `With` components are fine, 9 are too many; the same for `Without`.
    assert!(All(c[..8].iter().map(|&id| Has(id)).collect()).compile().is_ok());
    assert_eq!(
        All(c.iter().map(|&id| Has(id)).collect()).compile().err(),
        Some(FilterError::TooManyWith)
    );
    assert!(All(c[..8].iter().map(|&id| Lacks(id)).collect()).compile().is_ok());
    assert_eq!(
        All(c.iter().map(|&id| Lacks(id)).collect()).compile().err(),
        Some(FilterError::TooManyWithout)
    );
    // The typed path has the same boundary: 8 distinct (and a repeat) fit, a
    // 9th distinct component panics, for `With` and for `Without`.
    macro_rules! eight {
        ($w:expr, $f:ident) => {
            $w.query::<Entity>()
                .filter::<($f<C0>, $f<C1>, $f<C2>, $f<C3>, $f<C4>, $f<C5>, $f<C6>, $f<C7>)>()
        };
    }
    for (name, with) in [("With", true), ("Without", false)] {
        let fits = catch_unwind(AssertUnwindSafe(|| {
            if with {
                let _ = eight!(w, With).filter::<With<C0>>();
            } else {
                let _ = eight!(w, Without).filter::<Without<C0>>();
            }
        }));
        assert!(fits.is_ok(), "8 `{name}` plus a repeat is within the limit");
        let panics = catch_unwind(AssertUnwindSafe(|| {
            if with {
                let _ = eight!(w, With).filter::<With<C8>>();
            } else {
                let _ = eight!(w, Without).filter::<Without<C8>>();
            }
        }));
        assert!(
            panics.is_err(),
            "the 9th distinct `{name}` component panics, where compile fails"
        );
    }
    // A tuple of 8 plus a ninth in another filter call, as one list of nine.
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = eight!(w, With).filter::<With<C8>>();
        }))
        .is_err()
    );

    // Five `Or`s are five clauses.
    let one = |id| Any(vec![Has(id)]);
    let four = All(vec![one(i.player), one(i.enemy), one(i.dead), one(i.ghost)]);
    assert!(four.compile().is_ok());
    let five = All(vec![one(i.player), one(i.enemy), one(i.dead), one(i.ghost), one(i.tag)]);
    assert_eq!(five.compile().err(), Some(FilterError::TooManyClauses));
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = w
                .query::<Entity>()
                .filter::<Or<(With<Player>,)>>()
                .filter::<Or<(With<Enemy>,)>>()
                .filter::<Or<(With<Dead>,)>>()
                .filter::<Or<(With<Ghost>,)>>()
                .filter::<Or<(With<Tag>,)>>();
        }))
        .is_err()
    );

    // An `Or` of five components twice, and three of four: 16 terms at most.
    let four_terms = Any(vec![Has(i.player), Has(i.enemy), Has(i.dead), Has(i.ghost)]);
    let five_terms = Any(vec![Has(i.player), Has(i.enemy), Has(i.dead), Has(i.ghost), Has(i.tag)]);
    assert!(
        All(vec![
            four_terms.clone(),
            four_terms.clone(),
            four_terms.clone(),
            four_terms.clone()
        ])
        .compile()
        .is_ok()
    );
    assert_eq!(
        All(vec![
            four_terms.clone(),
            four_terms.clone(),
            four_terms.clone(),
            five_terms
        ])
        .compile()
        .err(),
        Some(FilterError::TooManyTerms)
    );

    // Expansion counts too: (a AND b) OR (c AND d) is four clauses of two.
    let expanded = Any(vec![
        All(vec![Has(i.player), Has(i.enemy)]),
        All(vec![Has(i.dead), Has(i.ghost)]),
    ]);
    assert!(expanded.compile().is_ok());
    let too_wide = Any(vec![
        All(vec![Has(i.player), Has(i.enemy), Has(i.tag)]),
        All(vec![Has(i.dead), Has(i.ghost)]),
    ]);
    assert_eq!(too_wide.compile().err(), Some(FilterError::TooManyClauses));
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = w
                .query::<Entity>()
                .filter::<Or<((With<Player>, With<Enemy>, With<Tag>), (With<Dead>, With<Ghost>))>>();
        }))
        .is_err()
    );
}

#[test]
fn compile_stops_at_a_depth_limit_instead_of_recursing_forever() {
    let mut w = World::new();
    let i = ids(&mut w);
    let mut f = Has(i.player);
    for _ in 0..MAX_FILTER_DEPTH {
        f = All(vec![f]);
    }
    assert!(f.compile().is_ok());
    assert_eq!(All(vec![f]).compile().err(), Some(FilterError::TooDeep));
}

#[test]
fn parallel_and_serial_visits_are_the_same_through_a_compiled_filter() {
    // The compiled filter is plain data: it can be reused, copied and sent.
    fn assert_send_sync<T: Send + Sync + Copy>() {}
    assert_send_sync::<CompiledFilter>();
    let mut w = world_of_combinations();
    let i = ids(&mut w);
    let filter = All(vec![Has(i.player), Lacks(i.dead)]).compile().unwrap();
    let copy = filter;
    let mut a = 0;
    let mut b = 0;
    w.raw()
        .query_chunks(ChunkQuery::new(&[]).filter(&filter), |c| a += c.entities.len());
    w.raw()
        .query_chunks(ChunkQuery::new(&[]).filter(&copy), |c| b += c.entities.len());
    assert_eq!((a, b), (8, 8));
}

#[test]
fn an_empty_group_is_a_leaf_for_the_depth_limit() {
    let mut f = All(vec![]);
    for _ in 0..MAX_FILTER_DEPTH {
        f = Any(vec![f]);
    }
    assert!(f.compile().is_ok());
    assert_eq!(Any(vec![f]).compile().err(), Some(FilterError::TooDeep));
}

#[test]
fn compile_tree_looks_components_up_and_reports_unknown_ones() {
    let mut w = World::new();
    let i = ids(&mut w);
    let filter = All(vec![Has(i.player), Lacks(i.dead)]);
    let compiled = compile_tree(&filter, |index| w.component_at(index as usize)).unwrap();
    let mut seen = 0;
    w.raw()
        .query_chunks(ChunkQuery::new(&[]).filter(&compiled), |c| seen += c.entities.len());
    assert_eq!(seen, 0, "an empty world");
    assert_eq!(
        compile_tree(&Has(i.player), |_| None).err(),
        Some(FilterError::UnknownComponent(i.player.index() as u32))
    );
}
