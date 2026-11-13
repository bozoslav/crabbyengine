//! Residency budgets: meshes loaded by name stay under a byte budget, the
//! least recently used ones are evicted first, and evicted ones load again
//! when asked for. Headless: the byte counts are the engine's estimates.
//! Loads never evict: the engine checks the budgets at the start of each
//! frame, so what a load in one tick pushes out is gone in the next.
#![cfg(feature = "3d")]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::d3::Transform as Transform3D;
use ponko::prelude::*;
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::render::{LodGroup, LodLevel, LodRenderer, Mesh, MeshData, Model};
#[allow(unused_imports)]
use ponko::streaming::{ResidencyKind, ResidencyStats};

/// `n` copies of the test house, each its own file (and so its own mesh),
/// as absolute paths in a fresh temp folder.
fn houses(test: &str, n: usize) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!("ponko-residency-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let house = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/models/house.glb");
    (0..n)
        .map(|i| {
            let path = dir.join(format!("house{i}.glb"));
            std::fs::copy(&house, &path).unwrap();
            path.to_str().unwrap().to_string()
        })
        .collect()
}

type Step = Box<dyn FnMut(&mut Ctx)>;

/// Runs one step per simulation tick, in order (setup runs none).
fn run(steps: Vec<Step>) {
    let steps = Rc::new(RefCell::new(steps));
    let ticks = steps.borrow().len() as u64;
    let mut next = 0;
    Game::new("residency", 1, 1)
        .sim_system(move |ctx| {
            (steps.borrow_mut()[next])(ctx);
            next += 1;
        })
        .run_headless(ticks, |_| {});
}

/// A tick that asks for nothing: what was asked for in the tick before a
/// gap can be evicted after it (assets asked for in this or the previous
/// frame or tick are kept).
fn gap() -> Step {
    Box::new(|_| {})
}

fn stats(ctx: &Ctx) -> ResidencyStats {
    ctx.gfx.residency(ResidencyKind::Mesh)
}

/// The estimated bytes of one test house.
fn house_bytes() -> u64 {
    let bytes = Rc::new(RefCell::new(0));
    let out = bytes.clone();
    Game::new("size", 1, 1).run_headless(0, move |ctx| {
        ctx.gfx.resident_mesh("models/house.glb");
        *out.borrow_mut() = stats(ctx).resident_bytes;
    });
    let b = *bytes.borrow();
    assert!(b > 0, "a house has vertices");
    b
}

#[test]
fn without_a_budget_nothing_is_evicted() {
    let paths = houses("unbudgeted", 4);
    let b = house_bytes();
    let mut steps: Vec<Step> = Vec::new();
    for path in paths.clone() {
        steps.push(Box::new(move |ctx| {
            let mesh = ctx.gfx.resident_mesh(&path);
            assert!(ctx.gfx.is_live(mesh));
        }));
    }
    steps.push(Box::new(move |ctx| {
        let s = stats(ctx);
        assert_eq!(s.budget, None);
        assert_eq!((s.resident, s.resident_bytes), (4, 4 * b));
        assert_eq!((s.loads, s.reloads, s.evictions), (4, 0, 0));
        assert_eq!(s.over_budget_bytes(), 0);
    }));
    run(steps);
}

#[test]
fn asking_again_returns_the_same_mesh_without_loading() {
    let path = houses("same", 1).remove(0);
    Game::new("t", 1, 1).run_headless(0, move |ctx| {
        let a = ctx.gfx.resident_mesh(&path);
        let b = ctx.gfx.resident_mesh(&path);
        assert_eq!(a, b);
        assert_eq!(stats(ctx).loads, 1);
        assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &path));
    });
}

#[test]
fn the_least_recently_used_mesh_is_evicted_first() {
    let p = Rc::new(houses("lru", 4));
    let b = house_bytes();
    let budget = 2 * b + b / 2; // room for two houses
    let resident = |ctx: &Ctx, p: &[String], i: usize| ctx.gfx.is_resident(ResidencyKind::Mesh, &p[i]);
    let (p1, p2, p3, p4, p5, p6) = (p.clone(), p.clone(), p.clone(), p.clone(), p.clone(), p.clone());
    run(vec![
        Box::new(move |ctx| {
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(budget));
            ctx.gfx.resident_mesh(&p1[0]);
        }),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&p2[1]);
        }),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&p3[0]); // 0 is now used more recently than 1
        }),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&p4[2]);
            // Loading never evicts: over the budget until the next frame.
            assert_eq!(stats(ctx).resident, 3);
        }),
        Box::new(move |ctx| {
            assert!(resident(ctx, &p6, 0));
            assert!(!resident(ctx, &p6, 1), "1 was the least recently used");
            assert!(resident(ctx, &p6, 2));
            let s = stats(ctx);
            assert_eq!((s.resident, s.resident_bytes), (2, 2 * b));
            assert_eq!((s.evictions, s.evicted_bytes), (1, b));
            ctx.gfx.resident_mesh(&p6[3]);
        }),
        Box::new(move |ctx| {
            assert_eq!(
                (0..4).map(|i| resident(ctx, &p5, i)).collect::<Vec<_>>(),
                [false, false, true, true],
                "then 0"
            );
            let s = stats(ctx);
            assert_eq!((s.evictions, s.evicted_bytes), (2, 2 * b));
            assert_eq!(s.budget, Some(budget));
            assert!(s.resident_bytes <= budget);
        }),
    ]);
}

#[test]
fn evicted_meshes_load_again_when_asked_for() {
    let p = Rc::new(houses("reload", 2));
    let b = house_bytes();
    let (p1, p2, p3, p4, p5) = (p.clone(), p.clone(), p.clone(), p.clone(), p.clone());
    run(vec![
        Box::new(move |ctx| {
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
            ctx.gfx.resident_mesh(&p1[0]);
        }),
        gap(),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&p2[1]); // evicts 0 at the next frame
        }),
        Box::new(move |ctx| assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &p4[0]))),
        Box::new(move |ctx| {
            let mesh = ctx.gfx.resident_mesh(&p3[0]); // loads 0 again, evicts 1 next frame
            assert!(ctx.gfx.is_live(mesh));
        }),
        Box::new(move |ctx| {
            assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &p5[0]));
            assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &p5[1]));
            let s = stats(ctx);
            assert_eq!((s.loads, s.reloads, s.evictions), (2, 1, 2));
            assert_eq!((s.resident, s.resident_bytes), (1, b));
            // Its materials were kept, so loading it again made no new ones.
            assert!(!ctx.gfx.loaded_materials("walls").is_empty());
        }),
    ]);
}

#[test]
fn pinned_meshes_are_never_evicted() {
    let p = houses("pinned", 4);
    let b = house_bytes();
    let pinned = p[0].clone();
    let rest = p[1..].to_vec();
    let last = p[3].clone();
    let mut steps: Vec<Step> = Vec::new();
    let pin = pinned.clone();
    steps.push(Box::new(move |ctx| {
        ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(2 * b));
        let mesh = ctx.gfx.pin_resident_mesh(&pin);
        assert!(ctx.gfx.is_live(mesh));
    }));
    for (i, path) in rest.clone().into_iter().enumerate() {
        if i > 0 {
            steps.push(gap());
        }
        let pin = pinned.clone();
        steps.push(Box::new(move |ctx| {
            // The frame's check kept the budget before this load.
            assert!(stats(ctx).resident_bytes <= 2 * b);
            ctx.gfx.resident_mesh(&path);
            assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &pin));
        }));
    }
    let pin = pinned.clone();
    steps.push(Box::new(move |ctx| {
        let s = stats(ctx);
        assert_eq!((s.pinned, s.pinned_bytes), (1, b));
        assert_eq!(s.evictions, 2);
        assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &rest[0]));
        assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &rest[1]));
        assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &rest[2]));
        // Unpinned, it is the least recently used: a smaller budget takes it
        // first, at the next frame.
        ctx.gfx.unpin_resident_mesh(&pin);
        ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
        assert_eq!(stats(ctx).pinned, 0);
        assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &pin));
    }));
    steps.push(Box::new(move |ctx| {
        assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &pinned));
        assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &last));
    }));
    run(steps);
}

#[test]
fn meshes_entities_show_are_kept() {
    let p = houses("in-use", 4);
    let b = house_bytes();
    let (model, lod, shadow, other) = (p[0].clone(), p[1].clone(), p[2].clone(), p[3].clone());
    let kept = [model.clone(), lod.clone(), shadow.clone()];
    let first = model.clone();
    run(vec![
        Box::new(move |ctx| {
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
            let m = ctx.gfx.resident_mesh(&model);
            ctx.world.spawn((Transform3D::default(), Model::new(m)));
        }),
        Box::new(move |ctx| {
            let m = ctx.gfx.resident_mesh(&lod);
            let group = LodGroup::new(1.0, vec![LodLevel::new(0.1, vec![LodRenderer::new(m)])]);
            ctx.world.spawn((Transform3D::default(), group));
        }),
        Box::new(move |ctx| {
            let m = ctx.gfx.resident_mesh(&shadow);
            ctx.world
                .spawn((Transform3D::default(), ponko::render::ShadowMesh(Some(m))));
        }),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&other);
        }),
        Box::new(move |ctx| {
            // Over budget, but all three are shown: only nothing else could go.
            for path in &kept {
                assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, path), "{path}");
            }
            let s = stats(ctx);
            assert_eq!(s.evictions, 0, "the fourth was asked for last tick, so it is newest");
            assert_eq!(s.over_budget_bytes(), 3 * b);
            // Despawned, they can go: the oldest first, down to the budget.
            let shown: Vec<Entity> = ctx
                .world
                .query::<(Entity, &Transform3D)>()
                .iter()
                .map(|(e, _)| e)
                .collect();
            for e in shown {
                ctx.world.despawn(e);
            }
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
        }),
        Box::new(move |ctx| {
            let s = stats(ctx);
            assert_eq!((s.evictions, s.resident), (3, 1));
            assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &first));
        }),
    ]);
}

#[test]
fn meshes_asked_for_this_tick_stay_and_the_overflow_is_reported() {
    let p = houses("same-tick", 3);
    let b = house_bytes();
    let (a, bb, c) = (p[0].clone(), p[1].clone(), p[2].clone());
    let (a2, b2) = (a.clone(), bb.clone());
    run(vec![
        Box::new(move |ctx| {
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b + b / 2));
            let first = ctx.gfx.resident_mesh(&a);
            let second = ctx.gfx.resident_mesh(&bb);
            // Both handles stay good for the rest of the tick.
            assert!(ctx.gfx.is_live(first) && ctx.gfx.is_live(second));
            let s = stats(ctx);
            assert_eq!(s.evictions, 0);
            assert_eq!(s.over_budget_bytes(), b / 2 + b % 2);
            let report = s.to_string();
            assert!(report.contains("over budget"), "{report}");
        }),
        // Still kept in the next tick, in case they are asked for again.
        Box::new(move |ctx| assert_eq!(stats(ctx).evictions, 0)),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&c);
        }),
        Box::new(move |ctx| {
            assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &a2));
            assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &b2));
            let s = stats(ctx);
            assert_eq!((s.evictions, s.resident, s.over_budget_bytes()), (2, 1, 0));
        }),
    ]);
}

#[test]
fn a_mesh_asked_for_every_tick_stays_while_shown_ones_keep_the_budget_over() {
    // A model's mesh fills the budget; another, kept in a resource and drawn
    // some other way, is asked for every tick. Checking the budget at the
    // start of each frame must not evict it (and reload it) between asks.
    let p = houses("every-tick", 2);
    let b = house_bytes();
    let (shown, asked) = (p[0].clone(), p[1].clone());
    let mut steps: Vec<Step> = vec![Box::new(move |ctx| {
        ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
        let m = ctx.gfx.resident_mesh(&shown);
        ctx.world.spawn((Transform3D::default(), Model::new(m)));
    })];
    for _ in 0..10 {
        let asked = asked.clone();
        steps.push(Box::new(move |ctx| {
            let mesh = ctx.gfx.resident_mesh(&asked);
            assert!(ctx.gfx.is_live(mesh));
            let s = stats(ctx);
            assert_eq!((s.loads, s.reloads, s.evictions), (2, 0, 0), "{s}");
            assert_eq!(s.over_budget_bytes(), b);
        }));
    }
    run(steps);
}

#[test]
fn freeing_an_evicted_meshs_handle_releases_its_id_once() {
    let p = houses("release", 2);
    let b = house_bytes();
    let held = Rc::new(RefCell::new(Mesh::NONE));
    let (h1, h2) = (held.clone(), held.clone());
    let (first, second) = (p[0].clone(), p[1].clone());
    run(vec![
        Box::new(move |ctx| {
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
            *h1.borrow_mut() = ctx.gfx.resident_mesh(&first);
        }),
        gap(),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&second); // evicts the first next frame; its id is kept
        }),
        Box::new(move |ctx| {
            let held = *h2.borrow();
            assert!(!ctx.gfx.is_live(held));
            // The game lets the handle go: the id is free for new meshes,
            // and freeing it again doesn't free it twice.
            ctx.gfx.free(held);
            ctx.gfx.free(held);
            let material = ctx.gfx.default_material();
            let a = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
            let c = ctx.gfx.create_mesh(&MeshData::cube(1.0), material);
            use ponko::__ffi::{self, SLOT_MASK}; // raw ids, to see the slot
            let slot = |m: Mesh| __ffi::id(m) & SLOT_MASK;
            assert_eq!(slot(a), slot(held), "the released slot is reused");
            assert_ne!(a, held, "under a new id");
            assert!(!ctx.gfx.is_live(held) && ctx.gfx.is_live(a));
            assert_ne!(slot(a), slot(c));
        }),
    ]);
}

#[test]
fn removing_the_budget_stops_evicting() {
    let p = houses("unset", 3);
    let b = house_bytes();
    let mut steps: Vec<Step> = vec![Box::new(move |ctx| {
        ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
        ctx.gfx.set_residency_budget(ResidencyKind::Mesh, None);
    })];
    for path in p {
        steps.push(Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&path);
        }));
    }
    steps.push(Box::new(|ctx| {
        let s = stats(ctx);
        assert_eq!((s.resident, s.evictions), (3, 0));
    }));
    run(steps);
}

#[test]
fn meshes_loaded_directly_are_never_evicted() {
    let p = houses("direct", 2);
    let b = house_bytes();
    let (a, bb) = (p[0].clone(), p[1].clone());
    let a2 = a.clone();
    run(vec![
        Box::new(move |ctx| {
            ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(b));
            let direct = ctx.gfx.load_mesh(&a);
            // The game holds this handle itself, so residency must not free it.
            assert_eq!(ctx.gfx.resident_mesh(&a), direct);
            assert_eq!(stats(ctx).pinned, 1);
        }),
        Box::new(move |ctx| {
            ctx.gfx.resident_mesh(&bb);
            assert!(ctx.gfx.is_resident(ResidencyKind::Mesh, &a2));
            let mesh = ctx.gfx.load_mesh(&a2);
            assert!(ctx.gfx.is_live(mesh));
            assert_eq!(stats(ctx).evictions, 0);
        }),
    ]);
}

#[test]
fn a_resident_mesh_the_game_freed_loads_again() {
    let path = houses("freed", 1).remove(0);
    let b = house_bytes();
    Game::new("t", 1, 1).run_headless(0, move |ctx| {
        let mesh = ctx.gfx.resident_mesh(&path);
        ctx.gfx.free(mesh);
        assert!(!ctx.gfx.is_resident(ResidencyKind::Mesh, &path));
        let again = ctx.gfx.resident_mesh(&path);
        assert!(ctx.gfx.is_live(again));
        let s = stats(ctx);
        assert_eq!((s.resident, s.resident_bytes), (1, b));
        assert_eq!((s.loads, s.reloads, s.evictions), (1, 1, 0));
    });
}

#[test]
fn eviction_is_the_same_every_run() {
    fn trace(paths: &[String], b: u64) -> Vec<Vec<bool>> {
        let out = Rc::new(RefCell::new(Vec::new()));
        let log = out.clone();
        let paths = paths.to_vec();
        // A fixed access pattern over five meshes with room for two. Each
        // tick logs what the frame's check left, then asks for one.
        let pattern = [0, 1, 2, 0, 3, 1, 4, 0, 2, 2, 3];
        let mut i = 0;
        Game::new("t", 1, 1)
            .sim_system(move |ctx| {
                if i == 0 {
                    ctx.gfx.set_residency_budget(ResidencyKind::Mesh, Some(2 * b));
                }
                log.borrow_mut().push(
                    paths
                        .iter()
                        .map(|p| ctx.gfx.is_resident(ResidencyKind::Mesh, p))
                        .collect(),
                );
                if let Some(&next) = pattern.get(i) {
                    ctx.gfx.resident_mesh(&paths[next]);
                }
                i += 1;
            })
            .run_headless(12, |_| {});
        out.take()
    }
    let paths = houses("deterministic", 5);
    let b = house_bytes();
    let first = trace(&paths, b);
    assert_eq!(first, trace(&paths, b));
    // Plain LRU with two slots: after the last access, 2 and 3 are resident.
    assert_eq!(first.last().unwrap(), &vec![false, false, true, true, false]);
}

#[test]
#[should_panic(expected = "not found in assets/")]
fn a_missing_file_panics_with_the_loading_error() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.gfx.resident_mesh("models/nope.glb");
    });
}
