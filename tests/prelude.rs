//! Importing the prelude must leave common game-owned names available.
#![deny(deprecated)]
use ponko::prelude::*;
struct Pose;
struct Material;
struct Job;
#[test]
fn common_game_names_do_not_collide_with_the_prelude() {
    let _ = (Pose, Material, Job);
    let mut world = World::new();
    let entity = world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
    assert_eq!(world.get::<d2::Transform>(entity).unwrap().position, vec2(1.0, 2.0));
    let _ = Game::new("prelude", 1, 1).run_headless(0, |_| {});
}
#[cfg(feature = "3d")]
#[test]
fn constants_are_discovered_from_their_owning_types() {
    assert_eq!(d3::Decal::MAX, 4096);
    assert_eq!(ponko::spatial::SpatialGrid::<()>::SORT_EVERY, 32);
    assert_eq!(ponko::render::GraphicsQuality::MIN_RENDER_SCALE, 0.25);
    assert_eq!(ponko::terrain::TerrainTile::LODS, 4);
}

/// A game's own module of common names, glob-imported next to the prelude:
/// any of these in the prelude would make its use below ambiguous (an error).
mod game {
    pub struct Pose;
    pub struct Material;
    pub struct Job;
    pub struct Motion;
    pub struct Blend;
    pub struct Effect;
    pub struct Shader;
    pub struct Joint;
    pub struct Post;
    pub struct Sky;
    pub struct Interest;
    pub struct Authority;
    pub struct Lane;
    pub struct Value;
    pub struct Field;
    pub struct Instant;
    pub struct Transform;
    pub struct Camera;
    pub struct Physics;
}

#[test]
fn a_game_glob_of_common_names_does_not_collide_with_the_prelude() {
    use game::*;
    let _ = (Pose, Material, Job, Motion, Blend, Effect, Shader, Joint, Post, Sky);
    let _ = (
        Interest, Authority, Lane, Value, Field, Instant, Transform, Camera, Physics,
    );
    let _: Vec2 = vec2(0.0, 0.0);
}
