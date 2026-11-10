// Ponko for C++17: a header-only wrapper over crabby.h.
//
// Mirrors the Rust API: crabby::game is Game, crabby::ctx is Ctx,
// crabby::world_view is World. Components are plain structs, found by their
// C++ type: world.spawn(Health{100}), world.get<Health>(e) and
// world.each([](crabby::transform2d& t, const Health& h) { ... }).
// Errors become exceptions; exceptions thrown by your systems and callbacks
// are caught before they reach the engine and rethrown from
// run()/run_headless()/each()/query().
#ifndef CRABBY_HPP
#define CRABBY_HPP
#include "crabby.h"
#include <algorithm>
#include <array>
#include <limits>
#include <atomic>
#include <climits>
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <cstdint>
#include <exception>
#include <fstream>
#include <functional>
#include <new>
#include <sstream>
#include <memory>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <string>
#include <initializer_list>
#include <limits>
#include <tuple>
#include <type_traits>
#include <typeindex>
#include <unordered_map>
#include <utility>
#include <vector>

// ---- math on the C types (global, so it works without `using namespace`) ----
inline CrabbyVec2 operator+(CrabbyVec2 a, CrabbyVec2 b) { return {a.x + b.x, a.y + b.y}; }
inline CrabbyVec2 operator-(CrabbyVec2 a, CrabbyVec2 b) { return {a.x - b.x, a.y - b.y}; }
inline CrabbyVec2 operator-(CrabbyVec2 a) { return {-a.x, -a.y}; }
inline CrabbyVec2 operator*(CrabbyVec2 a, float s) { return {a.x * s, a.y * s}; }
inline CrabbyVec2 operator*(float s, CrabbyVec2 a) { return {a.x * s, a.y * s}; }
inline CrabbyVec2 operator/(CrabbyVec2 a, float s) { return {a.x / s, a.y / s}; }
inline CrabbyVec2& operator+=(CrabbyVec2& a, CrabbyVec2 b) { a.x += b.x; a.y += b.y; return a; }
inline CrabbyVec2& operator-=(CrabbyVec2& a, CrabbyVec2 b) { a.x -= b.x; a.y -= b.y; return a; }
inline CrabbyVec2& operator*=(CrabbyVec2& a, float s) { a.x *= s; a.y *= s; return a; }
inline bool operator==(CrabbyVec2 a, CrabbyVec2 b) { return a.x == b.x && a.y == b.y; }
inline bool operator!=(CrabbyVec2 a, CrabbyVec2 b) { return !(a == b); }
inline CrabbyVec3 operator+(CrabbyVec3 a, CrabbyVec3 b) { return {a.x + b.x, a.y + b.y, a.z + b.z}; }
inline CrabbyVec3 operator-(CrabbyVec3 a, CrabbyVec3 b) { return {a.x - b.x, a.y - b.y, a.z - b.z}; }
inline CrabbyVec3 operator-(CrabbyVec3 a) { return {-a.x, -a.y, -a.z}; }
inline CrabbyVec3 operator*(CrabbyVec3 a, float s) { return {a.x * s, a.y * s, a.z * s}; }
inline CrabbyVec3 operator*(float s, CrabbyVec3 a) { return {a.x * s, a.y * s, a.z * s}; }
inline CrabbyVec3 operator/(CrabbyVec3 a, float s) { return {a.x / s, a.y / s, a.z / s}; }
inline CrabbyVec3& operator+=(CrabbyVec3& a, CrabbyVec3 b) { a.x += b.x; a.y += b.y; a.z += b.z; return a; }
inline CrabbyVec3& operator-=(CrabbyVec3& a, CrabbyVec3 b) { a.x -= b.x; a.y -= b.y; a.z -= b.z; return a; }
inline CrabbyVec3& operator*=(CrabbyVec3& a, float s) { a.x *= s; a.y *= s; a.z *= s; return a; }
inline bool operator==(CrabbyVec3 a, CrabbyVec3 b) { return a.x == b.x && a.y == b.y && a.z == b.z; }
inline bool operator!=(CrabbyVec3 a, CrabbyVec3 b) { return !(a == b); }
/// `a` after `b`: rotating by the result is rotating by `b`, then by `a`.
inline CrabbyQuat operator*(CrabbyQuat a, CrabbyQuat b) {
    return {a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y, a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w, a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z};
}
inline CrabbyQuat& operator*=(CrabbyQuat& a, CrabbyQuat b) { a = a * b; return a; }
/// `v` rotated by `q`.
inline CrabbyVec3 operator*(CrabbyQuat q, CrabbyVec3 v) {
    const CrabbyVec3 u{q.x, q.y, q.z};
    const CrabbyVec3 t{2 * (u.y * v.z - u.z * v.y), 2 * (u.z * v.x - u.x * v.z), 2 * (u.x * v.y - u.y * v.x)};
    return v + q.w * t + CrabbyVec3{u.y * t.z - u.z * t.y, u.z * t.x - u.x * t.z, u.x * t.y - u.y * t.x};
}

namespace crabby {
using entity = std::uint64_t;
using vec2 = CrabbyVec2;
using color = CrabbyColor;
using texture = CrabbyTexture;
using transform2d = CrabbyTransform2D;
using attached = CrabbyAttached;
using sprite = CrabbySprite;
using camera = CrabbyCamera;
using vec3 = CrabbyVec3;
using quat = CrabbyQuat;
using transform3d = CrabbyTransform3D;
/// One joint of a skeleton (Rust `Joint`): its name, the joint it hangs
/// from (an earlier index; -1 for a root) and where it sits relative to it
/// at rest.
struct joint {
    std::string name;
    int parent = -1;
    transform3d rest{{0, 0, 0, 1}, {0, 0, 0}, {1, 1, 1}, {0, 0}};
};
using model = CrabbyModel;
/// No mesh (Rust `Mesh::NONE`): a model showing it draws nothing, and no
/// mesh is ever made with its id.
constexpr CrabbyMesh mesh_none{CRABBY_MESH_NONE};
/// No resource of handle type `H` (Rust `Handle::NONE`): never live, and
/// freeing it does nothing. `none<mesh>()` is `mesh_none`.
template<class H> constexpr H none() { H h{}; h.id = CRABBY_HANDLE_NONE; return h; }
/// How many handles of each kind are live (Rust `LiveCounts`).
using live_counts = CrabbyLiveCounts;
/// A kind of asset with its own residency budget (Rust `ResidencyKind`).
enum class residency_kind : std::uint32_t { mesh = CRABBY_RESIDENCY_MESH, texture = CRABBY_RESIDENCY_TEXTURE };
/// Where a streamed asset is (Rust `AssetState`).
enum class asset_state : std::uint32_t {
    unknown = CRABBY_ASSET_UNKNOWN, loading = CRABBY_ASSET_LOADING, uploading = CRABBY_ASSET_UPLOADING,
    streaming = CRABBY_ASSET_STREAMING, resident = CRABBY_ASSET_RESIDENT, evicted = CRABBY_ASSET_EVICTED,
    failed = CRABBY_ASSET_FAILED,
};
/// A kind's residency budget and counters (Rust `ResidencyStats`).
using residency_stats = CrabbyResidencyStats;
/// The upload queue's budget and counters (Rust `UploadStats`).
using upload_stats = CrabbyUploadStats;
/// One mesh of a LOD level, placed and tinted (Rust `LodRenderer`).
using lod_renderer = CrabbyLodRenderer;
inline lod_renderer lod_renderer_of(CrabbyMesh m) { return crabby_lod_renderer(m); }
/// One level of a LOD group (Rust `LodLevel`): its renderers are drawn while
/// the group covers at least `screen_height` of the screen's height. With
/// `custom_shadow`, `shadow_casters` cast the shadow instead (none: no shadow).
struct lod_level {
    float screen_height = 0;
    float fade_width = 0;
    std::vector<lod_renderer> renderers;
    bool custom_shadow = false;
    std::vector<lod_renderer> shadow_casters;
};
/// Global LOD settings (Rust `ctx.lod`): `bias` scales every group's screen height.
using lod_settings = CrabbyLodSettings;
/// Which level a group shows: kind 0 culled, 1 `level`, 2 fading from `level` to `to` (-1: nothing).
using lod_pick = CrabbyLodPick;
/// The fraction of the screen's height something `size` tall covers at `distance`.
inline float lod_screen_height_at(float size, float distance, float fov_y) { return crabby_lod_screen_height_at(size, distance, fov_y); }
/// The distance at which something `size` tall covers `height` of the screen.
inline float lod_distance_at(float size, float height, float fov_y) { return crabby_lod_distance_at(size, height, fov_y); }
using point_light3d = CrabbyPointLight3D;
using reflection_probe3d = CrabbyReflectionProbe3D;
inline reflection_probe3d reflection_probe3d_default() { return crabby_reflection_probe3d_default(); }
using spot_light3d = CrabbySpotLight3D;
/// A box that paints an image onto the opaque surfaces inside it (Rust
/// `Decal3D`); start from decal3d_default(). Projected along its -Z.
using decal3d = CrabbyDecal3D;
inline decal3d decal3d_default() { return crabby_decal3d_default(); }
/// `image` / `normal_image` value that selects no image.
constexpr std::uint32_t decal_no_image = CRABBY_DECAL_NO_IMAGE;
/// Which decal layers a model takes (Rust `ReceivesDecals`); models without
/// it take all. {0} takes none.
using receives_decals = CrabbyReceivesDecals;
/// A square of heightfield terrain, drawn more coarsely further from the
/// camera (Rust `TerrainTile`); made with ctx::create_terrain_tile.
using terrain_tile = CrabbyTerrainTile;
/// Several same-size images in one texture (Rust `TextureArray`).
using texture_array = CrabbyTextureArray;
/// What a texture array's pixels hold (Rust `TextureKind`).
enum class texture_kind : uint32_t { color = 0, data = 1 };
/// Ground layers and a weight map for ctx::create_terrain_material (Rust
/// `TerrainMaterialDesc`); start from terrain_material_of.
using terrain_material_desc = CrabbyTerrainMaterialDesc;
/// Ground `layers` blended by a `weights` map lying over `size` world units
/// from `corner` (x, z); no normal maps, each layer repeating every 4 units.
inline terrain_material_desc terrain_material_of(texture_array layers, texture_array weights, vec2 corner, vec2 size) {
    return crabby_terrain_material_desc(layers, weights, corner, size);
}
using mesh = CrabbyMesh;
/// Frame timing over the last few seconds, in ms (`gpu_ms` -1 where the GPU can't time itself).
using frame_stats = CrabbyFrameStats;
/// Bytes the renderer uploaded over the last drawn frame, by kind, and the
/// buffers it made (`reallocations`: buffers and textures replaced because outgrown).
using gpu_uploads = CrabbyGpuUploads;
using material = CrabbyMaterial;
using material_desc = CrabbyMaterialDesc;
/// A material shader in WGSL (ctx::load_shader, ctx::create_shader).
using shader = CrabbyShader;
/// A compute pass shader in WGSL (ctx::load_compute, ctx::create_compute).
using compute = CrabbyCompute;
using instance_batch = CrabbyInstanceBatch;
using instance_batch_options = CrabbyInstanceBatchOptions;
using instance_level = CrabbyInstanceLevel;
using gpu_buffer = CrabbyGpuBuffer;
/// One compute pass run each frame (Rust `ComputePass`); see compute_pass_of.
using compute_pass = CrabbyComputePass;
/// A screen effect shader in WGSL (ctx::load_effect, ctx::create_effect).
using effect = CrabbyEffect;
/// One screen effect run over the lit 3D frame (Rust `ScreenEffect`); see screen_effect_of.
using screen_effect = CrabbyScreenEffect;
/// The latest compile or hot reload of one of the game's shaders (Rust
/// `ShaderStatus`; ctx::shader_status). `info.kind` is CRABBY_SHADER_MATERIAL,
/// _EFFECT or _COMPUTE; `info.outcome` CRABBY_SHADER_COMPILED, _RELOADED,
/// _FAILED or _RELOAD_FAILED; `info.line` and `info.column` place the first
/// error in the game's own WGSL (0: no place).
struct shader_status {
    /// The path it was loaded from, or the name given to create_*.
    std::string shader;
    CrabbyShaderStatus info;
    /// Its errors as a compiler prints them (`water.wgsl:4:12: error: ...`); empty when it worked.
    std::string errors;
    bool ok() const noexcept { return info.ok; }
};
/// How a material covers what's behind it (`material_desc::blend`).
enum class blend : uint32_t { opaque = 0, transparent = 1, cutout = 2 };
using camera3d = CrabbyCamera3D;
/// A camera drawing the 3D world into a render target each frame (Rust
/// `TargetCamera`); see target_camera_of and ctx::reflection_camera.
using target_camera = CrabbyTargetCamera;
/// The sun, the sky's light, shadows and global illumination (bounced
/// light, the gi_ fields); read and change it with ctx::lighting /
/// ctx::set_lighting.
using lighting = CrabbyLighting;
/// Post-processing of the 3D frame (Rust `Post`); read and change it with
/// ctx::sky / ctx::set_sky: the sky behind the 3D world and the fog.
using sky = CrabbySky;
/// Which way sunlight travels at `hour` o'clock, for a day and night cycle.
inline vec3 sun_direction_at(float hour) { return crabby_sun_direction_at(hour); }
/// ctx::post / ctx::set_post. tonemap: see crabby::tonemap.
using post = CrabbyPost;
/// Post AA mode (`post::anti_aliasing`).
/// `taa` is temporal: sub-pixel jitter and a reprojected history (WebGL2 draws FXAA).
enum class anti_aliasing : uint32_t { none = 0, fxaa = 1, smaa = 2, taa = 3 };
/// Quality settings that trade looks for speed, switched at run time (Rust
/// `GraphicsQuality`): the 3D resolution scale, shadows, AO, screen-space
/// reflections, global illumination and antialiasing.
/// Get a preset with graphics_quality_preset(); apply it with
/// ctx::set_graphics_quality(); read the current one with ctx::graphics_quality().
using graphics_quality = CrabbyGraphicsQuality;
/// The presets (Rust `QualityLevel`); `custom` is what graphics_quality_level()
/// says of settings that are none of them.
enum class quality_level : uint32_t { low = 0, medium = 1, high = 2, ultra = 3, custom = 4 };
/// A class of GPU (Rust `GpuTier`), from the weakest the engine runs on:
/// WebGL2 (no compute), baseline (compute: every screen-space effect and GPU
/// culling) and desktop (also GPU skinning, decals and global illumination).
/// See gpu_tier_report() and ctx::gpu_tier().
enum class gpu_tier : uint32_t { webgl2 = 0, baseline = 1, desktop = 2 };
/// A renderer feature that depends on the GPU (Rust `GpuFeature`).
enum class gpu_feature : uint32_t {
    timestamps = 0, compute = 1, storage_buffers = 2, compute_culling = 3, gpu_skinning = 4,
    occlusion_culling = 5, temporal_antialiasing = 6, ambient_occlusion = 7, contact_shadows = 8,
    screen_space_reflections = 9, global_illumination = 10, decals = 11, multisampling = 12
};
/// How HDR light maps to the screen (`post::tonemap`).
enum class tonemap : uint32_t { aces = 0, agx = 1, neutral = 2, none = 3 };
using draw_params = CrabbyDrawParams;
using grid_hit = CrabbyGridHit;
using grid_item = CrabbyGridItem;
using key = CrabbyKey;
using mouse = CrabbyMouseButton;
using gamepad_button = CrabbyGamepadButton;
using gamepad_axis = CrabbyGamepadAxis;
/// A gamepad, by slot (0 is player one): `for (crabby::gamepad pad : ctx.gamepads())`.
struct gamepad { std::uint32_t slot; };
/// The connected gamepads (ctx::gamepads), oldest slot first. No allocation.
struct gamepad_list {
    std::array<gamepad, CRABBY_MAX_GAMEPADS> pads{};
    std::uint32_t count = 0;
    const gamepad* begin() const noexcept { return pads.data(); }
    const gamepad* end() const noexcept { return pads.data() + count; }
    std::size_t size() const noexcept { return count; }
    bool empty() const noexcept { return count == 0; }
};
/// Which stick, for ctx::gamepad_stick.
enum class stick { left, right };
using rect = CrabbyRect;
using font = CrabbyFont;
/// Alignment within a world text block.
enum class text_align3d : uint32_t { left = 0, center = 1, right = 2 };
/// Where a system runs in the loop, in this order. `input` through `sim_last`
/// run once per fixed tick (also headless); `frame` through `draw` once per
/// rendered frame. sim_system uses after_physics, system frame, late_system late.
enum class phase : uint32_t {
    input = CRABBY_PHASE_INPUT,
    before_physics = CRABBY_PHASE_BEFORE_PHYSICS,
    physics = CRABBY_PHASE_PHYSICS,
    after_physics = CRABBY_PHASE_AFTER_PHYSICS,
    sim_last = CRABBY_PHASE_SIM_LAST,
    frame = CRABBY_PHASE_FRAME,
    animate = CRABBY_PHASE_ANIMATE,
    late = CRABBY_PHASE_LATE,
    draw = CRABBY_PHASE_DRAW,
};
/// A system's removal handle (Rust `SystemId`): from game::add_system, or a
/// constant in `systems` for a built-in. Ids are never reused.
struct system_id {
    std::uint32_t bits = 0;
    friend bool operator==(system_id a, system_id b) { return a.bits == b.bits; }
    friend bool operator!=(system_id a, system_id b) { return a.bits != b.bits; }
};
/// The built-in systems' ids (Rust `Systems`), for game::remove. Removing
/// one that isn't installed does nothing.
namespace systems {
inline constexpr system_id physics{CRABBY_SYSTEM_PHYSICS};
inline constexpr system_id physics3d{CRABBY_SYSTEM_PHYSICS3D};
inline constexpr system_id interpolate_physics3d{CRABBY_SYSTEM_INTERPOLATE_PHYSICS3D};
inline constexpr system_id rebuild_spatial_grid{CRABBY_SYSTEM_REBUILD_SPATIAL_GRID};
inline constexpr system_id animate_sprites{CRABBY_SYSTEM_ANIMATE_SPRITES};
inline constexpr system_id draw_tilemaps{CRABBY_SYSTEM_DRAW_TILEMAPS};
inline constexpr system_id draw_sprites{CRABBY_SYSTEM_DRAW_SPRITES};
inline constexpr system_id follow_attachments{CRABBY_SYSTEM_FOLLOW_ATTACHMENTS};
inline constexpr system_id step_navmesh3d{CRABBY_SYSTEM_STEP_NAVMESH3D};
/// Animators posing skeletons; stays at the end of phase::animate.
inline constexpr system_id update_animators{CRABBY_SYSTEM_UPDATE_ANIMATORS};
/// Particle emitters and trails; stays at the end of phase::animate.
inline constexpr system_id update_particles{CRABBY_SYSTEM_UPDATE_PARTICLES};
/// Positional audio; stays at the end of phase::draw, after the drawing systems.
inline constexpr system_id update_audio3d{CRABBY_SYSTEM_UPDATE_AUDIO3D};
} // namespace systems
/// Engine services an access declaration can name (see crabby::access).
enum class service : uint32_t {
    input = CRABBY_SERVICE_INPUT,
    time = CRABBY_SERVICE_TIME,
    random = CRABBY_SERVICE_RANDOM,
    gfx = CRABBY_SERVICE_GFX,
    audio = CRABBY_SERVICE_AUDIO,
    assets = CRABBY_SERVICE_ASSETS,
    ui = CRABBY_SERVICE_UI,
    perf = CRABBY_SERVICE_PERF,
    camera = CRABBY_SERVICE_CAMERA,
    spatial = CRABBY_SERVICE_SPATIAL,
    physics = CRABBY_SERVICE_PHYSICS,
    physics3d = CRABBY_SERVICE_PHYSICS3D,
    net = CRABBY_SERVICE_NET,
};
/// Owned world text, in meters. Settings follow Rust Text3D defaults.
struct text3d {
    std::string text;
    crabby::font font{};
    float size = 0.25f;
    CrabbyColor color{1, 1, 1, 1};
    CrabbyColor outline{0, 0, 0, 0};
    float outline_width = 0, softness = 0;
    text_align3d align = text_align3d::center;
    bool billboard = false;
    explicit text3d(std::string value = "") : text(std::move(value)) {}
};

using text_params = CrabbyTextParams;
using ui_style = CrabbyUiStyle;
using sound = CrabbySound;
using playing = CrabbyPlaying;
using sound_params = CrabbySoundParams;
using audio_bus = CrabbyAudioBus;
using audio_effects = CrabbyAudioEffects;
using audio_settings3d = CrabbyAudioSettings3D;
using audio_listener3d = CrabbyAudioListener3D;
using sprite_sheet = CrabbySpriteSheet;
using animation = CrabbyAnimation;
using rigid_body = CrabbyRigidBody;
using collider = CrabbyCollider;
using velocity = CrabbyVelocity;
using ray_hit = CrabbyRayHit;
using collision = CrabbyCollision;
using rigid_body3d = CrabbyRigidBody3D;
using collider3d = CrabbyCollider3D;
using velocity3d = CrabbyVelocity3D;
using character3d = CrabbyCharacter3D;
using joint3d = CrabbyJoint3D;
using contact_modifier3d = CrabbyContactModifier3D;
using joint_drive3d = CrabbyJointDrive3D;
using collision_mesh = CrabbyCollisionMesh;
using collision_groups = CrabbyCollisionGroups;
using query_filter3d = CrabbyQueryFilter3D;
using hit3d = CrabbyHit3D;
using vehicle3d = CrabbyVehicle3D;
using wheel3d = CrabbyWheel3D;
using friction_curve = CrabbyFrictionCurve;
using terrain_collider = CrabbyTerrainCollider;
using collider_focus = CrabbyColliderFocus;
enum class texture_filter { linear, nearest };

inline float length(vec2 v) { return std::sqrt(v.x * v.x + v.y * v.y); }
inline float length_squared(vec2 v) { return v.x * v.x + v.y * v.y; }
inline float dot(vec2 a, vec2 b) { return a.x * b.x + a.y * b.y; }
inline vec2 normalize_or(vec2 v, vec2 fallback) { float l = length(v); return l > 0 && std::isfinite(l) ? v / l : fallback; }
inline vec2 from_angle(float radians) { return {std::cos(radians), std::sin(radians)}; }
inline float to_angle(vec2 v) { return std::atan2(v.y, v.x); }
inline vec2 rem_euclid(vec2 v, vec2 m) {
    auto r = [](float a, float b) { float x = std::fmod(a, b); return x < 0 ? x + b : x; };
    return {r(v.x, m.x), r(v.y, m.y)};
}
/// The id other calls take, from an entity in a query chunk.
inline entity bits(CrabbyEntity e) { return entity(e.generation) << 32 | e.index; }
/// The entity struct (as in query chunks and crabby::attached) for an id.
inline CrabbyEntity entity_of(entity e) { return {static_cast<std::uint32_t>(e), static_cast<std::uint32_t>(e >> 32)}; }

/// Physics bodies (game::physics): moved by gravity and collisions, never
/// moving, or moved only by you.
inline rigid_body dynamic_body() { return crabby_rigid_body(CRABBY_BODY_DYNAMIC); }
inline rigid_body fixed_body() { return crabby_rigid_body(CRABBY_BODY_FIXED); }
inline rigid_body kinematic_body() { return crabby_rigid_body(CRABBY_BODY_KINEMATIC); }
/// Collider shapes, in pixels; change density, friction, restitution,
/// sensor and offset on the result.
inline collider circle_collider(float radius) { return crabby_collider_circle(radius); }
inline collider rect_collider(vec2 size) { return crabby_collider_rect(size); }
inline collider capsule_collider(float height, float radius) { return crabby_collider_capsule(height, radius); }
/// A convex polygon collider: the convex hull of 3 to 8 corners around the
/// entity's position (Rust: `Collider::polygon`). Spawn or set it like any
/// collider; `world.collider_points(e)` reads the corners back.
struct polygon : collider {
    std::array<vec2, 8> points{};
    std::uint32_t count = 0;
};
inline polygon polygon_collider(std::initializer_list<vec2> corners) {
    if (corners.size() < 3 || corners.size() > 8) throw std::invalid_argument("polygon_collider: 3 to 8 points");
    polygon p;
    static_cast<collider&>(p) = crabby_collider_circle(1.0f);  // the default settings
    std::copy(corners.begin(), corners.end(), p.points.begin());
    p.count = static_cast<std::uint32_t>(corners.size());
    return p;
}

inline color rgb(float r, float g, float b) { return {r, g, b, 1}; }
inline color rgba(float r, float g, float b, float a) { return {r, g, b, a}; }
namespace colors {
constexpr color white{1, 1, 1, 1}, black{0, 0, 0, 1}, gray{.5f, .5f, .5f, 1}, red{1, 0, 0, 1},
    green{0, 1, 0, 1}, blue{0, 0, 1, 1}, yellow{1, 1, 0, 1}, transparent{0, 0, 0, 0}, crabby_dark{.11f, .12f, .15f, 1};
}
/// Follows `to` at `offset` (in its space: turned by its rotation, stretched
/// by its scale), turned `rotation` more. Needs game::follow_attachments().
inline attached attach_to(entity to, vec2 offset = {0, 0}, float rotation = 0) { return {entity_of(to), offset, rotation}; }
/// A transform at `position`, no rotation, scale 1.
inline transform2d transform_at(vec2 position, float rotation = 0, vec2 scale = {1, 1}) { return {position, rotation, scale}; }
/// A sprite showing all of `t`, centered, white, z 0.
inline sprite sprite_of(texture t, color tint = colors::white, float z = 0) {
    sprite s{}; s.texture = t; s.color = tint; s.z = z; s.pivot = {.5f, .5f}; return s;
}
inline draw_params draw_defaults() { return crabby_draw_params_default(); }

// 3D (Rust `Transform3D`, `Model`, `MaterialDesc`, `Quat`).
/// A rotation of `angle` radians around `axis` (Rust `Quat::from_axis_angle`).
inline quat quat_from_axis_angle(vec3 axis, float angle) { return crabby_quat_from_axis_angle(axis, angle); }
/// Rotations around one axis (Rust `Quat::from_rotation_x` and friends).
inline quat quat_from_rotation_x(float angle) { return crabby_quat_from_axis_angle({1, 0, 0}, angle); }
inline quat quat_from_rotation_y(float angle) { return crabby_quat_from_axis_angle({0, 1, 0}, angle); }
inline quat quat_from_rotation_z(float angle) { return crabby_quat_from_axis_angle({0, 0, 1}, angle); }
/// A 3D transform at `position`, turned by `rotation`, scaled by `scale` on every axis.
inline transform3d transform3d_at(vec3 position, quat rotation = {0, 0, 0, 1}, float scale = 1) {
    transform3d t = crabby_transform3d_at(position); t.rotation = rotation; t.scale = {scale, scale, scale}; return t;
}
/// `t` turned so its -Z axis points at `target`, +Y up.
inline transform3d looking_at(const transform3d& t, vec3 target) { return crabby_transform3d_looking_at(t, target); }
/// The direction `t` faces: its -Z axis.
inline vec3 forward(const transform3d& t) { return crabby_transform3d_forward(t); }
/// 3D physics bodies (game::physics3d), like the 2D ones.
inline rigid_body3d dynamic_body3d() { return crabby_rigid_body3d(CRABBY_BODY_DYNAMIC); }
inline rigid_body3d fixed_body3d() { return crabby_rigid_body3d(CRABBY_BODY_FIXED); }
inline rigid_body3d kinematic_body3d() { return crabby_rigid_body3d(CRABBY_BODY_KINEMATIC); }
/// 3D collider shapes, in meters; change density, friction, restitution,
/// sensor, offset and groups on the result.
inline collider3d sphere_collider(float radius) { return crabby_collider3d_sphere(radius); }
inline collider3d box_collider(vec3 size) { return crabby_collider3d_box(size); }
inline collider3d capsule_collider3d(float height, float radius) { return crabby_collider3d_capsule(height, radius); }
/// A collider of a heightfield, convex hull or triangle mesh made by
/// ctx::heightfield_collider and friends.
inline collider3d mesh_collider(collision_mesh m) { return crabby_collider3d_mesh(m); }
/// Makes a terrain tile's entity solid: a collider of `m` (its heightfield)
/// at the tile's transform3d, turned and scaled with it (Rust
/// `TerrainCollider::new`). Change friction, restitution and groups on the result.
inline terrain_collider solid_terrain(collision_mesh m) { return crabby_terrain_collider(m); }
/// Keeps static colliders within `radius` of its entity, until they're
/// beyond `keep_radius` of every focus, once ctx::stream_colliders is on
/// (Rust `ColliderFocus::new`).
inline collider_focus focus_colliders(float radius, float keep_radius) { return crabby_collider_focus(radius, keep_radius); }
/// A walking capsule `height` tall and `radius` wide, its transform3d at
/// its feet: set `velocity`, read `grounded`.
inline character3d character(float height = 1.8f, float radius = 0.3f) { return crabby_character3d(height, radius); }
/// Index of each joint3d axis in `axes` and `drives`: local X is the hinge, slider and twist axis.
namespace joint_axis {
constexpr std::size_t linear_x = 0, linear_y = 1, linear_z = 2, angular_x = 3, angular_y = 4, angular_z = 5;
}
/// Welds bodies `a` and `b` together.
inline joint3d fixed_joint3d(entity a, entity b) { return crabby_joint3d_fixed(a, b); }
/// A ball joint: the frames stay together, turning freely.
inline joint3d spherical_joint3d(entity a, entity b) { return crabby_joint3d_spherical(a, b); }
/// A hinge about the frames' X axis.
inline joint3d revolute_joint3d(entity a, entity b) { return crabby_joint3d_revolute(a, b); }
/// A slider along the frames' X axis.
inline joint3d prismatic_joint3d(entity a, entity b) { return crabby_joint3d_prismatic(a, b); }
/// Every axis free: set them yourself.
inline joint3d generic_joint3d(entity a, entity b) { return crabby_joint3d_generic(a, b); }
/// Limits one axis of `j` to [min, max] (meters, or radians for turning axes).
inline joint3d with_limits(joint3d j, std::size_t axis, float min, float max) {
    j.axes[axis] = {CRABBY_JOINT_LIMITED, min, max}; return j;
}
/// Puts a motor on one axis of `j`.
inline joint3d with_drive(joint3d j, std::size_t axis, joint_drive3d d) { d.enabled = true; j.drives[axis] = d; return j; }
/// A motor turning (or sliding) at `velocity`, with this much `damping` strength.
inline joint_drive3d velocity_drive(float velocity, float damping) {
    return {true, 0, velocity, 0, damping, std::numeric_limits<float>::infinity()};
}
/// A spring toward `position`.
inline joint_drive3d spring_drive(float position, float stiffness, float damping) {
    return {true, position, 0, stiffness, damping, std::numeric_limits<float>::infinity()};
}
/// Gives a 3D body contact dominance: it pushes bodies with lower values without being pushed back.
inline rigid_body3d with_dominance(rigid_body3d b, std::int8_t dominance) { b.dominance = dominance; return b; }
/// Gives a 3D body this mass in kg, whatever its collider's density.
inline rigid_body3d with_mass(rigid_body3d b, float kg) { b.mass = kg; return b; }
/// Moves a 3D body's center of mass to `center`, in its own frame (cars want it low).
inline rigid_body3d with_center_of_mass(rigid_body3d b, vec3 center) {
    b.has_center_of_mass = true; b.center_of_mass = center; return b;
}
/// A raycast wheel of `radius` mounted at `position` (its center at full
/// compression, in the chassis's frame) with the usual suspension and tyres.
inline wheel3d wheel(vec3 position, float radius) { return crabby_wheel3d(position, radius); }
/// A vehicle on these wheels (at most 8), for an entity with a dynamic
/// rigid_body3d, a collider3d and a transform3d facing -Z.
inline vehicle3d vehicle(std::initializer_list<wheel3d> wheels) {
    vehicle3d v = crabby_vehicle3d();
    for (const wheel3d& w : wheels)
        if (crabby_vehicle3d_add_wheel(&v, w) != CRABBY_OK) throw std::invalid_argument("a vehicle has at most 8 wheels");
    return v;
}
/// Where to draw `w` for a chassis at `chassis`: steer, spin and suspension travel included.
inline transform3d wheel_pose(const wheel3d& w, const transform3d& chassis) { return crabby_wheel3d_pose(&w, chassis); }
/// One bone of a ragdoll (Rust `RagdollBone`): the joint it starts at and
/// the capsule standing in for it, which runs to the joint's first child.
/// `ragdoll_bone("spine", 0.1f).swing(0.5f)`.
class ragdoll_bone {
public:
    ragdoll_bone(std::string joint, float radius) : joint_(std::move(joint)), raw_(crabby_ragdoll_bone(nullptr, radius)) {}
    /// How far it may swing away from its parent, in radians (default 0.7).
    ragdoll_bone swing(float radians) const { auto b = *this; b.raw_.swing = radians; return b; }
    /// How far it may twist around itself, either way, in radians (default 0.3).
    ragdoll_bone twist(float radians) const { auto b = *this; b.raw_.twist = radians; return b; }
    /// kg per cubic meter (default 1000).
    ragdoll_bone density(float kg_per_m3) const { auto b = *this; b.raw_.density = kg_per_m3; return b; }
    /// The C struct, pointing at this bone's joint name.
    CrabbyRagdollBone raw() const { auto r = raw_; r.joint = joint_.c_str(); return r; }
private:
    std::string joint_;
    CrabbyRagdollBone raw_;
};
/// A contact modifier that does nothing (Rust `ContactModifier3D::default()`).
/// Set its fields to combine effects, e.g. `.hook = true`.
inline contact_modifier3d default_contact_modifier3d() { return crabby_contact_modifier3d_default(); }
/// Solid only from the side `up` (a local direction) points to: things pass
/// up through it and land on top (Rust `ContactModifier3D::one_way`). Set
/// `.one_way_angle` (radians, default 1.2) to change how far a contact may
/// lean from `up` and still count.
inline contact_modifier3d one_way_modifier3d(vec3 up) { return crabby_contact_modifier3d_one_way(up); }
/// A surface carrying what touches it at `velocity` (local to the body, m/s),
/// like a conveyor belt (Rust `ContactModifier3D::conveyor`).
inline contact_modifier3d conveyor_modifier3d(vec3 velocity) { return crabby_contact_modifier3d_conveyor(velocity); }
/// A modifier whose contacts go to the hook set with `ctx::set_contact_hook3d`
/// (Rust `ContactModifier3D::hook`).
inline contact_modifier3d hook_modifier3d() { return crabby_contact_modifier3d_hook(); }
/// How fast a wheel spins, in revolutions per minute.
inline float rpm(const wheel3d& w) { return w.angular_velocity * 60.0f / 6.2831853f; }
/// The grip of a tyre curve at `slip` (either sign), stiffness included.
inline float evaluate(const friction_curve& c, float slip) { return crabby_friction_curve_evaluate(c, slip); }
/// In groups `memberships`, touching colliders in groups `filter`.
inline collision_groups groups(std::uint32_t memberships, std::uint32_t filter) { return {memberships, filter}; }
/// Filter for 2D queries: solids only by default, with optional entity exclusion.
struct filter2d {
    CrabbyQueryFilter2D raw{};
    filter2d& exclude(entity e) { raw.exclude = true; raw.exclude_entity = e; return *this; }
    filter2d& with_sensors() { raw.sensors = true; return *this; }
};

/// What a 3D query sees: every solid collider (no sensors) by default.
struct filter3d {
    query_filter3d raw = crabby_query_filter3d();
    /// Looks through `e` (say, the one casting).
    filter3d& exclude(entity e) { raw.exclude = true; raw.exclude_entity = e; return *this; }
    /// Sees what a collider in these groups would touch.
    filter3d& groups(collision_groups g) { raw.groups = g; return *this; }
    /// Sees sensors too.
    filter3d& with_sensors() { raw.sensors = true; return *this; }
};

/// Draws `m` tinted by `tint` (white: its own colors).
inline model model_of(mesh m, color tint = colors::white) { model x = crabby_model(m); x.color = tint; return x; }
/// A plain material of one color: roughness 0.6, not metallic. Change
/// roughness, metallic, emissive or set a texture on the result.
inline material_desc material_of(color c) { return crabby_material_desc(c); }
/// Runs `e`, reading its values and textures from `m` (or from the default
/// material when `m` is left out).
inline screen_effect screen_effect_of(effect e) { return {e, false, {}}; }
inline screen_effect screen_effect_of(effect e, material m) { return {e, true, m}; }
/// Runs `fn update(i: u32)` on every buffer element, with optional material inputs.
inline compute_pass buffer_compute_pass(compute c, gpu_buffer b) { return {true, b, c, {}, false, {}}; }
inline compute_pass buffer_compute_pass(compute c, gpu_buffer b, material m) { return {true, b, c, {}, true, m}; }
/// Runs `c` on render target `target`, reading its values and textures
/// from `m` (or from the default material when `m` is left out).
inline compute_pass compute_pass_of(compute c, texture target) { return {false, {}, c, target, false, {}}; }
inline compute_pass compute_pass_of(compute c, texture target, material m) { return {false, {}, c, target, true, m}; }
/// `c` drawing into render target `target` (ctx::create_render_target),
/// with no clip plane.
inline target_camera target_camera_of(const camera3d& c, texture target) { return {c, target, false, {0, 0, 0, 0}}; }
/// Built-in font, 16 px, white.
inline text_params text_defaults() { return crabby_text_params_default(); }
/// Full volume, normal speed, once.
inline sound_params sound_defaults() { return crabby_sound_params_default(); }
/// No low-pass, no reverb.
inline audio_effects audio_effects_defaults() { return crabby_audio_effects_default(); }
/// Full volume within 1 unit, silent at 100, natural fade, on the `sfx` bus.
inline audio_settings3d audio_settings3d_defaults() { return crabby_audio_settings3d_default(); }
inline ui_style ui_style_defaults() { return crabby_ui_style_default(); }
/// A rectangle from its top-left corner and size.
inline rect rect_at(vec2 min, vec2 size) { return {min, min + size}; }
/// Splits `t` into `w` x `h` frames, numbered left to right, top to bottom.
inline sprite_sheet sheet_of(texture t, std::uint32_t w, std::uint32_t h) {
    sprite_sheet s{};
    if (crabby_sprite_sheet(t, w, h, &s) != CRABBY_OK) throw std::invalid_argument(crabby_last_error());
    return s;
}
/// Frame `i` of a sheet in texture pixels (wraps past the end).
inline rect frame_of(const sprite_sheet& s, std::uint32_t i) { return crabby_sprite_sheet_frame(s, i); }
/// A sprite showing frame `i` of a sheet.
inline sprite sprite_of(const sprite_sheet& s, std::uint32_t i, color tint = colors::white, float z = 0) {
    sprite out = sprite_of(s.texture, tint, z); out.has_region = true; out.region = frame_of(s, i); return out;
}
/// Frames `first` .. `first + count - 1` at `fps`, looping (`.looping = false` holds the last one).
inline animation animation_of(const sprite_sheet& s, std::uint32_t first, std::uint32_t count, float fps) {
    return animation{s, first, count > 0 ? count : 1, fps, true, 0};
}
/// The frame an animation shows now.
inline std::uint32_t current_frame(const animation& a) {
    const float t = a.time * a.fps;
    const std::uint32_t n = t > 0 ? std::uint32_t(t) : 0;
    return a.first + (a.looping ? n % a.frames : std::min(n, a.frames - 1));
}
/// Whether a non-looping animation has played to its end.
inline bool finished(const animation& a) { return !a.looping && a.time * a.fps >= float(a.frames); }
constexpr std::uint16_t tile_empty = CRABBY_TILE_EMPTY;
/// Which entities take their turn this tick (ctx::turns).
struct turns {
    CrabbyTurns raw{1, 0, 0};
    bool includes(CrabbyEntity e) const noexcept { return e.index % raw.every == raw.now; }
    bool includes(entity e) const noexcept { return std::uint32_t(e) % raw.every == raw.now; }
    /// Seconds a whole round covers (`every` ticks).
    float dt() const noexcept { return raw.dt; }
};

/// The engine's random number generator (xoshiro256++), bit-identical to Rust's `Rng`,
/// so a game gives the same numbers in both languages.
class rng {
    std::uint64_t s_[4];
    static std::uint64_t rotl(std::uint64_t x, int k) { return (x << k) | (x >> (64 - k)); }
public:
    static std::uint64_t mix(std::uint64_t z) {
        z += 0x9E3779B97F4A7C15ull;
        z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ull;
        z = (z ^ (z >> 27)) * 0x94D049BB133111EBull;
        return z ^ (z >> 31);
    }
    explicit rng(std::uint64_t seed) {
        s_[0] = mix(seed); s_[1] = mix(s_[0]); s_[2] = mix(s_[1]); s_[3] = mix(s_[2]);
    }
    std::uint64_t u64() {
        const std::uint64_t out = rotl(s_[0] + s_[3], 23) + s_[0];
        const std::uint64_t t = s_[1] << 17;
        s_[2] ^= s_[0]; s_[3] ^= s_[1]; s_[1] ^= s_[2]; s_[0] ^= s_[3]; s_[2] ^= t; s_[3] = rotl(s_[3], 45);
        return out;
    }
    /// 0 <= x < 1.
    float f32() { return float(u64() >> 40) * (1.0f / float(1u << 24)); }
    float range(float low, float high) { return low + (high - low) * f32(); }
    /// 0 <= x < n.
    std::uint32_t below(std::uint32_t n) { return std::uint32_t(((u64() >> 32) * std::uint64_t(n)) >> 32); }
    bool chance(float p) { return f32() < p; }
    float angle() { return f32() * 6.28318530717958647692f; }
    vec2 unit_vec2() { return from_angle(angle()); }
};
/// This tick and system's random streams (Rust `Random`): `of(e)` per entity,
/// `stream(n)` for anything else. Same numbers on every run with the same seed.
class random {
    std::uint64_t base_;
public:
    explicit random(std::uint64_t bits) : base_(bits) {}
    rng stream(std::uint64_t n) const { return rng(base_ ^ rng::mix(n)); }
    rng of(entity e) const { return stream(e); }
};

class ctx;
class contact_pair3d;
namespace detail {
inline void check(CrabbyStatus status) {
    if (status != CRABBY_OK) throw std::runtime_error(crabby_last_error());
}
/// The `CrabbyHandleKind` of each handle type, for `ctx::is_live` and `ctx::free`.
template<class H> struct handle_kind;
template<> struct handle_kind<CrabbyTexture> { static constexpr std::uint32_t value = CRABBY_HANDLE_TEXTURE; };
template<> struct handle_kind<CrabbySound> { static constexpr std::uint32_t value = CRABBY_HANDLE_SOUND; };
template<> struct handle_kind<CrabbyFont> { static constexpr std::uint32_t value = CRABBY_HANDLE_FONT; };
template<> struct handle_kind<CrabbyMesh> { static constexpr std::uint32_t value = CRABBY_HANDLE_MESH; };
template<> struct handle_kind<CrabbyMaterial> { static constexpr std::uint32_t value = CRABBY_HANDLE_MATERIAL; };
template<> struct handle_kind<CrabbyShader> { static constexpr std::uint32_t value = CRABBY_HANDLE_SHADER; };
template<> struct handle_kind<CrabbyCompute> { static constexpr std::uint32_t value = CRABBY_HANDLE_COMPUTE; };
template<> struct handle_kind<CrabbyEffect> { static constexpr std::uint32_t value = CRABBY_HANDLE_EFFECT; };
template<> struct handle_kind<CrabbyGpuBuffer> { static constexpr std::uint32_t value = CRABBY_HANDLE_GPU_BUFFER; };
template<> struct handle_kind<CrabbyInstanceBatch> { static constexpr std::uint32_t value = CRABBY_HANDLE_INSTANCE_BATCH; };
template<> struct handle_kind<CrabbyTextureArray> { static constexpr std::uint32_t value = CRABBY_HANDLE_TEXTURE_ARRAY; };
template<class T> constexpr bool pod = std::is_trivially_copyable<T>::value &&
    std::is_trivially_destructible<T>::value && std::is_standard_layout<T>::value;
template<class T> using bare = typename std::remove_cv<typename std::remove_reference<T>::type>::type;

/// Reads text a C function writes into a buffer (call with a null buffer
/// to learn the size, then again to fill it; asks again if it grew).
template<class F> std::string read_text(F&& fill) {
    std::size_t capacity = 0;
    for (;;) {
        std::size_t len = 0;
        std::string s(capacity + 1, '\0');
        check(fill(capacity ? &s[0] : nullptr, capacity, &len));
        if (len < capacity || (capacity == 0 && len == 0)) { s.resize(len); return s; }
        capacity = len + 1;
    }
}

/// A small number per C++ type, so a world finds a type's component id by
/// indexing an array instead of a hash map.
inline std::size_t next_type_slot() { static std::atomic<std::size_t> next{0}; return next++; }
template<class T> std::size_t type_slot() { static const std::size_t slot = next_type_slot(); return slot; }

/// The name a component gets unless reflect() names it: the C++ type's name
/// with its namespaces, as the compiler spells it ("Health", "game::Unit").
template<class T> std::string make_type_name() {
#if defined(_MSC_VER) && !defined(__clang__)
    std::string s = __FUNCSIG__;
    const auto open = s.find("make_type_name<") + 15;
    s = s.substr(open, s.rfind(">(") - open);
    for (const char* prefix : {"struct ", "class ", "enum ", "union "})
        for (std::size_t at; (at = s.find(prefix)) != std::string::npos;) s.erase(at, std::strlen(prefix));
    return s;
#else
    const std::string s = __PRETTY_FUNCTION__;
    const auto open = s.find("T = ") + 4;
    return s.substr(open, s.find_first_of(";]", open) - open);
#endif
}
template<class T> const std::string& type_name() {
    static const std::string name = make_type_name<T>();
    return name;
}

/// The first exception thrown by a callback, kept until the C call returns.
struct first_error {
    std::mutex lock;
    std::exception_ptr error;
    std::atomic<bool> set{false};
    void capture() noexcept {
        std::lock_guard<std::mutex> guard(lock);
        if (!error) error = std::current_exception();
        set.store(true);
    }
    void rethrow() { if (error) std::rethrow_exception(error); }
};

/// Component ids of one world, by C++ type. Shared by a game, its ctx and
/// the world run_headless returns.
struct identity {
    std::vector<std::uint32_t> ids;  // type_slot -> id + 1 (0: not registered yet)
    std::unordered_map<std::uint32_t, std::type_index> types;  // id -> the C++ type using it
    // Commands and events whose type was first used inside a query, when the
    // world can't register it: replayed right after the system returns.
    std::mutex later_lock;
    std::vector<std::function<void(ctx&)>> later;
    // The contact hook (ctx::set_contact_hook3d) lives here, not in a ctx:
    // a ctx only lasts one system call, the hook as long as the game. An
    // exception it throws is kept in hook_error and rethrown by the game.
    std::function<void(contact_pair3d&)> contact_hook;
    first_error hook_error;
};

/// The parameter types of a function, lambda or function object, and
/// whether calling it can change it (a `mutable` lambda can).
template<class F> struct params : params<decltype(&F::operator())> {};
template<class R, class... A> struct params<R (*)(A...)> { using type = std::tuple<A...>; static constexpr bool changes = false; };
template<class R, class... A> struct params<R (*)(A...) noexcept> { using type = std::tuple<A...>; static constexpr bool changes = false; };
template<class C, class R, class... A> struct params<R (C::*)(A...)> { using type = std::tuple<A...>; static constexpr bool changes = true; };
template<class C, class R, class... A> struct params<R (C::*)(A...) const> { using type = std::tuple<A...>; static constexpr bool changes = false; };
template<class C, class R, class... A> struct params<R (C::*)(A...) noexcept> { using type = std::tuple<A...>; static constexpr bool changes = true; };
template<class C, class R, class... A> struct params<R (C::*)(A...) const noexcept> { using type = std::tuple<A...>; static constexpr bool changes = false; };

template<class M> struct field_type;
template<> struct field_type<float> { static constexpr CrabbyFieldType value = CRABBY_FIELD_F32; };
template<> struct field_type<double> { static constexpr CrabbyFieldType value = CRABBY_FIELD_F64; };
template<> struct field_type<std::int8_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_I8; };
template<> struct field_type<std::int16_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_I16; };
template<> struct field_type<std::int32_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_I32; };
template<> struct field_type<std::int64_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_I64; };
template<> struct field_type<std::uint8_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_U8; };
template<> struct field_type<std::uint16_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_U16; };
template<> struct field_type<std::uint32_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_U32; };
template<> struct field_type<std::uint64_t> { static constexpr CrabbyFieldType value = CRABBY_FIELD_U64; };
template<> struct field_type<bool> { static constexpr CrabbyFieldType value = CRABBY_FIELD_BOOL; };
template<> struct field_type<CrabbyVec2> { static constexpr CrabbyFieldType value = CRABBY_FIELD_VEC2; };
template<> struct field_type<CrabbyColor> { static constexpr CrabbyFieldType value = CRABBY_FIELD_COLOR; };
}
/// The settings of a quality preset (low, medium, high or ultra; throws for
/// quality_level::custom). Change single fields before applying them.
inline graphics_quality graphics_quality_preset(quality_level level) {
    graphics_quality q{}; detail::check(crabby_graphics_quality_preset(uint32_t(level), &q)); return q;
}
/// Which preset `q` is, or quality_level::custom. Throws if a field is out of range.
inline quality_level graphics_quality_level(const graphics_quality& q) {
    uint32_t level = 0; detail::check(crabby_graphics_quality_level(&q, &level)); return quality_level(level);
}
/// The quality preset meant for tier `t` (Rust `GraphicsQuality::for_tier`):
/// low with FXAA, medium or high, asking for nothing the tier lacks.
inline graphics_quality graphics_quality_for_tier(gpu_tier t) {
    graphics_quality q{}; detail::check(crabby_graphics_quality_for_tier(uint32_t(t), &q)); return q;
}
/// The JSON report a device with exactly tier `t`'s minimum limits gives
/// (Rust `GpuTier::report`): what the tier guarantees, and for the other
/// features the fallback and the reason. Needs no GPU.
inline std::string gpu_tier_report(gpu_tier t) {
    return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_gpu_tier_report(uint32_t(t), b, c, n); });
}
/// Whether every GPU of tier `t` has feature `f`. Needs no GPU.
inline bool gpu_tier_has(gpu_tier t, gpu_feature f) {
    bool on = false; detail::check(crabby_gpu_tier_feature(uint32_t(t), uint32_t(f), &on)); return on;
}
/// One described field of a component, from crabby::field.
template<class T> struct field_info { CrabbyField raw; };
/// Describes member `m` of component `T` for reflection: tools and agents
/// see it as JSON under `name`, and world saves include it. Fields of type
/// float, double, (u)int8..64_t, bool, vec2 and color; entities are uint64_t.
template<class T, class M> field_info<T> field(const char* name, M T::*m) {
    static_assert(std::is_default_constructible<T>::value, "reflected components must be default-constructible");
    const T sample{};
    const auto offset = std::size_t(reinterpret_cast<const char*>(&(sample.*m)) - reinterpret_cast<const char*>(&sample));
    return {{name, std::uint32_t(detail::field_type<typename std::remove_cv<M>::type>::value), offset}};
}
/// Where a game's time and allocations went: frame times, systems, your
/// profile scopes, allocations, the slowest frames and warnings with advice.
/// From ctx::perf_report() or game::profile_headless(). It is the report's
/// JSON; text() reads well, check() and regressions() are for tests and CI.
class perf_report {
    std::string json_;
public:
    perf_report() = default;
    explicit perf_report(std::string json) : json_(std::move(json)) {}
    /// The report as JSON, to save and compare later.
    const std::string& json() const noexcept { return json_; }
    /// The report for people to read.
    std::string text() const {
        return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_perf_report_text(json_.c_str(), b, c, n); });
    }
    /// What is over `budget_json` (e.g. `{"frame_p99_ms": 2, "allocs_per_frame": 0,
    /// "systems": {"movement": 0.5}, "scopes": {"ai/pathing": 1}}`), one line
    /// each. Empty: every limit holds.
    std::string check(const std::string& budget_json) const {
        return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_perf_check(json_.c_str(), budget_json.c_str(), b, c, n); });
    }
    /// True if every limit in `budget_json` holds.
    bool within(const std::string& budget_json) const { return check(budget_json).empty(); }
    /// What got slower than in `baseline` (an earlier report), by more than
    /// `tolerance` (0.2 is 20%), one line each. Empty: nothing did.
    std::string regressions(const perf_report& baseline, float tolerance = 0.2f) const {
        return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_perf_compare(json_.c_str(), baseline.json_.c_str(), tolerance, b, c, n); });
    }
    /// Writes the JSON to `path`.
    void save(const char* path) const {
        std::ofstream out(path, std::ios::binary);
        out << json_;
        if (!out) throw std::runtime_error(std::string("could not write ") + path);
    }
    /// Reads a report written by save().
    static perf_report load(const char* path) {
        std::ifstream in(path, std::ios::binary);
        if (!in) throw std::runtime_error(std::string("could not read ") + path);
        std::stringstream text;
        text << in.rdbuf();
        return perf_report(text.str());
    }
};

/// Times the scope it lives in (see CRABBY_PROFILE_SCOPE).
class profile_scope_guard {
    const CrabbyProfileScope* scope_;
    std::uint64_t token_;
public:
    explicit profile_scope_guard(const CrabbyProfileScope* scope) noexcept : scope_(scope), token_(crabby_profile_begin(scope)) {}
    ~profile_scope_guard() { crabby_profile_end(scope_, token_); }
    profile_scope_guard(const profile_scope_guard&) = delete;
    profile_scope_guard& operator=(const profile_scope_guard&) = delete;
};

class world_view;
class ctx;
/// A query filter: only entities whose `T` changed since the calling system
/// last ran. Use it as `world.each<crabby::changed<Health>>(fn)`, or in the
/// list of `query` and `count`. A filter also requires the entity to have
/// `T`, and starts tracking `T` (see world_view::track_changes). Several
/// filters must all pass; see any_of.
///
/// What counts as a change: spawn and set (also their `_later` forms), a
/// query that takes the component by `&` (every row it visits, written or
/// not; `const&` and copies only read), and mark_changed.
template<class T> struct changed {};
/// A query filter: only entities that got their `T` since the calling system
/// last ran: spawned with it, or `set` when they had none.
template<class T> struct added {};
/// A query filter that passes when at least one of `Fs` does:
/// `world.each<crabby::any_of<crabby::changed<Pos>, crabby::changed<Vel>>>(fn)`.
/// The entity must have every component the filters name. It must be the
/// query's only filter.
template<class... Fs> struct any_of {};

namespace detail {
template<class T> struct is_filter : std::false_type {};
template<class T> struct is_filter<changed<T>> : std::true_type {};
template<class T> struct is_filter<added<T>> : std::true_type {};
template<class... Fs> struct is_filter<any_of<Fs...>> : std::true_type {};
template<class T> struct is_any : std::false_type {};
template<class... Fs> struct is_any<any_of<Fs...>> : std::true_type {};
template<class... Ts> struct types {};
template<class A, class B> struct joined;
template<class... A, class... B> struct joined<types<A...>, types<B...>> { using type = types<A..., B...>; };
/// The types among `Ts` that are query filters (`Filters`), or the rest.
template<bool Filters, class... Ts> struct pick { using type = types<>; };
template<bool Filters, class T, class... Rest> struct pick<Filters, T, Rest...> {
    using type = typename joined<typename std::conditional<is_filter<T>::value == Filters, types<T>, types<>>::type,
                                 typename pick<Filters, Rest...>::type>::type;
};
/// The column type of an each() parameter: `T` for `T&` (written), `const T`
/// for `const T&` and copies (only read).
template<class P> using column = typename std::conditional<
    std::is_lvalue_reference<P>::value && !std::is_const<typename std::remove_reference<P>::type>::value,
    bare<P>, const bare<P>>::type;
/// The component ids and options of one query.
struct query_plan {
    std::array<std::uint32_t, 32> ids{};
    std::size_t count = 0;
    CrabbyQueryOptions options = crabby_query_options_default();
    /// The bit of column `id`, added as a read-only column if it's new.
    std::uint32_t bit(std::uint32_t id) {
        for (std::size_t i = 0; i < count; ++i) if (ids[i] == id) return std::uint32_t(1) << i;
        if (count == ids.size()) throw std::length_error("a query takes at most 32 components");
        ids[count] = id;
        return std::uint32_t(1) << count++;
    }
};
}  // namespace detail

/// One archetype's rows in a query: a column (array) per component, in the
/// order the query lists them. Valid only during the query callback. With a
/// changed/added filter, a chunk is a run of consecutive rows that pass.
template<class... Ts> class chunk {
    const CrabbyChunk* raw_;
    template<std::size_t... I> std::tuple<Ts*...> columns(std::index_sequence<I...>) const noexcept {
        return std::tuple<Ts*...>{static_cast<Ts*>(raw_->columns[I])...};
    }
public:
    explicit chunk(const CrabbyChunk* raw) : raw_(raw) {}
    std::size_t size() const noexcept { return raw_->count; }
    /// Where row 0 is in the whole query (rows are numbered 0..count() in the
    /// same order every time): row `i` of a query_par chunk can fill slot
    /// `first() + i` of a buffer sized with world::count().
    std::size_t first() const noexcept { return raw_->first; }
    /// The C chunk (columns, entities, count).
    const CrabbyChunk* raw_chunk() const noexcept { return raw_; }
    /// All columns at once: `auto [pos, vel] = chunk.columns();`
    std::tuple<Ts*...> columns() const noexcept { return columns(std::index_sequence_for<Ts...>{}); }
    /// The entity in row `index`.
    crabby::entity entity(std::size_t index) const {
        if (index >= raw_->count) throw std::out_of_range("chunk entity index");
        return bits(raw_->entities[index]);
    }
    /// The entity array (`index`, `generation`); `bits()` turns one into an id.
    const CrabbyEntity* entities() const noexcept { return raw_->entities; }
};
/// A tilemap on an entity (world_view::tilemap). Tile writes go straight
/// into the engine's map; change how it's drawn with `style()` + `apply()`.
class tilemap_ref {
    CrabbyWorld* world_;
    entity e_;
    CrabbyTileMap map_;
public:
    tilemap_ref(CrabbyWorld* world, entity e) : world_(world), e_(e), map_{} {
        detail::check(crabby_tilemap_get(world, e, &map_));
    }
    std::uint32_t width() const noexcept { return map_.width; }
    std::uint32_t height() const noexcept { return map_.height; }
    /// All tiles, row by row (`width() * height()`); tile_empty is a hole.
    std::uint16_t* tiles() const noexcept { return map_.tiles; }
    /// Sets one tile (ignored outside the map); tile_empty clears it.
    void set(std::uint32_t x, std::uint32_t y, std::uint16_t frame) noexcept {
        if (x < map_.width && y < map_.height) map_.tiles[std::size_t(y) * map_.width + x] = frame;
    }
    /// The frame at (x, y), or nothing for holes and outside the map.
    std::optional<std::uint16_t> get(std::uint32_t x, std::uint32_t y) const noexcept {
        if (x >= map_.width || y >= map_.height) return std::nullopt;
        const std::uint16_t t = map_.tiles[std::size_t(y) * map_.width + x];
        if (t == tile_empty) return std::nullopt;
        return t;
    }
    /// The tile under world point `p` for a map whose top-left is `origin`.
    std::optional<std::array<std::uint32_t, 2>> tile_at(vec2 origin, vec2 p) const noexcept {
        const float tx = std::floor((p.x - origin.x) / map_.tile_size.x), ty = std::floor((p.y - origin.y) / map_.tile_size.y);
        if (!(tx >= 0 && ty >= 0 && tx < float(map_.width) && ty < float(map_.height))) return std::nullopt;
        return std::array<std::uint32_t, 2>{{std::uint32_t(tx), std::uint32_t(ty)}};
    }
    /// Sheet, tile size, z and tint; change them, then call apply().
    CrabbyTileMap& style() noexcept { return map_; }
    void apply() { detail::check(crabby_tilemap_update(world_, e_, &map_)); }
};
/// The entities and their components (Rust `World`). Components are plain
/// structs, found by their C++ type: no registration needed. A view given to
/// a system must not escape the call.
class world_view {
    friend class world;
    friend class ctx;
protected:
    CrabbyWorld* raw_;
    std::shared_ptr<detail::identity> owner_;
    world_view(CrabbyWorld* raw, std::shared_ptr<detail::identity> owner) : raw_(raw), owner_(std::move(owner)) {}
    void valid() const { if (!raw_) throw std::logic_error("world has been moved from"); }

    template<class T> static void check_component() {
        static_assert(detail::pod<T>, "components must be plain structs: trivially copyable, standard layout");
        static_assert(!std::is_empty<T>::value || sizeof(T) == 1, "empty components must have size 1");
    }
    /// Registers `T` in `world` under `name` (or its type name) and remembers
    /// its id. Nothing if the world is busy (inside a query).
    template<class T> static std::optional<std::uint32_t> try_register(CrabbyWorld* world, detail::identity& owner, const char* name) {
        check_component<T>();
        std::uint32_t id = 0;
        CrabbyStatus status;
        if constexpr (std::is_same<T, transform2d>::value) status = crabby_transform_component(world, &id);
        else if constexpr (std::is_same<T, sprite>::value) status = crabby_sprite_component(world, &id);
        else if constexpr (std::is_same<T, attached>::value) status = crabby_attached_component(world, &id);
        else if constexpr (std::is_same<T, animation>::value) status = crabby_animation_component(world, &id);
        else if constexpr (std::is_same<T, rigid_body>::value) status = crabby_rigid_body_component(world, &id);
        else if constexpr (std::is_same<T, collider>::value) status = crabby_collider_component(world, &id);
        else if constexpr (std::is_same<T, velocity>::value) status = crabby_velocity_component(world, &id);
        else if constexpr (std::is_same<T, rigid_body3d>::value) status = crabby_rigid_body3d_component(world, &id);
        else if constexpr (std::is_same<T, collider3d>::value) status = crabby_collider3d_component(world, &id);
        else if constexpr (std::is_same<T, velocity3d>::value) status = crabby_velocity3d_component(world, &id);
        else if constexpr (std::is_same<T, character3d>::value) status = crabby_character3d_component(world, &id);
        else if constexpr (std::is_same<T, joint3d>::value) status = crabby_joint3d_component(world, &id);
        else if constexpr (std::is_same<T, contact_modifier3d>::value) status = crabby_contact_modifier3d_component(world, &id);
        else if constexpr (std::is_same<T, vehicle3d>::value) status = crabby_vehicle3d_component(world, &id);
        else if constexpr (std::is_same<T, terrain_collider>::value) status = crabby_terrain_collider_component(world, &id);
        else if constexpr (std::is_same<T, collider_focus>::value) status = crabby_collider_focus_component(world, &id);
        else if constexpr (std::is_same<T, transform3d>::value) status = crabby_transform3d_component(world, &id);
        else if constexpr (std::is_same<T, reflection_probe3d>::value) status = crabby_reflection_probe3d_component(world, &id);
        else if constexpr (std::is_same<T, decal3d>::value) status = crabby_decal3d_component(world, &id);
        else if constexpr (std::is_same<T, receives_decals>::value) status = crabby_receives_decals_component(world, &id);
        else if constexpr (std::is_same<T, point_light3d>::value) status = crabby_point_light3d_component(world, &id);
        else if constexpr (std::is_same<T, spot_light3d>::value) status = crabby_spot_light3d_component(world, &id);
        else if constexpr (std::is_same<T, model>::value) status = crabby_model_component(world, &id);
        else if constexpr (std::is_same<T, terrain_tile>::value) status = crabby_terrain_tile_component(world, &id);
        else status = crabby_component_register(world, name ? name : detail::type_name<T>().c_str(), sizeof(T), alignof(T), &id);
        if (status == CRABBY_BUSY) return std::nullopt;
        detail::check(status);
        const std::type_index type(typeid(T));
        const auto known = owner.types.emplace(id, type);
        if (!known.second && known.first->second != type)
            throw std::invalid_argument("component name " + std::string(name ? name : detail::type_name<T>()) +
                                        " is already used by a different C++ type");
        const std::size_t slot = detail::type_slot<T>();
        if (owner.ids.size() <= slot) owner.ids.resize(slot + 1, 0);
        if (owner.ids[slot] != 0 && owner.ids[slot] != id + 1)
            throw std::logic_error("reflect<" + detail::type_name<T>() + ">(name, ...) must come before the component is first used");
        owner.ids[slot] = id + 1;
        return id;
    }
    template<class T> static std::uint32_t register_type(CrabbyWorld* world, detail::identity& owner, const char* name) {
        if (auto id = try_register<T>(world, owner, name)) return *id;
        throw std::logic_error(detail::type_name<T>() + " is used for the first time while the world is borrowed by a query");
    }
    /// The component id of `T` in this world, registering it on first use;
    /// nothing if it's new and the world is busy.
    template<class T> static std::optional<std::uint32_t> find_id(CrabbyWorld* world, detail::identity& owner) {
        const std::size_t slot = detail::type_slot<T>();
        if (slot < owner.ids.size() && owner.ids[slot] != 0) return owner.ids[slot] - 1;
        return try_register<T>(world, owner, nullptr);
    }
    template<class T> static std::uint32_t id_of(CrabbyWorld* world, detail::identity& owner) {
        const std::size_t slot = detail::type_slot<T>();
        if (slot < owner.ids.size() && owner.ids[slot] != 0) return owner.ids[slot] - 1;
        return register_type<T>(world, owner, nullptr);
    }
    template<class T> std::uint32_t id() const { valid(); return id_of<T>(raw_, *owner_); }

    template<class T> void add_filter(detail::query_plan& plan, changed<T>*) const { plan.options.changed |= plan.bit(id<T>()); }
    template<class T> void add_filter(detail::query_plan& plan, added<T>*) const { plan.options.added |= plan.bit(id<T>()); }
    template<class... Fs> void add_filter(detail::query_plan& plan, any_of<Fs...>*) const {
        static_assert(!std::disjunction<detail::is_any<Fs>...>::value, "any_of can't hold another any_of: list its filters directly");
        plan.options.any = true;
        (add_filter(plan, static_cast<Fs*>(nullptr)), ...);
    }
    /// Runs `fn(chunk<Ts...>)` for every archetype with all of `Ts`, or for
    /// every run of rows that pass the filters `Fs`. A `const` column is only
    /// read; the others count as changed for every row visited.
    template<class... Ts, class... Fs, class F> void run_query(detail::types<Ts...>, detail::types<Fs...>, bool parallel, F& fn) {
        static_assert(sizeof...(Ts) + sizeof...(Fs) > 0, "a query needs at least one component");
        static_assert(sizeof...(Ts) <= 32, "a query takes at most 32 components");
        static_assert(!std::disjunction<std::is_same<detail::bare<Ts>, sprite>...>::value,
                      "sprite is not a query column: query transform2d (and your own components) instead");
        static_assert(!std::disjunction<std::is_same<detail::bare<Ts>, rigid_body>..., std::is_same<detail::bare<Ts>, collider>...>::value,
                      "rigid_body and collider are not query columns: use get/set (velocity is fine)");
        static_assert(!std::disjunction<std::is_same<detail::bare<Ts>, rigid_body3d>..., std::is_same<detail::bare<Ts>, collider3d>...,
                                        std::is_same<detail::bare<Ts>, character3d>..., std::is_same<detail::bare<Ts>, joint3d>...,
                                        std::is_same<detail::bare<Ts>, contact_modifier3d>...>::value,
                      "rigid_body3d, collider3d, character3d, joint3d and contact_modifier3d are not query columns: use get/set (velocity3d is fine)");
        static_assert(!std::disjunction<detail::is_any<Fs>...>::value || sizeof...(Fs) == 1,
                      "any_of must be the query's only filter");
        valid();
        detail::query_plan plan;
        plan.options.writes = 0;
        plan.options.parallel = parallel;
        ((plan.options.writes |= std::uint32_t(!std::is_const<Ts>::value) << plan.count,
          plan.ids[plan.count++] = id<detail::bare<Ts>>()), ...);
        (add_filter(plan, static_cast<Fs*>(nullptr)), ...);
        struct context { F* fn; detail::first_error error; } c{&fn, {}};
        auto trampoline = [](const CrabbyChunk* raw, void* user) noexcept {
            auto& c = *static_cast<context*>(user);
            if (c.error.set.load()) return;
            try { (*c.fn)(chunk<Ts...>(raw)); } catch (...) { c.error.capture(); }
        };
        auto status = crabby_query_with(raw_, plan.ids.data(), plan.count, &plan.options, trampoline, &c);
        c.error.rethrow();
        detail::check(status);
    }
    template<class... Ts, class... Fs> std::size_t count_rows(detail::types<Ts...>, detail::types<Fs...> filters) {
        std::size_t n = 0;
        auto add = [&n](chunk<const detail::bare<Ts>...> c) { n += c.size(); };
        run_query(detail::types<const detail::bare<Ts>...>{}, filters, false, add);
        return n;
    }
    /// each(): calls `fn` per row with the row's entity (if `fn` asks for one)
    /// and its components. Inlined into a plain loop over the columns.
    template<bool WithEntity, class... Ts> struct each_row {
        template<class F, std::size_t... I> static void chunk_rows(F& fn, const CrabbyChunk* raw, std::index_sequence<I...> rows) {
            // A local copy of a function that can't change itself does the
            // same, and lets the compiler keep its captures in registers:
            // writes to components can't touch a local the engine never saw.
            if constexpr (!detail::params<F>::changes && std::is_copy_constructible<F>::value) {
                const F local(fn);
                rows_of(local, raw, rows);
            } else {
                rows_of(fn, raw, rows);
            }
        }
        template<class F, std::size_t... I> static void rows_of(F& fn, const CrabbyChunk* raw, std::index_sequence<I...>) {
            const std::tuple<Ts*...> cols{static_cast<Ts*>(raw->columns[I])...};
            const std::size_t n = raw->count;
            if constexpr (WithEntity) {
                const CrabbyEntity* es = raw->entities;
                for (std::size_t i = 0; i < n; ++i)
                    fn(entity(es[i].generation) << 32 | es[i].index, std::get<I>(cols)[i]...);
            } else {
                for (std::size_t i = 0; i < n; ++i) fn(std::get<I>(cols)[i]...);
            }
        }
        template<class F, class Filters> static void run(world_view& w, Filters filters, bool parallel, F& fn) {
            auto per_chunk = [&fn](chunk<Ts...> c) { chunk_rows(fn, c.raw_chunk(), std::index_sequence_for<Ts...>{}); };
            w.run_query(detail::types<Ts...>{}, filters, parallel, per_chunk);
        }
    };
    template<class Args> struct each_of;
    template<class First, class... Rest> struct each_of<std::tuple<First, Rest...>> {
        static constexpr bool with_entity = std::is_same<First, entity>::value;
        template<class F, class Filters> static void run(world_view& w, Filters filters, bool parallel, F& fn) {
            if constexpr (with_entity) each_row<true, detail::column<Rest>...>::run(w, filters, parallel, fn);
            else each_row<false, detail::column<First>, detail::column<Rest>...>::run(w, filters, parallel, fn);
        }
    };
public:
    // ---- entities and components ----
    /// A new entity with these components: `world.spawn(crabby::transform_at({0, 0}), Health{100})`.
    template<class... Ts> entity spawn(const Ts&... components) {
        valid();
        constexpr std::size_t polygons = (std::size_t{std::is_same<Ts, polygon>::value} + ... + 0);
        static_assert(polygons <= 1, "one collider per entity");
        entity result = 0;
        if constexpr (polygons == 0) {
            std::array<CrabbyValue, sizeof...(Ts)> args{{CrabbyValue{id<Ts>(), &components, sizeof(Ts)}...}};
            detail::check(crabby_spawn(raw_, args.data(), args.size(), &result));
        } else {
            // C has no polygon component value: spawn the rest, then add it.
            std::array<CrabbyValue, sizeof...(Ts) - 1> args{};
            std::size_t n = 0;
            const polygon* shape = nullptr;
            auto add = [&](const auto& c) {
                using C = std::decay_t<decltype(c)>;
                if constexpr (std::is_same<C, polygon>::value) shape = &c;
                else args[n++] = CrabbyValue{id<C>(), &c, sizeof(C)};
            };
            (add(components), ...);
            detail::check(crabby_spawn(raw_, args.data(), args.size(), &result));
            const auto status = crabby_set_polygon_collider(raw_, result, shape->points.data(), shape->count, *shape);
            if (status != CRABBY_OK) crabby_despawn(raw_, result);
            detail::check(status);
        }
        return result;
    }
    void despawn(entity e) { valid(); detail::check(crabby_despawn(raw_, e)); }
    bool alive(entity e) const { valid(); bool result = false; detail::check(crabby_is_alive(raw_, e, &result)); return result; }
    /// How many entities are alive.
    std::size_t size() const { valid(); std::size_t result = 0; detail::check(crabby_world_len(raw_, &result)); return result; }
    /// A copy of `e`'s `T`; throws if `e` is dead or has no `T` (see try_get).
    template<class T> T get(entity e) const {
        T result{}; detail::check(crabby_get(raw_, e, id<T>(), &result, sizeof(T))); return result;
    }
    /// A copy of `e`'s `T`, or nothing if `e` is dead or has no `T`.
    template<class T> std::optional<T> try_get(entity e) const {
        T result{};
        const auto status = crabby_get(raw_, e, id<T>(), &result, sizeof(T));
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return result;
    }
    template<class T> bool has(entity e) const { return try_get<T>(e).has_value(); }
    /// Adds or replaces `e`'s component of this type: `world.set(e, Health{50})`.
    template<class T> void set(entity e, const T& component) {
        detail::check(crabby_set(raw_, e, id<T>(), &component, sizeof(T)));
    }
    template<class T> void remove(entity e) { detail::check(crabby_remove(raw_, e, id<T>())); }
    /// Gives `e` a polygon collider (see polygon_collider).
    void set(entity e, const polygon& shape) {
        detail::check(crabby_set_polygon_collider(raw_, e, shape.points.data(), shape.count, shape));
    }
    /// `e`'s polygon corners (empty if its collider isn't a polygon).
    std::vector<vec2> collider_points(entity e) const {
        vec2 points[8];
        std::uint32_t n = 0;
        detail::check(crabby_collider_points(raw_, e, points, &n));
        return std::vector<vec2>(points, points + n);
    }

    // ---- resources: one value of a type per world, not on any entity ----
    /// Adds or replaces the world's `T`: `world.set_resource(Score{0, 0})`.
    template<class T> void set_resource(const T& value) {
        static_assert(detail::pod<T>, "resources are plain structs (trivially copyable, no destructor)");
        valid();
        detail::check(crabby_resource_set(raw_, detail::type_name<T>().c_str(), &value, sizeof(T), alignof(T)));
    }
    /// A copy of the world's `T`; throws if it has none (see try_resource).
    template<class T> T resource() const {
        auto value = try_resource<T>();
        if (!value) throw std::runtime_error("the world has no " + detail::type_name<T>() + " resource");
        return *value;
    }
    /// A copy of the world's `T`, or nothing.
    template<class T> std::optional<T> try_resource() const {
        static_assert(detail::pod<T>, "resources are plain structs (trivially copyable, no destructor)");
        valid();
        T result{};
        const auto status = crabby_resource_get(raw_, detail::type_name<T>().c_str(), &result, sizeof(T));
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return result;
    }
    /// Removes the world's `T`; returns whether it had one.
    template<class T> bool remove_resource() {
        valid();
        const auto status = crabby_resource_remove(raw_, detail::type_name<T>().c_str());
        if (status == CRABBY_NOT_FOUND) return false;
        detail::check(status);
        return true;
    }

    // ---- queries ----
    /// Calls `fn` for every entity that has all the components `fn` takes:
    ///
    ///     world.each([](crabby::transform2d& t, const Velocity& v) { t.position += v.value; });
    ///
    /// Take components by `&` to change them, by `const&` to read them. A
    /// first `crabby::entity` parameter gets the entity. As fast as a
    /// hand-written loop over the columns (see query).
    ///
    /// Filters narrow it to what changed since the calling system last ran
    /// (see changed, added, any_of):
    ///
    ///     world.each<crabby::changed<Health>>([](crabby::entity e, const Health& h) { ... });
    ///
    /// A component taken by `&` counts as changed for every entity visited,
    /// written or not; one taken by `const&` or by value doesn't.
    template<class... Filters, class F> void each(F&& fn) {
        static_assert(std::conjunction<detail::is_filter<Filters>...>::value,
                      "each<...> takes filters: crabby::changed<T>, crabby::added<T> or crabby::any_of<...>");
        using args = typename detail::params<detail::bare<F>>::type;
        each_of<args>::run(*this, detail::types<Filters...>{}, false, fn);
    }
    /// Like each, but on all CPU cores at once: `fn` must be thread-safe and
    /// only touch its own entity's components (plus ctx::dt/random and
    /// spatial queries).
    template<class... Filters, class F> void each_par(F&& fn) {
        static_assert(std::conjunction<detail::is_filter<Filters>...>::value,
                      "each_par<...> takes filters: crabby::changed<T>, crabby::added<T> or crabby::any_of<...>");
        using args = typename detail::params<detail::bare<F>>::type;
        each_of<args>::run(*this, detail::types<Filters...>{}, true, fn);
    }
    /// The lower-level loop: calls `fn(chunk)` for every archetype with all of
    /// `Ts`, with a column (array) per component, for code that works on
    /// whole arrays:
    ///
    ///     world.query<Position, Velocity>([](auto chunk) {
    ///         auto [pos, vel] = chunk.columns();
    ///         for (std::size_t i = 0; i < chunk.size(); ++i) pos[i].x += vel[i].x;
    ///     });
    ///
    /// A `const` column (`query<Position, const Velocity>`) is only read; the
    /// others count as changed. Filters go in the same list and aren't
    /// columns: `query<Position, crabby::changed<Position>>` gets the runs of
    /// rows whose Position changed.
    template<class... Ts, class F> void query(F&& fn) {
        run_query(typename detail::pick<false, Ts...>::type{}, typename detail::pick<true, Ts...>::type{}, false, fn);
    }
    /// query on all CPU cores at once (see each_par).
    template<class... Ts, class F> void query_par(F&& fn) {
        run_query(typename detail::pick<false, Ts...>::type{}, typename detail::pick<true, Ts...>::type{}, true, fn);
    }
    /// How many entities have all of these components (and pass these
    /// filters: `count<crabby::changed<Health>>()`). Counting changes nothing.
    template<class... Ts> std::size_t count() {
        return count_rows(typename detail::pick<false, Ts...>::type{}, typename detail::pick<true, Ts...>::type{});
    }

    // ---- change detection ----
    /// Starts recording when `T` values are added, changed and removed, for
    /// the changed/added filters and removed(). Until then a component keeps
    /// no change data and costs nothing. Values that exist already count as
    /// added and changed now. Using a filter on `T` does this too.
    template<class T> void track_changes() { detail::check(crabby_component_track_changes(raw_, id<T>())); }
    /// Marks `e`'s `T` as changed now, for a write the world can't see.
    /// False if `e` is dead or has no `T`.
    template<class T> bool mark_changed(entity e) {
        const auto status = crabby_mark_changed(raw_, e, id<T>());
        if (status == CRABBY_NOT_FOUND) return false;
        detail::check(status);
        return true;
    }
    /// The entities that lost their `T` since the calling system last ran
    /// (it was removed, or the entity despawned), oldest first. An entity is
    /// listed once per removal and may have a `T` again. `T` must be tracked
    /// (track_changes) or this throws.
    template<class T> std::vector<entity> removed() const { return removed_since<T>(CRABBY_SINCE_LAST_RUN); }
    /// removed() for a reader that keeps its own tick (see advance_change_tick).
    template<class T> std::vector<entity> removed_since(std::uint64_t tick) const {
        const std::uint32_t component = id<T>();
        std::size_t n = 0;
        detail::check(crabby_removed(raw_, component, tick, nullptr, 0, &n));
        std::vector<entity> out(n);
        if (n) detail::check(crabby_removed(raw_, component, tick, out.data(), out.size(), &n));
        return out;
    }
    /// The world's current change tick: changes made now carry it.
    std::uint64_t change_tick() const { valid(); std::uint64_t tick = 0; detail::check(crabby_world_change_tick(raw_, &tick)); return tick; }
    /// Ends the current change tick and returns it. Code that isn't a game
    /// system keeps its place this way: `set_changes_since(seen)`, look, then
    /// `seen = advance_change_tick()`. A game does this around every system.
    std::uint64_t advance_change_tick() { valid(); std::uint64_t tick = 0; detail::check(crabby_world_advance_change_tick(raw_, &tick)); return tick; }
    /// The tick the filters and removed() compare with: inside a game system,
    /// when that system last finished; otherwise 0 (everything) unless set.
    std::uint64_t changes_since() const { valid(); std::uint64_t tick = 0; detail::check(crabby_world_changes_since(raw_, &tick)); return tick; }
    void set_changes_since(std::uint64_t tick) { valid(); detail::check(crabby_world_set_changes_since(raw_, tick)); }

    // ---- reflection and saves ----
    /// Describes `T`'s fields, so tools and agents can read and write it as
    /// JSON (`ponko-mcp`) and `save()` includes it. It's saved under its
    /// C++ type name, or under `name` (call this before `T` is first used).
    template<class T> void reflect(std::initializer_list<field_info<T>> fields) { reflect<T>(nullptr, fields); }
    template<class T> void reflect(const char* name, std::initializer_list<field_info<T>> fields) {
        valid();
        const std::uint32_t id = register_type<T>(raw_, *owner_, name);
        std::vector<CrabbyField> raw;
        raw.reserve(fields.size());
        for (const auto& f : fields) raw.push_back(f.raw);
        detail::check(crabby_component_reflect(raw_, id, raw.data(), raw.size()));
    }
    /// Describes resource `T`'s fields, so tools and agents can read and
    /// set it and `save()` includes it (under its C++ type name).
    template<class T> void reflect_resource(std::initializer_list<field_info<T>> fields) {
        static_assert(detail::pod<T>, "resources are plain structs (trivially copyable, no destructor)");
        valid();
        std::vector<CrabbyField> raw;
        raw.reserve(fields.size());
        for (const auto& f : fields) raw.push_back(f.raw);
        detail::check(crabby_resource_reflect(raw_, detail::type_name<T>().c_str(), sizeof(T), alignof(T), raw.data(), raw.size()));
    }
    /// Every entity with reflected components as JSON, with its id.
    std::string save() const {
        valid(); std::size_t len = 0;
        detail::check(crabby_world_save(raw_, nullptr, 0, &len));
        std::string out(len + 1, '\0');
        detail::check(crabby_world_save(raw_, &out[0], out.size(), &len));
        out.resize(len);
        return out;
    }
    /// Replaces every entity with those in `json` (from save()), with the
    /// same ids. On error (an exception) the world is unchanged.
    void load(const std::string& json) { valid(); detail::check(crabby_world_load(raw_, json.c_str())); }

    /// Describes reflected component `name`'s JSON shape for discovery (the
    /// `capabilities` answer, see ctx::capabilities_json), encoded as Rust's
    /// ValueSchema: `{"Object":[["hp","Number"]]}`. Throws for invalid JSON or
    /// a name that isn't reflected; saving and loading don't change.
    void describe_component(const char* name, const char* schema) {
        valid();
        detail::check(crabby_world_describe_component(raw_, name, schema));
    }
    /// describe_component() for reflected resource `name`.
    void describe_resource(const char* name, const char* schema) {
        valid();
        detail::check(crabby_world_describe_resource(raw_, name, schema));
    }

    // ---- the rest ----
    /// Gives `e` an empty `width` x `height` tilemap (its Transform2D is the top-left).
    tilemap_ref add_tilemap(entity e, const sprite_sheet& sheet, std::uint32_t width, std::uint32_t height);
    /// `e`'s tilemap; throws if it has none. Valid until `e` loses its map.
    tilemap_ref tilemap(entity e);
    /// Reorders Transform2D entities so that neighbors are close in memory.
    void sort_spatial(float cell_size) { valid(); detail::check(crabby_world_sort_spatial(raw_, cell_size)); }
};
inline tilemap_ref world_view::add_tilemap(entity e, const sprite_sheet& sheet, std::uint32_t width, std::uint32_t height) {
    valid(); detail::check(crabby_tilemap_add(raw_, e, sheet, width, height)); return tilemap_ref(raw_, e);
}
inline tilemap_ref world_view::tilemap(entity e) { valid(); return tilemap_ref(raw_, e); }
/// An owned world (from crabby::game::run_headless, or empty).
class world final : public world_view {
    friend class game;
    world(CrabbyWorld* raw, std::shared_ptr<detail::identity> owner) : world_view(raw, std::move(owner)) {}
public:
    world() : world_view(nullptr, std::make_shared<detail::identity>()) {
        raw_ = crabby_world_new();
        if (!raw_) throw std::runtime_error(crabby_last_error());
    }
    ~world() { if (raw_) (void)crabby_world_free(raw_); }
    world(const world&) = delete;
    world& operator=(const world&) = delete;
    world(world&& other) noexcept : world_view(other.raw_, std::move(other.owner_)) { other.raw_ = nullptr; }
    world& operator=(world&& other) {
        if (this != &other) {
            if (raw_) detail::check(crabby_world_free(raw_));
            raw_ = other.raw_; owner_ = std::move(other.owner_); other.raw_ = nullptr;
        }
        return *this;
    }
};

/// A read-only view of a spatial grid that answers queries inline (no call
/// into the engine per query), from any thread. Get one after the grid is
/// rebuilt: `grid::view()` or `ctx::spatial_view()`; it expires on the next
/// rebuild. `Entry` is grid_item (owned grids) or grid_hit (the built-in grid).
template<class Entry> class grid_view {
    CrabbyGridView v_{};
    static std::int32_t cell(float x) {
        // floor, then saturate like Rust's `as i32` (NaN -> 0).
        const float f = std::floor(x);
        if (f != f) return 0;
        if (f >= 2147483648.0f) return INT32_MAX;
        if (f <= -2147483648.0f) return INT32_MIN;
        return std::int32_t(f);
    }
    static std::uint64_t pack(std::int64_t cx, std::int64_t cy) {
        return std::uint64_t(std::uint32_t(cx)) << 32 | std::uint32_t(cy);
    }
    std::size_t bucket(std::int64_t cx, std::int64_t cy) const {
        const std::uint32_t ux = std::uint32_t(cx), uy = std::uint32_t(cy);
        const std::uint64_t local = ((std::uint64_t(uy) & ((1ull << v_.y_bits) - 1)) << v_.x_bits) |
                                    (std::uint64_t(ux) & ((1ull << v_.x_bits) - 1));
        const std::uint64_t tx = ux >> v_.x_bits, ty = uy >> v_.y_bits;
        const std::uint64_t h = tx * 0x9E3779B97F4A7C15ull ^ ty * 0xC2B2AE3D27D4EB4Full;
        return std::size_t((local ^ (h >> 32)) & v_.bucket_mask);
    }
    const Entry* entries() const { return static_cast<const Entry*>(v_.entries); }
public:
    grid_view() = default;
    explicit grid_view(const CrabbyGridView& v) : v_(v) {
        if (v.len && v.entry_size != sizeof(Entry)) throw std::invalid_argument("grid_view entry type does not match the grid");
    }
    std::size_t size() const noexcept { return v_.len; }
    /// Calls `f(const Entry&)` for every entry within `radius` of `center` (inclusive).
    template<class F> void for_each_in_radius(vec2 center, float radius, F&& f) const {
        if (v_.len == 0 || !(radius >= 0)) return;
        const float r2 = radius * radius;
        const std::int64_t x0 = cell((center.x - radius) * v_.inv_cell), y0 = cell((center.y - radius) * v_.inv_cell);
        const std::int64_t x1 = cell((center.x + radius) * v_.inv_cell), y1 = cell((center.y + radius) * v_.inv_cell);
        const Entry* e = entries();
        if ((x1 - x0 + 1) * (y1 - y0 + 1) > std::int64_t(v_.len)) {
            for (std::size_t i = 0; i < v_.len; ++i)
                if (length_squared(e[i].position - center) <= r2) f(e[i]);
            return;
        }
        for (std::int64_t cy = y0; cy <= y1; ++cy) {
            for (std::int64_t cx = x0; cx <= x1; ++cx) {
                const std::size_t b = bucket(cx, cy);
                const std::uint64_t key = pack(cx, cy);
                for (std::uint32_t i = v_.starts[b], end = v_.starts[b + 1]; i < end; ++i)
                    if (v_.cells[i] == key && length_squared(e[i].position - center) <= r2) f(e[i]);
            }
        }
    }
};

/// A spatial grid you own: rebuild it from items, query it from any thread.
class grid {
    CrabbyGrid* raw_;
public:
    explicit grid(float cell_size) : raw_(crabby_grid_new(cell_size)) { if (!raw_) throw std::invalid_argument(crabby_last_error()); }
    ~grid() { crabby_grid_free(raw_); }
    grid(const grid&) = delete;
    grid& operator=(const grid&) = delete;
    void rebuild(const std::vector<grid_item>& items) { detail::check(crabby_grid_rebuild(raw_, items.data(), items.size())); }
    std::size_t size() const { return crabby_grid_len(raw_); }
    /// An inline-query view, valid until the next rebuild. Fastest for hot loops.
    grid_view<grid_item> view() const { CrabbyGridView v{}; detail::check(crabby_grid_view(raw_, &v)); return grid_view<grid_item>(v); }
    /// Items within `radius` of `center` into `out` (reuses its capacity).
    void radius(vec2 center, float r, std::vector<grid_item>& out) const {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(crabby_grid_radius(raw_, center, r, out.data(), out.size(), &n));
        if (n > out.size()) { out.resize(n); detail::check(crabby_grid_radius(raw_, center, r, out.data(), out.size(), &n)); }
        out.resize(n);
    }
};

/// Walking costs for flow fields: 1 is normal ground, up to 255 slower,
/// nav_grid::wall can't be entered.
class nav_grid {
    CrabbyNavGrid* raw_;
    std::uint8_t* costs_ = nullptr;
    std::uint32_t width_, height_;
    float cell_size_;
    vec2 origin_{0, 0};
public:
    static constexpr std::uint8_t wall = CRABBY_NAV_WALL;
    nav_grid(std::uint32_t width, std::uint32_t height, float cell_size)
        : raw_(crabby_nav_new(width, height, cell_size)), width_(width), height_(height), cell_size_(cell_size) {
        if (!raw_) throw std::invalid_argument(crabby_last_error());
        std::size_t len = 0;
        detail::check(crabby_nav_costs(raw_, &costs_, &len));
    }
    ~nav_grid() { crabby_nav_free(raw_); }
    nav_grid(const nav_grid&) = delete;
    nav_grid& operator=(const nav_grid&) = delete;
    const CrabbyNavGrid* raw() const noexcept { return raw_; }
    std::uint32_t width() const noexcept { return width_; }
    std::uint32_t height() const noexcept { return height_; }
    float cell_size() const noexcept { return cell_size_; }
    /// Where cell (0, 0)'s top-left corner is. Default (0, 0).
    void set_origin(vec2 o) { detail::check(crabby_nav_set_origin(raw_, o)); origin_ = o; }
    vec2 origin() const noexcept { return origin_; }
    /// Sets the cost of a cell; cells outside the grid are ignored.
    void set_cost(std::uint32_t x, std::uint32_t y, std::uint8_t cost) noexcept {
        if (x < width_ && y < height_) costs_[std::size_t(y) * width_ + x] = cost;
    }
    /// The cost of a cell, or nothing outside the grid.
    std::optional<std::uint8_t> cost(std::uint32_t x, std::uint32_t y) const noexcept {
        if (x >= width_ || y >= height_) return std::nullopt;
        return costs_[std::size_t(y) * width_ + x];
    }
    /// All costs row by row, for filling a map at once.
    std::uint8_t* costs() noexcept { return costs_; }
    /// The cell under a world point, or nothing outside the grid.
    std::optional<std::array<std::uint32_t, 2>> cell_at(vec2 p) const noexcept {
        const float cx = std::floor((p.x - origin_.x) / cell_size_), cy = std::floor((p.y - origin_.y) / cell_size_);
        if (!(cx >= 0 && cy >= 0 && cx < float(width_) && cy < float(height_))) return std::nullopt;
        return std::array<std::uint32_t, 2>{{std::uint32_t(cx), std::uint32_t(cy)}};
    }
    /// Inside the grid and not a wall.
    bool walkable(vec2 p) const noexcept {
        const auto c = cell_at(p);
        return c && costs_[std::size_t((*c)[1]) * width_ + (*c)[0]] != wall;
    }
    /// The world position of a cell's center.
    vec2 center_of(std::uint32_t x, std::uint32_t y) const noexcept {
        return origin_ + vec2{(float(x) + 0.5f) * cell_size_, (float(y) + 0.5f) * cell_size_};
    }
};

/// One shape a mesh can blend toward (Rust `MorphTarget`; a blend shape:
/// a smile, a dent): one offset per corner in `positions`, plus one normal
/// offset per corner in `normals` (empty when normals don't change).
struct morph_target {
    /// What the shape is called, to find it with ctx::morph_target.
    std::string name;
    /// One offset per corner.
    std::vector<vec3> positions;
    /// One normal offset per corner; empty when normals don't change.
    std::vector<vec3> normals;
};

/// Triangles on the CPU (Rust `MeshData`), before ctx::create_mesh puts
/// them on the GPU.
class ctx;
class mesh_data {
    friend class ctx;
    CrabbyMeshData* raw_;
    explicit mesh_data(CrabbyMeshData* raw) : raw_(raw) { if (!raw_) detail::check(CRABBY_INVALID_ARGUMENT); }
public:
    /// Corners and three indices per triangle, counter-clockwise seen from
    /// the front. Without normals they're computed; without texture
    /// coordinates they're zero. Throws if they don't fit together.
    mesh_data(const std::vector<vec3>& positions, const std::vector<std::uint32_t>& indices,
              const std::vector<vec3>& normals = {}, const std::vector<vec2>& uvs = {})
        : raw_(nullptr) {
        if ((!normals.empty() && normals.size() != positions.size()) || (!uvs.empty() && uvs.size() != positions.size()))
            throw std::invalid_argument("mesh_data: one normal and texture coordinate per corner");
        raw_ = crabby_mesh_data_new(positions.data(), normals.empty() ? nullptr : normals.data(),
                                    uvs.empty() ? nullptr : uvs.data(), positions.size(), indices.data(), indices.size());
        if (!raw_) detail::check(CRABBY_INVALID_ARGUMENT);
    }
    /// A cube `size` wide around the origin.
    static mesh_data cube(float size) { return mesh_data(crabby_mesh_data_cube(size)); }
    /// A flat square `size` wide on the XZ plane, facing up: ground.
    static mesh_data plane(float size) { return mesh_data(crabby_mesh_data_plane(size)); }
    /// A sphere of `radius` from `segments` slices around.
    static mesh_data sphere(float radius, std::uint32_t segments) { return mesh_data(crabby_mesh_data_sphere(radius, segments)); }
    /// A square of terrain `size` wide from n x n heights, row after row
    /// along +Z (NaN: a hole), using every `step`-th one, with skirts at the
    /// edges. Throws unless the heights are a square grid of at least 2 x 2.
    static mesh_data heightfield(const std::vector<float>& heights, float size, std::uint32_t step = 1) {
        return mesh_data(crabby_mesh_data_heightfield(heights.data(), heights.size(), size, step));
    }
    ~mesh_data() { crabby_mesh_data_free(raw_); }
    mesh_data(mesh_data&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    mesh_data& operator=(mesh_data&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    mesh_data(const mesh_data&) = delete;
    mesh_data& operator=(const mesh_data&) = delete;
    /// One color per corner, multiplying the material's color; empty
    /// removes them. Throws unless there's one per corner.
    void set_colors(const std::vector<color>& colors) {
        detail::check(crabby_mesh_data_set_colors(raw_, colors.empty() ? nullptr : colors.data(), colors.size()));
    }
    /// Up to four joints per corner and how much each moves it, for
    /// ctx::create_skinned_mesh; empty removes them. Throws unless there's
    /// one of each per corner.
    void set_skin(const std::vector<std::array<std::uint16_t, 4>>& joints,
                  const std::vector<std::array<float, 4>>& weights) {
        if (joints.size() != weights.size()) throw std::invalid_argument("mesh_data::set_skin: one weight set per joint set");
        detail::check(crabby_mesh_data_set_skin(raw_, joints.empty() ? nullptr : joints.front().data(),
                                                weights.empty() ? nullptr : weights.front().data(), joints.size()));
    }
    /// Links existing target deltas as a frame of a piecewise linear channel.
    void add_morph_frame(std::size_t channel, std::size_t target, float weight) {
        detail::check(crabby_mesh_data_add_morph_frame(raw_, channel, target, weight));
    }
    /// Adds a morph target (Rust `MeshData::morph_targets`): a shape the
    /// mesh can blend toward. Targets are numbered in the order they're
    /// added. Throws unless `positions` (and `normals`, when given) have one
    /// offset per corner.
    void add_morph_target(const morph_target& target) {
        if (!target.normals.empty() && target.normals.size() != target.positions.size())
            throw std::invalid_argument("mesh_data::add_morph_target: one normal offset per corner");
        detail::check(crabby_mesh_data_add_morph_target(raw_, target.name.c_str(), target.positions.data(),
                                                        target.normals.empty() ? nullptr : target.normals.data(),
                                                        target.positions.size()));
    }
    /// Its arrays, valid while it lives.
    CrabbyMeshView view() const { CrabbyMeshView v{}; detail::check(crabby_mesh_data_view(raw_, &v)); return v; }
    /// A decal (Rust `MeshData::decal`): the part of this mesh inside a box,
    /// as mesh data of its own with an image projected straight onto it.
    /// `projector` (16 floats, column after column) maps the unit cube
    /// (-0.5 to 0.5 on each axis) onto the box in this mesh's space, so its
    /// scale is the decal's size and depth; the image is projected along the
    /// box's -Z, its top along +Y, and only faces turned toward +Z get it.
    /// Corners are lifted `lift` meters along their normals (a few
    /// millimeters is typical). It has no triangles (`view().index_count ==
    /// 0`) when the box misses the mesh; don't make a mesh from an empty
    /// one. Throws if the projector isn't finite.
    mesh_data decal(const std::array<float, 16>& projector, float lift) const {
        return mesh_data(crabby_mesh_data_decal(raw_, projector.data(), lift));
    }
    const CrabbyMeshData* raw() const noexcept { return raw_; }
};

/// An animation clip of a glTF file with skins (Rust `AnimationClip`),
/// from ctx::load_clip; it owns its handle.
using lane = CrabbyLane;
using message_stats = CrabbyMessageStats;
/// One event from net_messages::next_event (bytes and reason copied).
struct message_event {
    CrabbyMessageEventKind kind;
    std::uint64_t peer;
    lane on;
    std::vector<std::uint8_t> bytes;
    std::string reason;
};

/// A message endpoint the game owns (Rust `NetMessages`): poll it every
/// frame, send bytes on a reliable or unreliable lane, read events. The
/// host is peer 0. Plain UDP: not authenticated or encrypted.
/// Steam lobby visibility (feature steam in the library).
enum class steam_lobby_visibility : std::uint32_t { friends_only = 0, public_lobby = 1, private_lobby = 2 };
/// Owned asynchronous Steam lobby. Poll each frame; id is zero until ready.
class steam_lobby {
    CrabbySteamLobby* raw_ = nullptr;
    explicit steam_lobby(CrabbySteamLobby* raw) : raw_(raw) {}
public:
    ~steam_lobby() { crabby_steam_lobby_free(raw_); }
    steam_lobby(steam_lobby&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    steam_lobby& operator=(steam_lobby&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    steam_lobby(const steam_lobby&) = delete;
    steam_lobby& operator=(const steam_lobby&) = delete;
    static steam_lobby create(std::uint32_t app_id, steam_lobby_visibility visibility, std::uint32_t max_members) {
        CrabbySteamLobby* p = nullptr;
        detail::check(crabby_steam_lobby_create(app_id, static_cast<std::uint32_t>(visibility), max_members, &p));
        return steam_lobby(p);
    }
    static steam_lobby join(std::uint32_t app_id, std::uint64_t id) {
        CrabbySteamLobby* p = nullptr; detail::check(crabby_steam_lobby_join(app_id, id, &p)); return steam_lobby(p);
    }

    void poll() { detail::check(crabby_steam_lobby_poll(raw_)); }
    void leave() { detail::check(crabby_steam_lobby_leave(raw_)); }

    std::uint64_t id() const { std::uint64_t v = 0; detail::check(crabby_steam_lobby_id(raw_, &v)); return v; }
    void invite() const { detail::check(crabby_steam_lobby_invite(raw_)); }
    void set_metadata(const char* key, const char* value) { detail::check(crabby_steam_lobby_set_metadata(raw_, key, value)); }
    std::optional<std::string> metadata(const char* key) const {
        std::size_t n = 0; detail::check(crabby_steam_lobby_metadata(raw_, key, nullptr, 0, &n));
        if (n == 0) return std::nullopt;
        std::vector<char> b(n); detail::check(crabby_steam_lobby_metadata(raw_, key, b.data(), b.size(), &n));
        return std::string(b.data());
    }
    std::vector<std::uint64_t> members() const {
        std::size_t n = 0; detail::check(crabby_steam_lobby_members(raw_, nullptr, 0, &n));
        std::vector<std::uint64_t> b(n); detail::check(crabby_steam_lobby_members(raw_, b.data(), b.size(), &n));
        b.resize(n); return b;
    }
};

class net_messages {
    CrabbyNetMessages* raw_ = nullptr;
    explicit net_messages(CrabbyNetMessages* raw) : raw_(raw) {}
public:
    ~net_messages() { crabby_net_messages_free(raw_); }
    net_messages(net_messages&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    net_messages& operator=(net_messages&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    net_messages(const net_messages&) = delete;
    net_messages& operator=(const net_messages&) = delete;
    /// Hosts at `addr` ("0.0.0.0:7777"; port 0 picks one) for up to `max_clients`.
    static net_messages host(const char* addr, std::size_t max_clients) {
        CrabbyNetMessages* m = nullptr; detail::check(crabby_net_messages_host(addr, max_clients, &m)); return net_messages(m);
    }
    /// Joins the host at `addr`.
    static net_messages join(const char* addr) {
        CrabbyNetMessages* m = nullptr; detail::check(crabby_net_messages_join(addr, &m)); return net_messages(m);
    }
    /// `players` endpoints joined in memory (the first hosts), for tests.
    static std::vector<net_messages> loopback(std::size_t players) {
        std::vector<CrabbyNetMessages*> raw(players);
        detail::check(crabby_net_messages_loopback(players, raw.data()));
        std::vector<net_messages> out;
        for (auto* m : raw) out.push_back(net_messages(m));
        return out;
    }
    std::uint64_t steam_peer(std::uint64_t peer) const { std::uint64_t id = 0; detail::check(crabby_net_messages_steam_peer(raw_, peer, &id)); return id; }
    /// Steam relays plus a friends-only lobby, for 1..=249 clients.
    static net_messages steam_host(std::uint32_t app_id, std::size_t max_clients) {
        CrabbyNetMessages* p = nullptr; detail::check(crabby_net_messages_steam_host(app_id, max_clients, &p)); return net_messages(p);
    }
    static net_messages steam_join(std::uint32_t app_id, std::uint64_t lobby) {
        CrabbyNetMessages* p = nullptr; detail::check(crabby_net_messages_steam_join(app_id, lobby, &p)); return net_messages(p);
    }

    std::uint64_t steam_lobby_id() const { std::uint64_t v = 0; detail::check(crabby_net_messages_steam_lobby_id(raw_, &v)); return v; }
    void steam_invite() const { detail::check(crabby_net_messages_steam_lobby_invite(raw_)); }
    void steam_set_metadata(const char* key, const char* value) { detail::check(crabby_net_messages_steam_lobby_set_metadata(raw_, key, value)); }
    std::optional<std::string> steam_metadata(const char* key) const {
        std::size_t n = 0; detail::check(crabby_net_messages_steam_lobby_metadata(raw_, key, nullptr, 0, &n));
        if (n == 0) return std::nullopt;
        std::vector<char> b(n); detail::check(crabby_net_messages_steam_lobby_metadata(raw_, key, b.data(), b.size(), &n));
        return std::string(b.data());
    }
    std::vector<std::uint64_t> steam_members() const {
        std::size_t n = 0; detail::check(crabby_net_messages_steam_lobby_members(raw_, nullptr, 0, &n));
        std::vector<std::uint64_t> b(n); detail::check(crabby_net_messages_steam_lobby_members(raw_, b.data(), b.size(), &n));
        b.resize(n); return b;
    }

    /// How long a silent peer may stay connected.
    void set_timeout(std::uint64_t ms) { detail::check(crabby_net_messages_set_timeout(raw_, ms)); }
    std::uint16_t local_port() const { std::uint16_t p = 0; detail::check(crabby_net_messages_local_port(raw_, &p)); return p; }
    /// Sends `len` bytes (at most 1 MiB) to `peer`.
    void send(std::uint64_t peer, lane l, const void* bytes, std::size_t len) {
        detail::check(crabby_net_messages_send(raw_, peer, l, static_cast<const std::uint8_t*>(bytes), len));
    }
    void send(std::uint64_t peer, lane l, const std::string& text) { send(peer, l, text.data(), text.size()); }
    void send_to_all(lane l, const void* bytes, std::size_t len) {
        detail::check(crabby_net_messages_send_to_all(raw_, l, static_cast<const std::uint8_t*>(bytes), len));
    }
    void send_to_all(lane l, const std::string& text) { send_to_all(l, text.data(), text.size()); }
    /// Sends what's queued and receives what arrived; then read next_event until empty.
    void poll() { detail::check(crabby_net_messages_poll(raw_)); }
    /// The oldest event, or std::nullopt.
    std::optional<message_event> next_event() {
        CrabbyMessageEvent e{};
        bool found = false;
        detail::check(crabby_net_messages_next_event(raw_, &e, &found));
        if (!found) return std::nullopt;
        return message_event{e.kind, e.peer, e.lane, std::vector<std::uint8_t>(e.bytes, e.bytes + e.len), std::string(e.reason)};
    }
    void disconnect(std::uint64_t peer) { detail::check(crabby_net_messages_disconnect(raw_, peer)); }
    message_stats stats() const { message_stats s{}; detail::check(crabby_net_messages_stats(raw_, &s)); return s; }
};

// ---- Replication helpers (sit on top of net_messages; the packet format stays yours) ----

/// Is `a` newer than `b` among 16-bit sequence numbers that wrap around
/// (Rust `seq_newer`)? Equal numbers are not newer.
inline bool seq_newer(std::uint16_t a, std::uint16_t b) { return crabby_seq_newer(a, b); }
/// How far `a` is ahead of `b` among wrapping 16-bit sequence numbers;
/// negative when behind.
inline std::int16_t seq_diff(std::uint16_t a, std::uint16_t b) { return crabby_seq_diff(a, b); }
/// A state update's authority epoch and wrapping sequence number (Rust `UpdateStamp`).
using update_stamp = CrabbyUpdateStamp;
/// Is stamp `a` newer than `b`: a newer epoch, or the same epoch and a newer sequence?
inline bool stamp_newer(update_stamp a, update_stamp b) { return crabby_stamp_newer(a, b); }

/// How an update compares with an authority record (Rust `AuthorityCheck`).
enum class authority_check : std::uint32_t {
    accept = CRABBY_AUTHORITY_ACCEPT,           ///< From the owner, current epoch: apply it.
    stale = CRABBY_AUTHORITY_STALE,             ///< Older epoch, before a handoff: drop it.
    wrong_owner = CRABBY_AUTHORITY_WRONG_OWNER, ///< Current epoch, not from the owner: drop it.
    ahead = CRABBY_AUTHORITY_AHEAD,             ///< Newer epoch than we know: the handoff announcement is still on its way.
};
/// Who owns an entity: an owner peer and an epoch (Rust `Authority`). A
/// handoff bumps the epoch, and updates stamped with an older one are
/// rejected, so a late packet from the previous owner can't undo it. Peer 0 is the host.
class authority {
    CrabbyAuthority a_;
public:
    explicit authority(std::uint64_t owner) : a_(crabby_authority_new(owner)) {}
    std::uint64_t owner() const noexcept { return a_.owner; }
    std::uint16_t epoch() const noexcept { return a_.epoch; }
    /// Gives the entity to `new_owner`; returns the new epoch to announce reliably.
    std::uint16_t handoff(std::uint64_t new_owner) {
        std::uint16_t epoch = 0; detail::check(crabby_authority_handoff(&a_, new_owner, &epoch)); return epoch;
    }
    /// Applies a handoff announced by another peer; true if it changed the
    /// record. Older epochs are ignored; the same epoch with another owner is
    /// a race that the lower owner id wins.
    bool adopt(std::uint64_t new_owner, std::uint16_t epoch) {
        bool changed = false; detail::check(crabby_authority_adopt(&a_, new_owner, epoch, &changed)); return changed;
    }
    /// Judges an update from peer `from` stamped with `epoch`.
    authority_check check(std::uint64_t from, std::uint16_t epoch) const {
        CrabbyAuthorityCheck c{}; detail::check(crabby_authority_check(&a_, from, epoch, &c)); return static_cast<authority_check>(c);
    }
};

/// "Latest wins per entity" for unreliable state updates (Rust `LatestWins`):
/// accepts an update only if it is newer than all accepted before for that key.
class latest_wins {
    CrabbyLatestWins* raw_ = crabby_latest_wins_new();
public:
    latest_wins() = default;
    ~latest_wins() { crabby_latest_wins_free(raw_); }
    latest_wins(latest_wins&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    latest_wins& operator=(latest_wins&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    latest_wins(const latest_wins&) = delete;
    latest_wins& operator=(const latest_wins&) = delete;
    /// True if the update should be applied, false if it should be dropped.
    bool accept(std::uint64_t key, update_stamp stamp) {
        bool ok = false; detail::check(crabby_latest_wins_accept(raw_, key, stamp, &ok)); return ok;
    }
    /// For streams without epochs.
    bool accept(std::uint64_t key, std::uint16_t seq) { return accept(key, update_stamp{0, seq}); }
    /// The entity is gone: its next update is accepted.
    void forget(std::uint64_t key) { detail::check(crabby_latest_wins_forget(raw_, key)); }
    void clear() { detail::check(crabby_latest_wins_clear(raw_)); }
    std::size_t size() const { return crabby_latest_wins_len(raw_); }
};

/// A position, rotation and optional velocity (Rust `Pose`).
using pose = CrabbyPose;
/// A pose at `position`, not rotated, without a velocity.
inline pose pose_at(vec3 position) { pose p{}; p.position = position; p.rotation = {0, 0, 0, 1}; return p; }
/// The same pose moving at `velocity` (units per second). With a velocity
/// on both sides of a gap, the path is a smooth curve instead of a line.
inline pose moving(pose p, vec3 velocity) { p.velocity = velocity; p.has_velocity = true; return p; }
/// Delay, extrapolation limit and capacity of a snapshot stream.
using snapshot_config = CrabbySnapshotConfig;
/// 0.1 s delay, 0.25 s extrapolation, 64 snapshots.
inline snapshot_config snapshot_defaults() { return crabby_snapshot_config_default(); }
/// What happened to a pushed snapshot (Rust `SnapshotPush`).
enum class snapshot_push : std::uint32_t {
    added = CRABBY_SNAPSHOT_ADDED, duplicate = CRABBY_SNAPSHOT_DUPLICATE,
    too_old = CRABBY_SNAPSHOT_TOO_OLD, invalid = CRABBY_SNAPSHOT_INVALID,
};
/// How a sampled pose was made (Rust `SnapshotState`).
enum class snapshot_state : std::uint32_t {
    interpolated = CRABBY_SNAPSHOT_INTERPOLATED, ///< Between two snapshots.
    extrapolated = CRABBY_SNAPSHOT_EXTRAPOLATED, ///< Predicted past the newest, for a bounded time.
    held = CRABBY_SNAPSHOT_HELD,                 ///< Standing still: before the oldest or past the limit.
};
/// A pose read from a snapshot_stream and how it was made.
struct snapshot_sample { pose value; snapshot_state state; };

/// The snapshots of one remote entity plus an estimate of the sender's clock
/// (Rust `SnapshotStream<Pose>`): push each decoded update with the sender's
/// timestamp and your clock, sample once a frame with your clock. The
/// entity is shown `delay` seconds in the past, interpolated between
/// snapshots; late packets are extrapolated for a bounded time, then held.
class snapshot_stream {
    CrabbySnapshotStream* raw_ = nullptr;
public:
    explicit snapshot_stream(const snapshot_config& config = crabby_snapshot_config_default()) {
        detail::check(crabby_snapshot_stream_new(&config, &raw_));
    }
    ~snapshot_stream() { crabby_snapshot_stream_free(raw_); }
    snapshot_stream(snapshot_stream&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    snapshot_stream& operator=(snapshot_stream&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    snapshot_stream(const snapshot_stream&) = delete;
    snapshot_stream& operator=(const snapshot_stream&) = delete;
    /// Stores a snapshot stamped `remote_time` (sender's clock, seconds) that
    /// arrived at `local_now` (your clock, seconds). Any order, duplicates fine.
    snapshot_push push(double remote_time, double local_now, const pose& p) {
        CrabbySnapshotPush r{}; detail::check(crabby_snapshot_stream_push(raw_, remote_time, local_now, &p, &r));
        return static_cast<snapshot_push>(r);
    }
    /// The pose to show at your time `now`; std::nullopt until the first snapshot.
    std::optional<snapshot_sample> sample(double now) const {
        CrabbyPose p{}; CrabbySnapshotState st{}; bool found = false;
        detail::check(crabby_snapshot_stream_sample(raw_, now, &p, &st, &found));
        if (!found) return std::nullopt;
        return snapshot_sample{p, static_cast<snapshot_state>(st)};
    }
    /// How far in the past to show the entity, in seconds.
    void set_delay(double seconds) { detail::check(crabby_snapshot_stream_set_delay(raw_, seconds)); }
    /// Forgets the snapshots, keeping the clock estimate: a teleport or respawn.
    void clear() { detail::check(crabby_snapshot_stream_clear(raw_)); }
    /// Forgets snapshots and clock estimate: a new connection.
    void reset() { detail::check(crabby_snapshot_stream_reset(raw_)); }
    std::size_t size() const { return crabby_snapshot_stream_len(raw_); }
    /// Your time minus the sender's, once a snapshot arrived.
    std::optional<double> clock_offset() const {
        double o = 0; bool known = false; detail::check(crabby_snapshot_stream_clock_offset(raw_, &o, &known));
        return known ? std::optional<double>(o) : std::nullopt;
    }
};

using interest_tier = CrabbyInterestTier;
/// Rings of interest around each viewer, and how sticky their edges are.
using interest_config = CrabbyInterestConfig;
using interest_viewer = CrabbyInterestViewer;
using interest_entity = CrabbyInterestEntity;
using interest_entry = CrabbyInterestEntry;
using interest_change = CrabbyInterestChange;
/// 40 units every tick, 120 units every 4th tick, hysteresis 5, staggered.
inline interest_config interest_defaults() { return crabby_interest_config_default(); }
/// Which entities each peer should be sent, and how often, by distance
/// (Rust `Interest`): rings around each viewer, a hysteresis at their edges
/// so boundary entities don't flicker, and a spatial grid so thousands of
/// entities are cheap. Call update once per send tick, then read entries per
/// peer for what to send and changes for who needs a spawn or despawn.
class interest {
    CrabbyInterest* raw_ = nullptr;
public:
    explicit interest(const interest_config& config = crabby_interest_config_default()) {
        detail::check(crabby_interest_new(&config, &raw_));
    }
    ~interest() { crabby_interest_free(raw_); }
    interest(interest&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    interest& operator=(interest&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    interest(const interest&) = delete;
    interest& operator=(const interest&) = delete;
    /// Recomputes every peer's interest for send tick `tick`. Peers without a
    /// viewer are forgotten and their entities reported as left.
    void update(std::uint64_t tick, const std::vector<interest_viewer>& viewers, const std::vector<interest_entity>& entities) {
        detail::check(crabby_interest_update(raw_, tick, viewers.data(), viewers.size(), entities.data(), entities.size()));
    }
    /// Everything `peer` is interested in as of the last update; `send` marks
    /// what to send on this tick.
    std::vector<interest_entry> entries(std::uint64_t peer) const {
        std::size_t n = 0; detail::check(crabby_interest_entries(raw_, peer, nullptr, 0, &n));
        std::vector<interest_entry> out(n);
        detail::check(crabby_interest_entries(raw_, peer, out.data(), out.size(), &n));
        return out;
    }
    /// Entities that entered or left a peer's interest in the last update.
    std::vector<interest_change> changes() const {
        std::size_t n = 0; detail::check(crabby_interest_changes(raw_, nullptr, 0, &n));
        std::vector<interest_change> out(n);
        detail::check(crabby_interest_changes(raw_, out.data(), out.size(), &n));
        return out;
    }
    /// A peer disconnected: forget it, without reporting its entities as left.
    void remove_peer(std::uint64_t peer) { detail::check(crabby_interest_remove_peer(raw_, peer)); }
};

using nav_bake_settings = CrabbyNavBakeSettings;
/// A 0.3 m wide, 1.8 m tall agent on 45 degree slopes.
inline nav_bake_settings nav_bake_defaults() { return crabby_nav_bake_settings_default(); }

/// Walkable geometry to bake into a navmesh (ctx::bake_navmesh).
class nav_sources {
    CrabbyNavSources* raw_ = crabby_nav_sources_new();
public:
    nav_sources() = default;
    ~nav_sources() { crabby_nav_sources_free(raw_); }
    nav_sources(nav_sources&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    nav_sources& operator=(nav_sources&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    nav_sources(const nav_sources&) = delete;
    nav_sources& operator=(const nav_sources&) = delete;
    /// World-space triangles: three corners each, counter-clockwise seen from above.
    nav_sources& add_triangles(const std::vector<vec3>& corners) {
        if (corners.size() % 3) throw std::invalid_argument("nav_sources::add_triangles: three corners per triangle");
        detail::check(crabby_nav_sources_add_triangles(raw_, corners.data(), corners.size() / 3));
        return *this;
    }
    /// A mesh's triangles placed by `t`.
    nav_sources& add_mesh(const mesh_data& data, const transform3d& t) {
        detail::check(crabby_nav_sources_add_mesh(raw_, data.raw(), t));
        return *this;
    }
    const CrabbyNavSources* raw() const noexcept { return raw_; }
};

enum class particle_blend : std::uint32_t { alpha = 0, additive = 1 };
enum class particle_space : std::uint32_t { world = 0, local = 1 };
struct particle_key { float t; float value; };
struct particle_color_key { float t; color value; };

/// A particle emitter's settings (Rust `ParticleSettings3D`), built by
/// chaining: `particle_settings().rate(40).cone(0.3f).blend(particle_blend::additive)`.
class particle_settings {
    CrabbyParticleSettings3D c_ = crabby_particle_settings3d_default();
    std::vector<CrabbyParticleKey> size_keys_;
    std::vector<CrabbyParticleColorKey> color_keys_;
    particle_settings& shape(CrabbyParticleShape3D kind, vec3 size) {
        c_.shape = static_cast<std::uint32_t>(kind);
        c_.shape_size = size;
        return *this;
    }
public:
    /// Particles per second; 0 emits only bursts.
    particle_settings& rate(float per_second) { c_.rate = per_second; return *this; }
    /// Particles emitted at once on the first frame.
    particle_settings& burst(std::uint32_t count) { c_.burst = count; return *this; }
    /// Seconds of emission, and whether it starts again after.
    particle_settings& duration(float seconds, bool looping = true) { c_.duration = seconds; c_.looping = looping; return *this; }
    particle_settings& lifetime(float min, float max) { c_.lifetime_min = min; c_.lifetime_max = max; return *this; }
    particle_settings& speed(float min, float max) { c_.speed_min = min; c_.speed_max = max; return *this; }
    particle_settings& size(float min, float max) { c_.size_min = min; c_.size_max = max; return *this; }
    particle_settings& rotation(float min, float max) { c_.rotation_min = min; c_.rotation_max = max; return *this; }
    /// From the origin, along +Y (the default).
    particle_settings& point() { return shape(CRABBY_PARTICLES_POINT, {0, 0, 0}); }
    /// Inside a ball, moving outward.
    particle_settings& sphere(float radius) { return shape(CRABBY_PARTICLES_SPHERE, {radius, 0, 0}); }
    /// From the origin, inside a cone around +Y with this half angle (radians).
    particle_settings& cone(float angle) { return shape(CRABBY_PARTICLES_CONE, {angle, 0, 0}); }
    /// Inside a box, moving along +Y.
    particle_settings& box(vec3 half_extents) { return shape(CRABBY_PARTICLES_BOX, half_extents); }
    particle_settings& gravity(vec3 acceleration) { c_.gravity = acceleration; return *this; }
    /// Velocity damping per second.
    particle_settings& drag(float per_second) { c_.drag = per_second; return *this; }
    /// Size multiplier over normalized lifetime (0 birth, 1 death).
    particle_settings& size_over_life(std::initializer_list<particle_key> keys) {
        size_keys_.clear();
        for (auto k : keys) size_keys_.push_back({k.t, k.value});
        return *this;
    }
    /// Tint over normalized lifetime (0 birth, 1 death).
    particle_settings& color_over_life(std::initializer_list<particle_color_key> keys) {
        color_keys_.clear();
        for (auto k : keys) color_keys_.push_back({k.t, k.value});
        return *this;
    }
    /// The particle image; with `columns` x `rows` sprite sheet frames played over lifetime.
    particle_settings& image(texture t, std::uint32_t columns = 1, std::uint32_t rows = 1) {
        c_.has_texture = true;
        c_.texture = t;
        c_.sheet_columns = columns;
        c_.sheet_rows = rows;
        return *this;
    }
    particle_settings& blend(particle_blend b) { c_.blend = static_cast<std::uint32_t>(b); return *this; }
    /// Most living particles; storage is made up front.
    particle_settings& max_particles(std::uint32_t n) { c_.max_particles = n; return *this; }
    particle_settings& space(particle_space s) { c_.space = static_cast<std::uint32_t>(s); return *this; }
    /// Equal seeds give equal particles.
    particle_settings& seed(std::uint64_t s) { c_.seed = s; return *this; }
    /// The C settings; its key pointers live as long as this object.
    CrabbyParticleSettings3D raw() const {
        CrabbyParticleSettings3D c = c_;
        c.size_keys = size_keys_.data();
        c.size_key_count = size_keys_.size();
        c.color_keys = color_keys_.data();
        c.color_key_count = color_keys_.size();
        return c;
    }
};

/// A ribbon left behind its entity as it moves (Rust `Trail3D`), built by
/// chaining: `trail3d(0.25f, 0.05f).blend(particle_blend::additive)`. Give it
/// to an entity with ctx::set_trail3d.
class trail3d {
    CrabbyTrail3D c_;
    std::vector<CrabbyParticleKey> width_keys_;
    std::vector<CrabbyParticleColorKey> color_keys_;
public:
    /// A ribbon `width` meters wide (narrowing to 0 at its end) whose points last `lifetime` seconds.
    trail3d(float lifetime, float width) : c_(crabby_trail3d_new(lifetime, width)) {}
    /// Width in meters over age (0 the newest end, 1 the oldest).
    trail3d& width_over_age(std::initializer_list<particle_key> keys) {
        width_keys_.clear();
        for (auto k : keys) width_keys_.push_back({k.t, k.value});
        return *this;
    }
    /// Color over age (0 the newest end, 1 the oldest).
    trail3d& color_over_age(std::initializer_list<particle_color_key> keys) {
        color_keys_.clear();
        for (auto k : keys) color_keys_.push_back({k.t, k.value});
        return *this;
    }
    /// The ribbon image: u is each point's age over the lifetime, v across.
    trail3d& image(texture t) { c_.has_texture = true; c_.texture = t; return *this; }
    trail3d& blend(particle_blend b) { c_.blend = static_cast<std::uint32_t>(b); return *this; }
    /// A new point is kept only after the entity moved this far (meters) from the last one.
    trail3d& min_distance(float meters) { c_.min_distance = meters; return *this; }
    /// At most this many points (at least 2); the oldest go first.
    trail3d& max_points(std::uint32_t n) { c_.max_points = n; return *this; }
    /// While false, no new points are added and the ribbon fades out.
    trail3d& emitting(bool on) { c_.emitting = on; return *this; }
    /// The C trail; its key pointers live as long as this object.
    CrabbyTrail3D raw() const {
        CrabbyTrail3D c = c_;
        c.width_keys = width_keys_.data();
        c.width_key_count = width_keys_.size();
        c.color_keys = color_keys_.data();
        c.color_key_count = color_keys_.size();
        return c;
    }
};

/// A line through points in its entity's space, drawn as a camera-facing
/// strip (Rust `Line3D`): beams, cables, ropes. Give it to an entity with
/// ctx::set_line3d, again any time to change it.
class line3d {
    CrabbyLine3D c_;
    std::vector<vec3> points_;
public:
    /// A `c` colored line `width` meters wide through `points` (at least two to draw anything).
    line3d(std::vector<vec3> points, float width, color c) : c_(crabby_line3d_new(width, c)), points_(std::move(points)) {}
    /// The line image: u along it, v across.
    line3d& image(texture t) { c_.has_texture = true; c_.texture = t; return *this; }
    line3d& blend(particle_blend b) { c_.blend = static_cast<std::uint32_t>(b); return *this; }
    /// Meters of line per repeat of the image; 0 stretches it once over the whole line.
    line3d& texture_length(float meters) { c_.texture_length = meters; return *this; }
    /// The C line; its points live as long as this object.
    CrabbyLine3D raw() const {
        CrabbyLine3D c = c_;
        c.points = points_.data();
        c.point_count = points_.size();
        return c;
    }
};

/// The retained UI (Rust `Ui`, reached with `ctx.ui_retained()`): a tree of
/// panels, labels, buttons, images, scroll areas, text inputs, toggles and
/// sliders, laid out by flexbox, grid and absolute positions.
namespace ui {
/// A node's id, never reused: the id of a removed node fails.
using id = std::uint64_t;
/// "No node": a detached root's parent.
constexpr id none = CRABBY_UI_NONE;
/// A flex or grid length: automatic, in points, or a fraction of the parent.
using length = CrabbyUiLength;
inline length automatic() { return {static_cast<std::uint32_t>(CRABBY_UI_LENGTH_AUTO), 0.0f}; }
inline length points(float v) { return {static_cast<std::uint32_t>(CRABBY_UI_LENGTH_POINTS), v}; }
/// A fraction of the parent: `percent(0.5f)` is half.
inline length percent(float fraction) { return {static_cast<std::uint32_t>(CRABBY_UI_LENGTH_PERCENT), fraction}; }
/// One grid row or column.
using track = CrabbyUiTrack;
inline track track_auto() { return {static_cast<std::uint32_t>(CRABBY_UI_TRACK_AUTO), 0.0f}; }
inline track track_points(float v) { return {static_cast<std::uint32_t>(CRABBY_UI_TRACK_POINTS), v}; }
inline track track_percent(float fraction) { return {static_cast<std::uint32_t>(CRABBY_UI_TRACK_PERCENT), fraction}; }
/// A share of the free space, like CSS `fr`.
inline track fr(float share) { return {static_cast<std::uint32_t>(CRABBY_UI_TRACK_FR), share}; }
inline track min_content() { return {static_cast<std::uint32_t>(CRABBY_UI_TRACK_MIN_CONTENT), 0.0f}; }
inline track max_content() { return {static_cast<std::uint32_t>(CRABBY_UI_TRACK_MAX_CONTENT), 0.0f}; }
/// A single template track or an owned, non-nested repeat fragment.
class template_track {
    track single_{};
    std::uint32_t repetition_ = 0, count_ = 0;
    std::vector<track> tracks_;
    std::vector<std::string> names_;
    mutable std::vector<const char*> raw_names_;
public:
    template_track(track t) : single_(t) {}
    template_track(std::uint32_t mode, std::uint32_t count, std::initializer_list<track> tracks)
        : repetition_(mode), count_(count), tracks_(tracks) {}
    /// Names on every line in a repeat fragment, including both edges.
    template_track& names(std::initializer_list<std::string> names) { names_.assign(names); return *this; }
    CrabbyUiTemplateTrack raw() const {
        raw_names_.clear(); for (const auto& n : names_) raw_names_.push_back(n.c_str());
        return {single_, repetition_, count_, tracks_.data(), tracks_.size(), raw_names_.data(), raw_names_.size()};
    }
};
inline template_track repeat(std::uint32_t count, std::initializer_list<track> tracks) { return {1, count, tracks}; }
inline template_track repeat(std::uint32_t count, track t) { return repeat(count, {t}); }
inline template_track auto_fill(std::initializer_list<track> tracks) { return {2, 0, tracks}; }
inline template_track auto_fill(track t) { return auto_fill({t}); }
inline template_track auto_fit(std::initializer_list<track> tracks) { return {3, 0, tracks}; }
inline template_track auto_fit(track t) { return auto_fit({t}); }
/// Where a grid item starts or ends: placed automatically, at a numbered line (from 1; negative counts from the end), or across `n` tracks.
using grid_line = CrabbyUiGridLine;
inline grid_line grid_auto() { return {static_cast<std::uint32_t>(CRABBY_UI_GRID_AUTO), 0}; }
inline grid_line line(int number) { return {static_cast<std::uint32_t>(CRABBY_UI_GRID_LINE), number}; }
inline grid_line span(int tracks) { return {static_cast<std::uint32_t>(CRABBY_UI_GRID_SPAN), tracks}; }

enum class display : std::uint32_t { flex = 0, grid = 1, none = 2 };
/// `flow` keeps a node in normal flow; `relative` also nudges it by its inset and anchors absolute children.
enum class position : std::uint32_t { flow = 0, relative = 1, absolute = 2, fixed = 3 };
enum class direction : std::uint32_t { row = 0, column = 1, row_reverse = 2, column_reverse = 3 };
enum class wrap : std::uint32_t { no_wrap = 0, wrap = 1, wrap_reverse = 2 };
/// Alignment of items on one axis. For `align_self` and `justify_self`, `normal` follows the parent.
enum class align : std::uint32_t { normal = 0, start = 1, end = 2, flex_start = 3, flex_end = 4, center = 5, baseline = 6, stretch = 7 };
/// How lines or items share the free space.
enum class content : std::uint32_t { normal = 0, start = 1, end = 2, flex_start = 3, flex_end = 4, center = 5, stretch = 6, space_between = 7, space_evenly = 8, space_around = 9 };
enum class text_align : std::uint32_t { start = 0, center = 1, end = 2 };

/// A node's layout (Rust `ui_layout::Style`), built by chaining:
/// `ui::layout().column().size(ui::percent(1), ui::points(40)).padding(8).gap(4)`.
class layout {
    CrabbyUiLayout c_ = crabby_ui_layout_default();
    std::vector<template_track> columns_, rows_;
    std::vector<CrabbyUiTrack> auto_columns_, auto_rows_;
    std::vector<std::string> areas_, column_names_, row_names_;
    std::optional<std::string> area_;
    mutable std::vector<CrabbyUiTemplateTrack> raw_columns_, raw_rows_;
    mutable std::vector<const char*> raw_areas_, raw_column_names_, raw_row_names_;
    static void strings(const std::vector<std::string>& from, std::vector<const char*>& to) {
        to.clear(); for (const auto& s : from) to.push_back(s.c_str());
    }
    static CrabbyUiSides sides(length top, length right, length bottom, length left) { return {top, right, bottom, left}; }
public:
    layout() = default;
    /// Starts from a C layout, such as a preset (`ui::row(30)`, `ui::fixed(220, 40)`).
    explicit layout(const CrabbyUiLayout& c) : c_(c) {}
    layout& display(ui::display d) { c_.display = static_cast<std::uint32_t>(d); return *this; }
    layout& position(ui::position p) { c_.position = static_cast<std::uint32_t>(p); return *this; }
    /// Out of flow, placed by `inset` inside the closest relative ancestor.
    layout& absolute() { return position(ui::position::absolute); }
    layout& hidden() { return display(ui::display::none); }
    layout& direction(ui::direction d) { c_.flex_direction = static_cast<std::uint32_t>(d); return *this; }
    layout& row() { return direction(ui::direction::row); }
    layout& column() { return direction(ui::direction::column); }
    layout& wrap(ui::wrap w) { c_.flex_wrap = static_cast<std::uint32_t>(w); return *this; }
    /// Share of the free main-axis space this node takes.
    layout& grow(float g) { c_.flex_grow = g; return *this; }
    /// How much this node shrinks when space is short (default 1; 0 keeps its size).
    layout& shrink(float s) { c_.flex_shrink = s; return *this; }
    layout& basis(length l) { c_.flex_basis = l; return *this; }
    layout& size(length width, length height) { c_.width = width; c_.height = height; return *this; }
    layout& size(float width, float height) { return size(points(width), points(height)); }
    layout& width(length l) { c_.width = l; return *this; }
    layout& height(length l) { c_.height = l; return *this; }
    layout& min_size(length width, length height) { c_.min_width = width; c_.min_height = height; return *this; }
    layout& max_size(length width, length height) { c_.max_width = width; c_.max_height = height; return *this; }
    layout& aspect_ratio(float width_over_height) { c_.aspect_ratio = width_over_height; return *this; }
    /// Outside space; `automatic()` margins absorb free space (to center a node).
    layout& margin(length all) { c_.margin = sides(all, all, all, all); return *this; }
    layout& margin(length top, length right, length bottom, length left) { c_.margin = sides(top, right, bottom, left); return *this; }
    layout& margin(float all) { return margin(points(all)); }
    layout& padding(length all) { c_.padding = sides(all, all, all, all); return *this; }
    layout& padding(length top, length right, length bottom, length left) { c_.padding = sides(top, right, bottom, left); return *this; }
    layout& padding(float all) { return padding(points(all)); }
    /// Offsets from the parent's edges for relative, absolute and fixed nodes.
    layout& inset(length top, length right, length bottom, length left) { c_.inset = sides(top, right, bottom, left); return *this; }
    layout& gap(length columns, length rows) { c_.gap = {columns, rows}; return *this; }
    layout& gap(float both) { return gap(points(both), points(both)); }
    /// Puts its top edge `y` points below its parent's top instead of where
    /// the preset put it (a `centered` dialog near the top); in a row or
    /// column it is a margin above it.
    layout& top(float y) {
        if (c_.position == static_cast<std::uint32_t>(ui::position::absolute)) { c_.inset.top = points(y); c_.margin.top = points(0); }
        else c_.margin.top = points(y);
        return *this;
    }
    /// Puts its left edge `x` points right of its parent's left; see `top`.
    layout& left(float x) {
        if (c_.position == static_cast<std::uint32_t>(ui::position::absolute)) { c_.inset.left = points(x); c_.margin.left = points(0); }
        else c_.margin.left = points(x);
        return *this;
    }
    layout& align_items(align a) { c_.align_items = static_cast<std::uint32_t>(a); return *this; }
    layout& align_self(align a) { c_.align_self = static_cast<std::uint32_t>(a); return *this; }
    layout& justify_items(align a) { c_.justify_items = static_cast<std::uint32_t>(a); return *this; }
    layout& justify_self(align a) { c_.justify_self = static_cast<std::uint32_t>(a); return *this; }
    layout& align_content(content c) { c_.align_content = static_cast<std::uint32_t>(c); return *this; }
    layout& justify_content(content c) { c_.justify_content = static_cast<std::uint32_t>(c); return *this; }
    /// A grid with these columns (rows are added as needed, see `auto_rows`).
    layout& grid(std::initializer_list<template_track> columns) { display(ui::display::grid); columns_.assign(columns); return *this; }
    layout& grid_rows(std::initializer_list<template_track> rows) { rows_.assign(rows); return *this; }
    /// Size of columns beyond the explicit ones; they repeat.
    layout& auto_columns(std::initializer_list<track> columns) { auto_columns_.assign(columns); return *this; }
    /// Size of rows beyond the explicit ones; they repeat.
    layout& auto_rows(std::initializer_list<track> rows) { auto_rows_.assign(rows); return *this; }
    /// For a grid item: where it starts and ends along the columns.
    layout& grid_column(grid_line start, grid_line end = grid_auto()) { c_.grid_column_start = start; c_.grid_column_end = end; return *this; }
    /// For a grid item: where it starts and ends along the rows.
    layout& grid_row(grid_line start, grid_line end = grid_auto()) { c_.grid_row_start = start; c_.grid_row_end = end; return *this; }
    /// CSS-like template rows. Strings are owned by this builder.
    layout& areas(std::initializer_list<std::string> rows) { areas_.assign(rows); return *this; }
    layout& area(std::string name) { area_ = std::move(name); return *this; }
    /// One space-separated set of names per template boundary (empty means unnamed).
    layout& column_names(std::initializer_list<std::string> names) { column_names_.assign(names); return *this; }
    layout& row_names(std::initializer_list<std::string> names) { row_names_.assign(names); return *this; }
    /// The C layout; pointers remain valid until this builder is changed, destroyed, or raw() is called again.
    CrabbyUiLayout raw() const {
        CrabbyUiLayout c = c_;
        raw_columns_.clear(); for (const auto& t : columns_) raw_columns_.push_back(t.raw());
        raw_rows_.clear(); for (const auto& t : rows_) raw_rows_.push_back(t.raw());
        c.grid_template_columns = raw_columns_.data(); c.grid_template_column_count = raw_columns_.size();
        c.grid_template_rows = raw_rows_.data(); c.grid_template_row_count = raw_rows_.size();
        strings(areas_, raw_areas_); strings(column_names_, raw_column_names_); strings(row_names_, raw_row_names_);
        c.grid_template_areas = raw_areas_.data(); c.grid_template_area_count = raw_areas_.size();
        c.grid_column_names = raw_column_names_.data(); c.grid_column_name_count = raw_column_names_.size();
        c.grid_row_names = raw_row_names_.data(); c.grid_row_name_count = raw_row_names_.size();
        c.grid_area = area_ ? area_->c_str() : nullptr;
        c.grid_auto_columns = auto_columns_.data();
        c.grid_auto_column_count = auto_columns_.size();
        c.grid_auto_rows = auto_rows_.data();
        c.grid_auto_row_count = auto_rows_.size();
        return c;
    }
};

/// A node's looks (Rust `UiPaint`), built by chaining:
/// `ui::paint().background(c).border(c2, 2).font_size(24).align(ui::text_align::center)`.
class paint {
    CrabbyUiPaint c_ = crabby_ui_paint_default();
public:
    paint& background(crabby::color c) { c_.background = c; return *this; }
    /// Background while hovered or focused.
    paint& hover(crabby::color c) { c_.hover = c; return *this; }
    /// Background while the pointer is held on it.
    paint& pressed(crabby::color c) { c_.pressed = c; return *this; }
    paint& disabled(crabby::color c) { c_.disabled = c; return *this; }
    /// Border and focus-ring color, and thickness in points.
    paint& border(crabby::color c, float width) { c_.border = c; c_.border_width = width; return *this; }
    /// Border thickness in points, in the default border color.
    paint& border_width(float width) { c_.border_width = width; return *this; }
    /// Text and image tint.
    paint& color(crabby::color c) { c_.color = c; return *this; }
    /// Text font (from `ctx::load_font`) and line height in points.
    paint& font(crabby::font f, float size) { c_.font = f; c_.font_size = size; return *this; }
    paint& font_size(float size) { c_.font_size = size; return *this; }
    /// Break text at word boundaries.
    paint& wrap(bool on = true) { c_.wrap = on; return *this; }
    paint& align(text_align a) { c_.align = static_cast<std::uint32_t>(a); return *this; }
    /// An image or background texture; with `slice` > 0 it is drawn as nine patches with this many pixels of border.
    paint& image(crabby::texture t, float slice = 0.0f) { c_.has_texture = true; c_.texture = t; c_.slice = slice; return *this; }
    CrabbyUiPaint raw() const { return c_; }
};

// Layout presets (Rust `UiLayout`): the common shapes, so no screen needs
// its own helpers. Refine them by chaining: `ui::fill().padding(24).gap(12)`.
/// A full-width row `height` points tall; its children go left to right.
inline layout row(float height) { return layout(crabby_ui_layout_row(height)); }
/// A full-height column `width` points wide; its children stack.
inline layout column(float width) { return layout(crabby_ui_layout_column(width)); }
/// All of the parent (the window for a screen root), shrinking to what its siblings leave.
inline layout fill() { return layout(crabby_ui_layout_fill()); }
/// Exactly `width` x `height` points, never shrunk.
inline layout fixed(float width, float height) { return layout(crabby_ui_layout_fixed(width, height)); }
/// Out of the flow at `x`, `y`, `width` x `height` in fractions of the parent.
inline layout anchored(float x, float y, float width, float height) { return layout(crabby_ui_layout_anchored(x, y, width, height)); }
/// `width` x `height` points in the middle of the parent: a dialog.
inline layout centered(float width, float height) { return layout(crabby_ui_layout_centered(width, height)); }

/// The colors and text defaults of a tree (Rust `UiTheme`): new nodes take
/// their paint from it, and the screen's immediate widgets follow it.
using theme = CrabbyUiTheme;
inline theme default_theme() { return crabby_ui_theme_default(); }

enum class event_kind : std::uint32_t { clicked = 0, changed = 1, submitted = 2, screen_popped = 3 };
/// Something that happened: a button was clicked, a value changed, Enter was
/// pressed in an input (`submitted`), or Cancel popped a screen (`node` is its root).
struct event {
    event_kind kind;
    id node;
};
/// One frame of input for `tree::frame`: `ui::default_input()`, then set what happened.
using input = CrabbyUiInput;
inline input default_input() { return crabby_ui_input_default(); }

/// The UI of a context (Rust `Ctx::ui_retained`); get it with `ctx::ui_retained()`.
/// It is a cheap handle: copy it freely while the system runs.
class tree {
    CrabbyCtx* raw_;
    entity target_;
    static std::optional<id> maybe(id n) { return n == none ? std::nullopt : std::optional<id>(n); }
public:
    /// The screen's UI, or (with `target`) the UI of a world-space panel: `ctx::panel_ui(entity)`.
    explicit tree(CrabbyCtx* raw, entity target = CRABBY_UI_SCREEN) : raw_(raw), target_(target) {}

    // Adding nodes. `parent` is `ui::none` for a detached root (a screen).
    id add_panel(id parent, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_panel(raw_, target_, parent, &c, &n)); return n;
    }
    id add_label(id parent, const char* text, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_label(raw_, target_, parent, text, &c, &n)); return n;
    }
    /// A button; activating it makes a `clicked` event.
    id add_button(id parent, const char* text, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_button(raw_, target_, parent, text, &c, &n)); return n;
    }
    id add_image(id parent, crabby::texture t, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_image(raw_, target_, parent, t, &c, &n)); return n;
    }
    /// A clipped area scrolled by wheel, drag and a bar. Give it a bounded size.
    id add_scroll(id parent, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_scroll(raw_, target_, parent, &c, &n)); return n;
    }
    /// A single-line input; typing makes `changed` events, Enter `submitted`.
    id add_text_input(id parent, const char* text = "", const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_text_input(raw_, target_, parent, text, &c, &n)); return n;
    }
    /// A checkbox; toggling it makes a `changed` event.
    id add_toggle(id parent, const char* text, bool on = false, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_toggle(raw_, target_, parent, text, on, &c, &n)); return n;
    }
    /// A slider from 0 to 1; moving it makes a `changed` event.
    id add_slider(id parent, float value = 0.0f, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_slider(raw_, target_, parent, value, &c, &n)); return n;
    }
    /// A title: a label in the theme's title color and size, which it keeps following.
    id add_title(id parent, const char* text, const layout& l = {}) {
        id n = none; const CrabbyUiLayout c = l.raw();
        detail::check(crabby_ctx_ui_add_title(raw_, target_, parent, text, &c, &n)); return n;
    }
    /// Removes a node and everything under it; their ids stay invalid.
    void remove(id node) { detail::check(crabby_ctx_ui_remove(raw_, target_, node)); }

    // Changing nodes. No setter makes an event.
    void set_layout(id node, const layout& l) { const CrabbyUiLayout c = l.raw(); detail::check(crabby_ctx_ui_set_layout(raw_, target_, node, &c)); }
    void set_grid_template_areas(id node, std::initializer_list<const char*> rows) { detail::check(crabby_ctx_ui_set_grid_template_areas(raw_, target_, node, rows.begin(), rows.size())); }
    void set_grid_line_names(id node, std::uint32_t axis, std::initializer_list<const char*> names) { detail::check(crabby_ctx_ui_set_grid_line_names(raw_, target_, node, axis, names.begin(), names.size())); }
    void set_grid_area(id node, const char* name) { detail::check(crabby_ctx_ui_set_grid_area(raw_, target_, node, name)); }
    void set_grid_named_line(id node, std::uint32_t axis, bool end, const char* name, int occurrence = 0) { detail::check(crabby_ctx_ui_set_grid_named_line(raw_, target_, node, axis, end, name, occurrence)); }
    void set_paint(id node, const paint& p) { const CrabbyUiPaint c = p.raw(); detail::check(crabby_ctx_ui_set_paint(raw_, target_, node, &c)); }
    void set_text(id node, const char* text) { detail::check(crabby_ctx_ui_set_text(raw_, target_, node, text)); }
    std::string text(id node) const {
        std::size_t len = 0;
        detail::check(crabby_ctx_ui_text(raw_, target_, node, nullptr, 0, &len));
        std::string out(len + 1, '\0');
        detail::check(crabby_ctx_ui_text(raw_, target_, node, &out[0], out.size(), &len));
        out.resize(len);
        return out;
    }
    /// The node's paint as drawn: the theme's, plus what was set on it.
    CrabbyUiPaint resolved_paint(id node) const { CrabbyUiPaint p = crabby_ui_paint_default(); detail::check(crabby_ctx_ui_paint(raw_, target_, node, &p)); return p; }
    ui::theme theme() const { ui::theme t = default_theme(); detail::check(crabby_ctx_ui_theme(raw_, target_, &t)); return t; }
    /// Replaces the theme; nodes keep the paint fields set on them.
    void set_theme(const ui::theme& t) { detail::check(crabby_ctx_ui_set_theme(raw_, target_, &t)); }
    /// Names a node, so code that didn't build it finds it: no list of ids to keep.
    /// A name belongs to one node; an empty name takes it off.
    void set_name(id node, const char* name) { detail::check(crabby_ctx_ui_set_name(raw_, target_, node, name)); }
    /// The node with this name, if any.
    std::optional<id> find(const char* name) const { id n = none; detail::check(crabby_ctx_ui_find(raw_, target_, name, &n)); return maybe(n); }
    /// A node's name (empty without one).
    std::string name(id node) const {
        std::size_t len = 0;
        detail::check(crabby_ctx_ui_name(raw_, target_, node, nullptr, 0, &len));
        std::string out(len + 1, '\0');
        detail::check(crabby_ctx_ui_name(raw_, target_, node, &out[0], out.size(), &len));
        out.resize(len);
        return out;
    }
    /// A disabled node and everything under it ignore input.
    void set_enabled(id node, bool on) { detail::check(crabby_ctx_ui_set_enabled(raw_, target_, node, on)); }
    /// A slider's value (0 to 1) or a toggle's (0 off, 1 on).
    void set_value(id node, float value) { detail::check(crabby_ctx_ui_set_value(raw_, target_, node, value)); }
    float value(id node) const { float v = 0; detail::check(crabby_ctx_ui_value(raw_, target_, node, &v)); return v; }
    void set_scroll(id node, vec2 offset) { detail::check(crabby_ctx_ui_set_scroll(raw_, target_, node, offset)); }
    vec2 scroll(id node) const { vec2 v{}; detail::check(crabby_ctx_ui_scroll(raw_, target_, node, &v)); return v; }
    /// A node's rectangle in window points, as of the last layout.
    crabby::rect rect(id node) const { crabby::rect r{}; detail::check(crabby_ctx_ui_rect(raw_, target_, node, &r)); return r; }

    // Screens and focus. Only the top screen takes input; a modal one also owns the pointer outside it.
    void push_screen(id root, bool modal = false) { detail::check(crabby_ctx_ui_push_screen(raw_, target_, root, modal)); }
    /// Pops the top screen; the nodes stay and can be pushed again.
    std::optional<id> pop_screen() { id n = none; detail::check(crabby_ctx_ui_pop_screen(raw_, target_, &n)); return maybe(n); }
    std::optional<id> top_screen() const { id n = none; detail::check(crabby_ctx_ui_top_screen(raw_, target_, &n)); return maybe(n); }
    std::optional<id> focus() const { id n = none; detail::check(crabby_ctx_ui_focus(raw_, target_, &n)); return maybe(n); }
    std::optional<id> hit_test(vec2 point) const { id n = none; detail::check(crabby_ctx_ui_hit_test(raw_, target_, point, &n)); return maybe(n); }
    /// A scale on top of the window's own, from 0.25 to 8.
    void set_scale(float scale) { detail::check(crabby_ctx_ui_set_scale(raw_, target_, scale)); }
    /// The UI owns the pointer (it is over a control or a modal screen is open): skip clicks on the world.
    bool wants_pointer() const { bool b = false; detail::check(crabby_ctx_ui_wants_pointer(raw_, target_, &b)); return b; }
    /// The UI owns the keyboard and gamepad (a screen is open): skip the game's keys. The context's, not a panel's.
    bool wants_keyboard() const { bool b = false; detail::check(crabby_ctx_ui_wants_keyboard(raw_, &b)); return b; }

    // Events. Take them every frame; unread ones expire at the next UI frame.
    std::optional<event> next_event() {
        CrabbyUiEvent e{};
        const CrabbyStatus status = crabby_ctx_ui_next_event(raw_, target_, &e);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return event{static_cast<event_kind>(e.kind), e.id};
    }
    /// Takes the queued events and gives the last click's node, if any: for
    /// screens that react only to clicks (the other events are dropped).
    std::optional<id> take_click() { id n = none; detail::check(crabby_ctx_ui_take_click(raw_, target_, &n)); return maybe(n); }
    /// Takes every queued event.
    std::vector<event> events() {
        std::vector<event> all;
        while (auto e = next_event()) all.push_back(*e);
        return all;
    }

    // Driving the UI by hand (headless tests, custom hosts): the engine already does this every frame.
    void layout_for(vec2 viewport) { detail::check(crabby_ctx_ui_layout(raw_, target_, viewport)); }
    /// Lays out and handles one frame of `in`; this drops unread events.
    void frame(vec2 viewport, const input& in = default_input()) { detail::check(crabby_ctx_ui_frame(raw_, target_, viewport, &in)); }
    /// Typed text for the engine's next UI frame (the context's, not a panel's).
    void insert_text(const char* text) { detail::check(crabby_ctx_ui_insert_text(raw_, text)); }
};
}  // namespace ui

class clip {
    CrabbyClip* raw_ = nullptr;
public:
    explicit clip(CrabbyClip* raw) : raw_(raw) {}
    /// Empty clip for procedural morph animation.
    explicit clip(const char* name) { detail::check(crabby_clip_new(name, &raw_)); }
    /// Scalar keys: interpolation 0 STEP, 1 LINEAR, 2 cubic in/value/out triples.
    void set_morph_track(const char* name, const std::vector<float>& times,
                         const std::vector<float>& values, unsigned interpolation = 1) {
        if (interpolation > 2 || values.size() != times.size() * (interpolation == 2 ? 3 : 1))
            throw std::invalid_argument("clip::set_morph_track: invalid key count or interpolation");
        detail::check(crabby_clip_set_morph_track(raw_, name, times.data(), values.data(), times.size(), interpolation));
    }
    ~clip() { crabby_clip_free(raw_); }
    clip(clip&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    clip& operator=(clip&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    clip(const clip&) = delete;
    clip& operator=(const clip&) = delete;
    /// Its length in seconds.
    float duration() const noexcept { return crabby_clip_duration(raw_); }
    const CrabbyClip* raw() const noexcept { return raw_; }
};

/// A motion for an animation state: a clip, or a 1D/2D blend tree of motions.
/// Handing it to a tree or a state moves it.
class motion {
    CrabbyMotion* raw_ = nullptr;
    explicit motion(CrabbyMotion* raw) : raw_(raw) {}
public:
    ~motion() { crabby_motion_free(raw_); }
    motion(motion&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    motion& operator=(motion&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    motion(const motion&) = delete;
    motion& operator=(const motion&) = delete;
    /// Plays `c` (copied).
    static motion of(const clip& c) { CrabbyMotion* m = nullptr; detail::check(crabby_motion_clip(c.raw(), &m)); return motion(m); }
    /// A 1D blend tree on float parameter `parameter`.
    static motion blend1d(std::size_t parameter) { CrabbyMotion* m = nullptr; detail::check(crabby_motion_blend1d(parameter, &m)); return motion(m); }
    /// A 2D blend tree on float parameters `x` and `y`.
    static motion blend2d(std::size_t x, std::size_t y) { CrabbyMotion* m = nullptr; detail::check(crabby_motion_blend2d(x, y, &m)); return motion(m); }
    /// Adds `child` to a 1D tree at `threshold` (in increasing order).
    motion& add(float threshold, motion child, float time_scale = 1) {
        detail::check(crabby_motion_blend1d_add(raw_, threshold, child.release(), time_scale)); return *this;
    }
    /// Adds `child` to a 2D tree at point (x, y).
    motion& add(float x, float y, motion child, float time_scale = 1) {
        detail::check(crabby_motion_blend2d_add(raw_, x, y, child.release(), time_scale)); return *this;
    }
    CrabbyMotion* release() noexcept { auto r = raw_; raw_ = nullptr; return r; }
};

/// A finished, immutable animation graph, shared by the animators made from it.
enum class layer_blend : std::uint32_t { override_pose = 0, additive = 1 };
enum class root_motion : std::uint32_t { off = 0, extract = 1, apply = 2 };

class animation_graph {
    CrabbyAnimationGraph* raw_ = nullptr;
public:
    explicit animation_graph(CrabbyAnimationGraph* raw) : raw_(raw) {}
    ~animation_graph() { crabby_animation_graph_free(raw_); }
    animation_graph(animation_graph&& o) noexcept : raw_(o.raw_) { o.raw_ = nullptr; }
    animation_graph& operator=(animation_graph&& o) noexcept { std::swap(raw_, o.raw_); return *this; }
    animation_graph(const animation_graph&) = delete;
    animation_graph& operator=(const animation_graph&) = delete;
    const CrabbyAnimationGraph* raw() const noexcept { return raw_; }
};

/// Builds an animation_graph: parameters, then states, then transitions.
/// Indices count from 0 in the order added.
class animation_graph_builder {
    CrabbyAnimationGraphBuilder* raw_ = crabby_animation_graph_builder_new();
public:
    animation_graph_builder() = default;
    ~animation_graph_builder() { crabby_animation_graph_builder_free(raw_); }
    animation_graph_builder(const animation_graph_builder&) = delete;
    animation_graph_builder& operator=(const animation_graph_builder&) = delete;
    std::size_t add_float(float value = 0) { std::size_t i = 0; detail::check(crabby_animation_graph_add_float(raw_, value, &i)); return i; }
    std::size_t add_bool(bool value = false) { std::size_t i = 0; detail::check(crabby_animation_graph_add_bool(raw_, value, &i)); return i; }
    std::size_t add_trigger() { std::size_t i = 0; detail::check(crabby_animation_graph_add_trigger(raw_, &i)); return i; }
    /// A state playing `m`; its speed is also multiplied by `speed_parameter` if given.
    std::size_t add_state(motion m, float speed = 1, std::optional<std::size_t> speed_parameter = std::nullopt, bool looping = true) {
        std::size_t i = 0;
        auto p = speed_parameter ? static_cast<std::int64_t>(*speed_parameter) : -1;
        detail::check(crabby_animation_graph_add_state(raw_, m.release(), speed, p, looping, &i));
        return i;
    }
    /// An event on `state` at normalized time `time` (0 to 1).
    void add_event(std::size_t state, float time, const char* name) { detail::check(crabby_animation_graph_add_event(raw_, state, time, name)); }
    /// A transition from `from` (std::nullopt: any state) to `to`, crossfading over `duration` seconds.
    std::size_t add_transition(std::optional<std::size_t> from, std::size_t to, float duration,
                               std::optional<float> exit_time = std::nullopt, float offset = 0) {
        std::size_t i = 0;
        auto f = from ? static_cast<std::int64_t>(*from) : -1;
        detail::check(crabby_animation_graph_add_transition(raw_, f, to, exit_time ? *exit_time : -1.0f, duration, offset, &i));
        return i;
    }
    /// A condition on `transition`; all must hold.
    void add_condition(std::size_t transition, CrabbyAnimationCondition kind, std::size_t parameter, float value = 0) {
        detail::check(crabby_animation_graph_add_condition(raw_, transition, kind, parameter, value));
    }
    /// Checks and finishes the graph, starting in `initial`; the builder is used up.
    animation_graph build(std::size_t initial = 0) {
        CrabbyAnimationGraph* g = nullptr;
        auto b = raw_;
        raw_ = nullptr;
        detail::check(crabby_animation_graph_build(b, initial, &g));
        return animation_graph(g);
    }
};

/// Inline lookups into a flow field, from any thread (flow_field::view()).
/// Expires when the field is computed again.
class flow_view {
    CrabbyFlowView v_{};
public:
    flow_view() = default;
    explicit flow_view(const CrabbyFlowView& v) : v_(v) {}
    /// Unit direction toward the nearest goal, or zero (goal, wall, outside, unreachable).
    vec2 direction_at(vec2 p) const noexcept {
        if (!v_.cells) return {0, 0};
        const float cx = std::floor((p.x - v_.origin.x) / v_.cell_size), cy = std::floor((p.y - v_.origin.y) / v_.cell_size);
        if (!(cx >= 0 && cy >= 0 && cx < float(v_.width) && cy < float(v_.height))) return {0, 0};
        return v_.directions[v_.cells[(std::size_t(cy) + 1) * v_.stride + std::size_t(cx) + 1]];
    }
};

/// The way to the nearest goal from every cell of a nav_grid (Rust `FlowField`).
class flow_field {
    CrabbyFlowField* raw_ = crabby_flow_new();
public:
    flow_field() = default;
    ~flow_field() { crabby_flow_free(raw_); }
    flow_field(const flow_field&) = delete;
    flow_field& operator=(const flow_field&) = delete;
    /// Recomputes for these goals (world points); reuses its memory.
    void compute(const nav_grid& nav, const std::vector<vec2>& goals) {
        detail::check(crabby_flow_compute(raw_, nav.raw(), goals.data(), goals.size()));
    }
    void compute(const nav_grid& nav, vec2 goal) { detail::check(crabby_flow_compute(raw_, nav.raw(), &goal, 1)); }
    vec2 direction_at(vec2 p) const noexcept { return crabby_flow_direction(raw_, p); }
    /// Length of the shortest way to a goal in world units, or nothing if none can be reached.
    std::optional<float> distance_at(vec2 p) const {
        float d = 0;
        const auto status = crabby_flow_distance(raw_, p, &d);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return d;
    }
    /// Inline lookups for hot loops (query_par), valid until the next compute.
    flow_view view() const { CrabbyFlowView v{}; detail::check(crabby_flow_view(raw_, &v)); return flow_view(v); }
};

/// What a system gets (Rust `Ctx`). Valid only during the call.
/// Events read by ctx::read: a borrowed array, valid until the next send of
/// the type or the end of the system.
template<class T> class events_view {
    const T* data_;
    std::size_t size_;
public:
    events_view(const T* data, std::size_t size) noexcept : data_(data), size_(size) {}
    const T* begin() const noexcept { return data_; }
    const T* end() const noexcept { return data_ + size_; }
    std::size_t size() const noexcept { return size_; }
    bool empty() const noexcept { return size_ == 0; }
    const T& operator[](std::size_t i) const noexcept { return data_[i]; }
};

/// One cell of a cell_streamer's grid: it covers x * cell_size to
/// (x + 1) * cell_size along X, and the same along Z.
using cell = CrabbyCell;

/// Which cells of a square grid on the ground (X and Z) to have loaded
/// around a point, such as the camera (Rust `CellStreamer`): those within
/// load_radius, started nearest first, and kept until they're beyond
/// keep_radius. It only keeps the list; the game loads and drops each cell.
class cell_streamer {
    std::unique_ptr<CrabbyCellStreamer, void (*)(CrabbyCellStreamer*)> raw_;
    float cell_size_;
    static CrabbyCellStreamer* make(float cell_size, float load_radius, float keep_radius) {
        CrabbyCellStreamer* s = crabby_cells_new(cell_size, load_radius, keep_radius);
        if (!s) throw std::invalid_argument(crabby_last_error());
        return s;
    }
    events_view<cell> list(CrabbyStatus (*f)(CrabbyCellStreamer*, const CrabbyCell**, std::size_t*)) const {
        const CrabbyCell* c = nullptr; std::size_t n = 0;
        detail::check(f(raw_.get(), &c, &n));
        return {c, n};
    }
public:
    /// cell_size above 0, radii not negative (keep_radius is raised to load_radius).
    cell_streamer(float cell_size, float load_radius, float keep_radius)
        : raw_(make(cell_size, load_radius, keep_radius), crabby_cells_free), cell_size_(cell_size) {}
    /// Works out the cells to load and drop around `at` (its height is
    /// ignored): at most max_new new ones, nearest first.
    void update(vec3 at, std::size_t max_new) { detail::check(crabby_cells_update(raw_.get(), at, max_new)); }
    /// What the last update started loading, nearest first; valid until the next update.
    events_view<cell> to_load() const { return list(crabby_cells_to_load); }
    /// What the last update dropped; valid until the next update.
    events_view<cell> to_drop() const { return list(crabby_cells_to_drop); }
    bool is_loaded(cell c) const {
        bool loaded = false; detail::check(crabby_cells_is_loaded(raw_.get(), c, &loaded, nullptr)); return loaded;
    }
    std::size_t loaded_count() const {
        bool loaded = false; std::size_t n = 0;
        detail::check(crabby_cells_is_loaded(raw_.get(), cell{0, 0}, &loaded, &n)); return n;
    }
    /// Where a cell starts: its corner with the lowest X and Z, at height 0.
    vec3 corner(cell c) const noexcept { return {float(c.x) * cell_size_, 0.0f, float(c.z) * cell_size_}; }
    /// Forgets every cell, as if none were loaded.
    void clear() { detail::check(crabby_cells_clear(raw_.get())); }
};

/// A terrain tile's heights, for finding the ground under a point on the
/// CPU (Rust `Heightfield`): coordinates from the tile's corner, heights
/// following the drawn triangles.
class heightfield {
    std::unique_ptr<CrabbyHeightfield, void (*)(CrabbyHeightfield*)> raw_;
    static CrabbyHeightfield* make(const std::vector<float>& heights, float size) {
        CrabbyHeightfield* g = crabby_heightfield_new(heights.data(), heights.size(), size);
        if (!g) throw std::invalid_argument(crabby_last_error());
        return g;
    }
public:
    /// n x n heights (row after row along +Z, NaN for a hole), `size` wide.
    heightfield(const std::vector<float>& heights, float size) : raw_(make(heights, size), crabby_heightfield_free) {}
    /// The height at (x, z); nothing off the tile or over a hole.
    std::optional<float> height_at(float x, float z) const {
        bool found = false; float y = 0;
        detail::check(crabby_heightfield_at(raw_.get(), x, z, &found, &y, nullptr));
        if (!found) return std::nullopt;
        return y;
    }
    /// Which way the flat triangle at (x, z) faces; nothing off the tile or over a hole.
    std::optional<vec3> normal_at(float x, float z) const {
        bool found = false; float y = 0; vec3 n{};
        detail::check(crabby_heightfield_at(raw_.get(), x, z, &found, &y, &n));
        if (!found) return std::nullopt;
        return n;
    }
};

/// A terrain tile's meshes at every detail level, built on the CPU (Rust
/// `TerrainTileData`): make it on any thread (std::async), then
/// ctx::upload_terrain_tile on the game's.
class terrain_tile_data {
    std::unique_ptr<CrabbyTerrainTileData, void (*)(CrabbyTerrainTileData*)> raw_;
    static CrabbyTerrainTileData* make(const std::vector<float>& heights, float size) {
        CrabbyTerrainTileData* d = crabby_terrain_tile_data_new(heights.data(), heights.size(), size);
        if (!d) throw std::invalid_argument(crabby_last_error());
        return d;
    }
public:
    /// n x n heights (row after row along +Z, NaN for a hole), `size` wide.
    terrain_tile_data(const std::vector<float>& heights, float size)
        : raw_(make(heights, size), crabby_terrain_tile_data_free) {}
    const CrabbyTerrainTileData* raw() const noexcept { return raw_.get(); }
};

/// A convex hull, triangle mesh or heightfield built on the CPU (Rust
/// `CollisionMeshData`): make it on any thread (std::async), then
/// ctx::add_collision_mesh on the game's, which is quick.
class collision_mesh_data {
    std::unique_ptr<CrabbyCollisionMeshData, void (*)(CrabbyCollisionMeshData*)> raw_;
    explicit collision_mesh_data(CrabbyCollisionMeshData* d) : raw_(d, crabby_collision_mesh_data_free) {
        if (!d) throw std::invalid_argument(crabby_last_error());
    }
public:
    /// A heightfield from n x n heights `size` wide (row after row along +Z,
    /// NaN for a hole), as ctx::heightfield_collider makes it.
    static collision_mesh_data heightfield(const std::vector<float>& heights, float size) {
        return collision_mesh_data(crabby_collision_mesh_data_heightfield(heights.data(), heights.size(), size));
    }
    /// A triangle mesh: three indices into `vertices` per triangle.
    static collision_mesh_data trimesh(const std::vector<vec3>& vertices, const std::vector<std::uint32_t>& indices) {
        return collision_mesh_data(
            crabby_collision_mesh_data_trimesh(vertices.data(), vertices.size(), indices.data(), indices.size()));
    }
    /// A convex hull around `points`.
    static collision_mesh_data convex_hull(const std::vector<vec3>& points) {
        return collision_mesh_data(crabby_collision_mesh_data_convex_hull(points.data(), points.size()));
    }
    const CrabbyCollisionMeshData* raw() const noexcept { return raw_.get(); }
};

/// One physical source for a named action. Configure once or when rebinding.
struct action_binding {
    CrabbyActionBinding raw{};
    static action_binding key(crabby::key k) { return {{CRABBY_ACTION_KEY, 0, static_cast<std::uint32_t>(k), 0, 0, 0, 0}}; }
    static action_binding mouse(crabby::mouse b) { return {{CRABBY_ACTION_MOUSE, 0, static_cast<std::uint32_t>(b), 0, 0, 0, 0}}; }
    static action_binding button(gamepad p, gamepad_button b) { return {{CRABBY_ACTION_GAMEPAD_BUTTON, p.slot, static_cast<std::uint32_t>(b), 0, 0, 0, 0}}; }
    static action_binding keys(crabby::key negative, crabby::key positive) { return {{CRABBY_ACTION_KEYS, 0, static_cast<std::uint32_t>(negative), static_cast<std::uint32_t>(positive), 0, 0, 0}}; }
    static action_binding axis(gamepad p, gamepad_axis a, float deadzone = 0.15f) { return {{CRABBY_ACTION_GAMEPAD_AXIS, p.slot, static_cast<std::uint32_t>(a), 0, 0, 0, deadzone}}; }
    static action_binding keys2(crabby::key left, crabby::key right, crabby::key up, crabby::key down) { return {{CRABBY_ACTION_KEYS2, 0, static_cast<std::uint32_t>(left), static_cast<std::uint32_t>(right), static_cast<std::uint32_t>(up), static_cast<std::uint32_t>(down), 0}}; }
    static action_binding stick(gamepad p, crabby::stick s, float deadzone = 0.15f) { return {{CRABBY_ACTION_GAMEPAD_STICK, p.slot, s == crabby::stick::left ? 0u : 1u, 0, 0, 0, deadzone}}; }
};

/// A button, scalar axis, or vector axis with several alternative sources.
struct action {
    std::uint32_t kind;
    std::vector<CrabbyActionBinding> bindings;
    static action button(std::initializer_list<action_binding> sources) { return make(0, sources); }
    static action axis(std::initializer_list<action_binding> sources) { return make(1, sources); }
    static action axis2(std::initializer_list<action_binding> sources) { return make(2, sources); }
private:
    static action make(std::uint32_t kind, std::initializer_list<action_binding> sources) {
        action a{kind, {}};
        a.bindings.reserve(sources.size());
        for (const auto& s : sources) a.bindings.push_back(s.raw);
        return a;
    }
};

/// Owned, serializable bindings. Install with ctx::set_actions; edit and reinstall to rebind.
class action_map {
    CrabbyActionMap* raw_ = nullptr;
public:
    action_map() : raw_(crabby_action_map_new()) { if (!raw_) throw std::runtime_error(crabby_last_error()); }
    ~action_map() { crabby_action_map_free(raw_); }
    action_map(const action_map&) = delete;
    action_map& operator=(const action_map&) = delete;
    action_map(action_map&& other) noexcept : raw_(std::exchange(other.raw_, nullptr)) {}
    action_map& operator=(action_map&& other) noexcept { if (this != &other) { crabby_action_map_free(raw_); raw_ = std::exchange(other.raw_, nullptr); } return *this; }
    void set(const char* name, const action& a) { detail::check(crabby_action_map_set(raw_, name, a.kind, a.bindings.data(), a.bindings.size())); }
    void remove(const char* name) { detail::check(crabby_action_map_remove(raw_, name)); }
    std::string to_json() const {
        std::size_t len = 0;
        detail::check(crabby_action_map_save(raw_, nullptr, 0, &len));
        std::string json(len + 1, '\0');
        detail::check(crabby_action_map_save(raw_, &json[0], json.size(), &len));
        json.resize(len); return json;
    }
    static action_map from_json(const std::string& json) {
        action_map result;
        detail::check(crabby_action_map_load(result.raw_, json.c_str()));
        return result;
    }
    CrabbyActionMap* raw() const noexcept { return raw_; }
};

/// What a system declares it reads and writes: components, resources and
/// events by C++ type (their registered names) or by name, commands and
/// services. Pass it to game::add with a system; debug builds stop the game
/// if the system uses anything it didn't declare. Copyable. Build it at
/// registration, not per frame: every step copies the declaration and keeps
/// each name for the rest of the program.
class access {
    CrabbyAccess* raw_ = nullptr;
    access& step(CrabbyStatus s) { detail::check(s); return *this; }
public:
    access() : raw_(crabby_access_new()) { if (!raw_) throw std::runtime_error(crabby_last_error()); }
    ~access() { crabby_access_free(raw_); }
    access(const access& other) : access() { *this = other; }
    access& operator=(const access& other) {
        if (this != &other) { access fresh; std::swap(raw_, fresh.raw_); names_.clear(); copy_from(other); }
        return *this;
    }
    access(access&& other) noexcept : raw_(std::exchange(other.raw_, nullptr)) {}
    access& operator=(access&& other) noexcept { if (this != &other) { crabby_access_free(raw_); raw_ = std::exchange(other.raw_, nullptr); } return *this; }
    /// Reads component T.
    template<class T> access& read() { return read_component(detail::type_name<T>().c_str()); }
    /// Reads and writes component T.
    template<class T> access& write() { return write_component(detail::type_name<T>().c_str()); }
    /// Reads resource T.
    template<class T> access& read_resource() { return read_resource(detail::type_name<T>().c_str()); }
    /// Reads and writes resource T.
    template<class T> access& write_resource() { return write_resource(detail::type_name<T>().c_str()); }
    /// Reads T events.
    template<class T> access& read_events() { return read_events(detail::type_name<T>().c_str()); }
    /// Sends T events (doesn't cover reading them).
    template<class T> access& send_events() { return send_events(detail::type_name<T>().c_str()); }
    access& read_component(const char* name) { names_.push_back({0, name}); return step(crabby_access_read_component(raw_, name)); }
    access& write_component(const char* name) { names_.push_back({1, name}); return step(crabby_access_write_component(raw_, name)); }
    access& read_resource(const char* name) { names_.push_back({2, name}); return step(crabby_access_read_resource(raw_, name)); }
    access& write_resource(const char* name) { names_.push_back({3, name}); return step(crabby_access_write_resource(raw_, name)); }
    access& read_events(const char* name) { names_.push_back({4, name}); return step(crabby_access_read_events(raw_, name)); }
    access& send_events(const char* name) { names_.push_back({5, name}); return step(crabby_access_send_events(raw_, name)); }
    /// Records commands (ctx::*_later).
    access& commands() { names_.push_back({6, {}}); return step(crabby_access_commands(raw_)); }
    /// Uses `s`: reading it, or also changing it.
    access& read_service(service s) { names_.push_back({7, {}, static_cast<std::uint32_t>(s)}); return step(crabby_access_service(raw_, static_cast<std::uint32_t>(s), false)); }
    access& write_service(service s) { names_.push_back({8, {}, static_cast<std::uint32_t>(s)}); return step(crabby_access_service(raw_, static_cast<std::uint32_t>(s), true)); }
    /// Number of reasons this system, registered before `later`, can't share
    /// a parallel batch with it (0: it could). `ordered = false` checks
    /// either order.
    std::size_t conflicts_before(const access& later, bool ordered = true) const {
        std::size_t n = 0;
        detail::check(crabby_access_conflicts(raw_, later.raw_, ordered, &n));
        return n;
    }
    const CrabbyAccess* raw() const noexcept { return raw_; }
private:
    struct entry { int kind; std::string name; std::uint32_t service = 0; };
    std::vector<entry> names_;
    void copy_from(const access& other) {
        for (const auto& e : other.names_) {
            switch (e.kind) {
            case 0: read_component(e.name.c_str()); break;
            case 1: write_component(e.name.c_str()); break;
            case 2: read_resource(e.name.c_str()); break;
            case 3: write_resource(e.name.c_str()); break;
            case 4: read_events(e.name.c_str()); break;
            case 5: send_events(e.name.c_str()); break;
            case 6: commands(); break;
            case 7: read_service(static_cast<service>(e.service)); break;
            default: write_service(static_cast<service>(e.service)); break;
            }
        }
    }
};

/// Manual input for tests and replay (same state as live input).
struct input_event {
    std::uint32_t kind, slot, input;
    float value;
    static input_event key(crabby::key k, bool down) { return {0, 0, static_cast<std::uint32_t>(k), down ? 1.0f : 0.0f}; }
    static input_event mouse(crabby::mouse b, bool down) { return {1, 0, static_cast<std::uint32_t>(b), down ? 1.0f : 0.0f}; }
    static input_event button(gamepad p, gamepad_button b, bool down) { return {2, p.slot, static_cast<std::uint32_t>(b), down ? 1.0f : 0.0f}; }
    static input_event axis(gamepad p, gamepad_axis a, float value) { return {3, p.slot, static_cast<std::uint32_t>(a), value}; }
    static input_event focus_lost() { return {4, 0, 0, 0}; }
    static input_event disconnected(gamepad p) { return {5, p.slot, 0, 0}; }
};

/// One contact point of a pair: where, and how far apart the bodies are there.
struct contact_point3d {
    vec3 position;
    float distance;
};
/// The contacts between two bodies in one step, as a contact hook sees them
/// (Rust `ContactPair3D`). Only valid inside the hook call; changes last for
/// this step. Calls throw on bad arguments (a non-finite value).
class contact_pair3d {
public:
    explicit contact_pair3d(CrabbyContactPair3D* raw) noexcept : raw_(raw) {}
    contact_pair3d(const contact_pair3d&) = delete;
    contact_pair3d& operator=(const contact_pair3d&) = delete;
    /// The two bodies (Rust `a` and `b`).
    entity a() const { return both().first; }
    entity b() const { return both().second; }
    /// The contact direction in the world, from `a` toward `b`.
    vec3 normal() const { vec3 n{}; detail::check(crabby_contact_pair3d_normal(raw_, &n)); return n; }
    /// How many contact points there are (0 once ignored).
    std::size_t point_count() const { std::size_t n = 0; detail::check(crabby_contact_pair3d_point_count(raw_, &n)); return n; }
    /// Point `i`, or nothing if there is no such point.
    std::optional<contact_point3d> point(std::size_t i) const {
        contact_point3d p{};
        auto status = crabby_contact_pair3d_point(raw_, i, &p.position, &p.distance);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return p;
    }
    /// Drops every contact: the bodies pass through each other this step.
    void ignore() { detail::check(crabby_contact_pair3d_ignore(raw_)); }
    /// Drops point `i` (later points may change places); past the end does nothing.
    void remove_point(std::size_t i) { detail::check(crabby_contact_pair3d_remove_point(raw_, i)); }
    float friction() const { float f = 0; detail::check(crabby_contact_pair3d_friction(raw_, &f)); return f; }
    void set_friction(float friction) { detail::check(crabby_contact_pair3d_set_friction(raw_, friction)); }
    float restitution() const { float r = 0; detail::check(crabby_contact_pair3d_restitution(raw_, &r)); return r; }
    void set_restitution(float restitution) { detail::check(crabby_contact_pair3d_set_restitution(raw_, restitution)); }
    /// Makes `b` slide along `a`'s surface at `velocity` (world, m/s).
    void set_surface_velocity(vec3 velocity) { detail::check(crabby_contact_pair3d_set_surface_velocity(raw_, velocity)); }
    CrabbyContactPair3D* raw() const noexcept { return raw_; }
private:
    std::pair<entity, entity> both() const {
        entity a = 0, b = 0;
        detail::check(crabby_contact_pair3d_entities(raw_, &a, &b));
        return {a, b};
    }
    CrabbyContactPair3D* raw_;
};
class ctx {
    CrabbyCtx* raw_;
    std::shared_ptr<detail::identity> owner_;
    template<class T> std::uint32_t id() const { return world_view::id_of<T>(crabby_ctx_world(raw_), *owner_); }
    template<class T> std::optional<std::uint32_t> find_id() const { return world_view::find_id<T>(crabby_ctx_world(raw_), *owner_); }
    /// Runs `fn` right after the system returns (see detail::identity::later).
    void after_system(std::function<void(ctx&)> fn) {
        std::lock_guard<std::mutex> guard(owner_->later_lock);
        owner_->later.push_back(std::move(fn));
    }
    /// Calls the hook from ctx::set_contact_hook3d. Nothing may unwind into
    /// the engine: an exception is kept for the game to rethrow.
    static void contact_trampoline(CrabbyContactPair3D* raw, void* user) noexcept {
        auto& owner = *static_cast<detail::identity*>(user);
        if (owner.hook_error.set.load() || !owner.contact_hook) return;
        try { contact_pair3d pair(raw); owner.contact_hook(pair); }
        catch (...) { owner.hook_error.capture(); }
    }
    friend class game;
    /// Replays what after_system queued, now that no query holds the world.
    void run_after_system() {
        for (;;) {
            std::vector<std::function<void(ctx&)>> todo;
            { std::lock_guard<std::mutex> guard(owner_->later_lock); todo.swap(owner_->later); }
            if (todo.empty()) return;
            for (auto& fn : todo) fn(*this);
        }
    }
public:
    ctx(CrabbyCtx* raw, std::shared_ptr<detail::identity> owner) : raw_(raw), owner_(std::move(owner)) {}
    CrabbyCtx* raw() const noexcept { return raw_; }
    world_view world() const { return world_view(crabby_ctx_world(raw_), owner_); }

    // Clock (safe from query_par callbacks).
    float dt() const noexcept { return crabby_ctx_dt(raw_); }
    std::uint64_t sim_tick() const noexcept { return crabby_ctx_sim_tick(raw_); }
    double sim_time() const noexcept { return crabby_ctx_sim_time(raw_); }
    double time() const noexcept { return crabby_ctx_time(raw_); }
    std::uint64_t frame() const noexcept { return crabby_ctx_frame(raw_); }
    float fps() const noexcept { return crabby_ctx_fps(raw_); }
    /// fps, frame-time percentiles, CPU and GPU time over the last few seconds.
    crabby::frame_stats frame_stats() const { crabby::frame_stats s{}; detail::check(crabby_ctx_frame_stats(raw_, &s)); return s; }
    /// What the renderer uploaded to the GPU over the last drawn frame (all zero headless).
    crabby::gpu_uploads gpu_uploads() const { crabby::gpu_uploads u{}; detail::check(crabby_ctx_gpu_uploads(raw_, &u)); return u; }
    /// Shows or hides the on-screen performance overlay.
    void set_perf_hud(bool on) { detail::check(crabby_ctx_set_perf_hud(raw_, on)); }
    /// Turns on scope timing (CRABBY_PROFILE_SCOPE) and allocation counting;
    /// the overlay and CRABBY_BENCH do it by themselves. Off, a scope costs one atomic load.
    void set_profiling(bool on) { detail::check(crabby_ctx_set_profiling(raw_, on)); }
    /// Where time and allocations went over the last frames (needs set_profiling(true) for a while).
    crabby::perf_report perf_report() const {
        return crabby::perf_report(detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_ctx_perf_report(raw_, b, c, n); }));
    }
    /// Records the next `frames` frames as a Chrome/Perfetto trace at `path`.
    void record_trace(const char* path, std::uint32_t frames) { detail::check(crabby_ctx_record_trace(raw_, path, frames)); }
    vec2 window_size() const noexcept { return crabby_ctx_window_size(raw_); }
    crabby::random random() const noexcept { return crabby::random(crabby_ctx_random_bits(raw_)); }

    // Simulation controls.
    void quit() { detail::check(crabby_ctx_quit(raw_)); }

    // Lockstep networking (game::lockstep_host / lockstep_join); offline: one player.
    std::uint32_t player() const { std::uint32_t p = 0, n = 0; detail::check(crabby_ctx_net_player(raw_, &p, &n)); return p; }
    std::uint32_t players() const { std::uint32_t p = 0, n = 0; detail::check(crabby_ctx_net_player(raw_, &p, &n)); return n; }
    CrabbyNetState net_state() const { std::uint32_t s = 0; detail::check(crabby_ctx_net_state(raw_, &s)); return static_cast<CrabbyNetState>(s); }
    /// Whether the players' worlds stopped matching (details on stderr).
    bool desynced() const {
        bool found = false; std::uint64_t tick = 0; std::uint32_t who = 0;
        detail::check(crabby_ctx_net_desync(raw_, &found, &tick, &who));
        return found;
    }
    /// This player's input for the next tick it can go into (a plain struct
    /// of at most 1024 bytes). The last call before that tick wins.
    template<class T> void set_input(const T& input) {
        static_assert(detail::pod<T> && sizeof(T) <= 1024, "inputs are plain structs of at most 1024 bytes");
        detail::check(crabby_ctx_set_input(raw_, &input, sizeof(T)));
    }
    /// `player`'s input for this tick, if they sent one.
    template<class T> std::optional<T> input(std::uint32_t player) const {
        static_assert(detail::pod<T>, "inputs are plain structs");
        const void* data = nullptr;
        std::size_t size = 0;
        detail::check(crabby_ctx_input(raw_, player, &data, &size));
        if (size != sizeof(T)) return std::nullopt;
        T value;
        std::memcpy(&value, data, sizeof(T));
        return value;
    }
    /// The Steam lobby of this session, or 0 (not created yet, or not Steam).
    std::uint64_t steam_lobby() const { std::uint64_t l = 0; detail::check(crabby_ctx_steam_lobby(raw_, &l)); return l; }
    /// Opens the Steam overlay to invite friends to this session's lobby.
    void steam_invite() { detail::check(crabby_ctx_steam_invite(raw_)); }
    void pause() { detail::check(crabby_ctx_pause(raw_)); }
    void resume() { detail::check(crabby_ctx_resume(raw_)); }
    void step() { detail::check(crabby_ctx_step(raw_)); }
    void set_sim_speed(float speed) { detail::check(crabby_ctx_set_sim_speed(raw_, speed)); }
    float sim_speed() const { float s = 0; detail::check(crabby_ctx_sim_speed(raw_, &s)); return s; }
    bool is_paused() const { bool p = false; detail::check(crabby_ctx_is_paused(raw_, &p)); return p; }

    // Input.
    /// Install a copy of bindings. Rebinding itself generates no button edges.
    void set_actions(const action_map& map) { detail::check(crabby_ctx_set_actions(raw_, map.raw())); }
    action_map actions() const { action_map map; detail::check(crabby_ctx_actions(raw_, map.raw())); return map; }
    void feed_input(input_event e) { detail::check(crabby_ctx_feed_input(raw_, e.kind, e.slot, e.input, e.value)); }
    bool action_down(const char* name) const { bool value = false; detail::check(crabby_ctx_action_down(raw_, name, &value)); return value; }
    bool action_pressed(const char* name) const { bool value = false; detail::check(crabby_ctx_action_pressed(raw_, name, &value)); return value; }
    bool action_released(const char* name) const { bool value = false; detail::check(crabby_ctx_action_released(raw_, name, &value)); return value; }
    float action_axis(const char* name) const { float value = 0; detail::check(crabby_ctx_action_axis(raw_, name, &value)); return value; }
    vec2 action_axis2(const char* name) const { vec2 value{}; detail::check(crabby_ctx_action_axis2(raw_, name, &value)); return value; }
    bool key_down(key k) const { bool b = false; detail::check(crabby_ctx_key_down(raw_, k, &b)); return b; }
    bool key_pressed(key k) const { bool b = false; detail::check(crabby_ctx_key_pressed(raw_, k, &b)); return b; }
    bool key_released(key k) const { bool b = false; detail::check(crabby_ctx_key_released(raw_, k, &b)); return b; }
    bool mouse_down(mouse m) const { bool b = false; detail::check(crabby_ctx_mouse_down(raw_, m, &b)); return b; }
    bool mouse_pressed(mouse m) const { bool b = false; detail::check(crabby_ctx_mouse_pressed(raw_, m, &b)); return b; }
    bool mouse_released(mouse m) const { bool b = false; detail::check(crabby_ctx_mouse_released(raw_, m, &b)); return b; }
    vec2 mouse_pos() const { vec2 v{}; detail::check(crabby_ctx_mouse_pos(raw_, &v)); return v; }
    vec2 mouse_world() const { vec2 v{}; detail::check(crabby_ctx_mouse_world(raw_, &v)); return v; }
    vec2 mouse_wheel() const { vec2 v{}; detail::check(crabby_ctx_mouse_wheel(raw_, &v)); return v; }
    /// How far the mouse moved this frame in raw device units (+y down),
    /// even while the cursor is locked: for mouse look.
    vec2 mouse_motion() const { vec2 v{}; detail::check(crabby_ctx_mouse_motion(raw_, &v)); return v; }
    /// Hides the cursor and holds it in the window (or lets it go), after this frame.
    void set_cursor_locked(bool locked) { detail::check(crabby_ctx_set_cursor_locked(raw_, locked)); }
    bool cursor_locked() const { bool b = false; detail::check(crabby_ctx_cursor_locked(raw_, &b)); return b; }
    /// The connected gamepads: `for (crabby::gamepad pad : ctx.gamepads())`.
    gamepad_list gamepads() const {
        gamepad_list list;
        detail::check(crabby_ctx_gamepad_count(raw_, &list.count));
        list.count = std::min<std::uint32_t>(list.count, CRABBY_MAX_GAMEPADS);
        for (std::uint32_t i = 0; i < list.count; ++i) detail::check(crabby_ctx_gamepad_at(raw_, i, &list.pads[i].slot));
        return list;
    }
    bool gamepad_down(gamepad p, gamepad_button b) const { bool v = false; detail::check(crabby_ctx_gamepad_down(raw_, p.slot, b, &v)); return v; }
    bool gamepad_pressed(gamepad p, gamepad_button b) const { bool v = false; detail::check(crabby_ctx_gamepad_pressed(raw_, p.slot, b, &v)); return v; }
    bool gamepad_released(gamepad p, gamepad_button b) const { bool v = false; detail::check(crabby_ctx_gamepad_released(raw_, p.slot, b, &v)); return v; }
    /// Sticks -1..1 (y down), triggers 0..1.
    float gamepad_axis(gamepad p, gamepad_axis a) const { float v = 0; detail::check(crabby_ctx_gamepad_axis(raw_, p.slot, a, &v)); return v; }
    /// A stick as a vector, y down: add it to a position to move that way.
    vec2 gamepad_stick(gamepad p, stick s) const {
        bool left = s == stick::left;
        return {gamepad_axis(p, left ? CRABBY_GAMEPAD_LEFT_STICK_X : CRABBY_GAMEPAD_RIGHT_STICK_X),
                gamepad_axis(p, left ? CRABBY_GAMEPAD_LEFT_STICK_Y : CRABBY_GAMEPAD_RIGHT_STICK_Y)};
    }

    // Textures and drawing.
    /// Whether `path` (as loaded) was reloaded at the start of this frame.
    bool was_reloaded(const char* path) const { bool b = false; detail::check(crabby_ctx_was_reloaded(raw_, path, &b)); return b; }
    /// Loads a JSON file from assets/ into resource `T` (described with
    /// world().reflect_resource; fields merge, and it's added if missing).
    /// With hot reload on, edits to the file show up live.
    template<class T> void load_json(const char* path) {
        detail::check(crabby_ctx_load_resource_json(raw_, path, detail::type_name<T>().c_str()));
    }
    /// Loads a JSON file from assets/ into reflected component `T` of `e`
    /// (fields merge); with hot reload on, edits to the file show up live.
    template<class T> void load_json(const char* path, entity e) {
        detail::check(crabby_ctx_load_json(raw_, path, e, id<T>()));
    }
    // Scenes and prefabs: worlds and entities kept as JSON under assets/.
    /// Replaces the world with the scene in assets/`path`; ids are kept.
    void load_scene(const char* path) { detail::check(crabby_ctx_load_scene(raw_, path)); }
    /// load_scene, named for a lockstep game: call it from a sim system or
    /// setup, which every peer runs. Both throw from a frame system in a session.
    void load_scene_lockstep(const char* path) { detail::check(crabby_ctx_load_scene_lockstep(raw_, path)); }
    /// Saves the world as a scene in assets/`path`.
    void save_scene(const char* path) { detail::check(crabby_ctx_save_scene(raw_, path)); }
    /// Spawns the prefab in assets/`path` with its position at `at`.
    entity spawn_prefab(const char* path, vec2 at) { entity e = 0; detail::check(crabby_ctx_spawn_prefab(raw_, path, at, &e)); return e; }
    /// Repositions a 3D prefab while preserving saved rotation and scale.
    entity spawn_prefab3d(const char* path, vec3 at) { entity e = 0; detail::check(crabby_ctx_spawn_prefab3d(raw_, path, at, &e)); return e; }
    /// Places a prefab with a complete transform. Separate names preserve old brace-init calls.
    entity spawn_prefab_transform3d(const char* path, transform3d at) { entity e = 0; detail::check(crabby_ctx_spawn_prefab_transform3d(raw_, path, at, &e)); return e; }
    entity spawn_prefab_transform2d(const char* path, transform2d at) { entity e = 0; detail::check(crabby_ctx_spawn_prefab_transform2d(raw_, path, at, &e)); return e; }
    /// Saves `e` as a prefab in assets/`path` (without its position).
    void save_prefab(entity e, const char* path) { detail::check(crabby_ctx_save_prefab(raw_, e, path)); }
    texture load_texture(const char* path) { texture t{}; detail::check(crabby_ctx_load_texture(raw_, path, &t)); return t; }
    texture create_texture(std::uint32_t w, std::uint32_t h, const std::uint8_t* rgba, std::size_t len) {
        texture t{}; detail::check(crabby_ctx_create_texture(raw_, w, h, rgba, len, &t)); return t;
    }
    void update_texture(texture t, const std::uint8_t* rgba, std::size_t len) { detail::check(crabby_ctx_update_texture(raw_, t, rgba, len)); }
    void clear(color c) { detail::check(crabby_ctx_clear(raw_, c)); }
    void draw(texture t, vec2 pos) { detail::check(crabby_ctx_draw(raw_, t, pos)); }
    void draw(texture t, vec2 pos, const draw_params& params) { detail::check(crabby_ctx_draw_ex(raw_, t, pos, &params)); }
    void draw_text(const char* text, vec2 pos, float size, color c) { detail::check(crabby_ctx_draw_text(raw_, text, pos, size, c)); }
    vec2 measure_text(const char* text, float size) { vec2 v{}; detail::check(crabby_ctx_measure_text(raw_, text, size, &v)); return v; }
    crabby::camera camera() const { crabby::camera c{}; detail::check(crabby_ctx_camera(raw_, &c)); return c; }
    void set_camera(const crabby::camera& c) { detail::check(crabby_ctx_set_camera(raw_, &c)); }

    // 3D: meshes, materials, the camera and the sun.
    /// Puts a material on the GPU (roughness and metallic 0 to 1).
    material create_material(const material_desc& d) { material m{}; detail::check(crabby_ctx_create_material(raw_, &d, &m)); return m; }
    /// Changes a material: every model drawn with it changes too.
    void update_material(material m, const material_desc& d) { detail::check(crabby_ctx_update_material(raw_, m, &d)); }
    /// The materials loaded from glTF files so far that the files call
    /// `name`, with what each was made from: to give them a shader of your
    /// own with update_material.
    std::vector<std::pair<material, material_desc>> loaded_materials(const char* name) {
        std::size_t n = 0;
        detail::check(crabby_ctx_loaded_materials(raw_, name, nullptr, nullptr, 0, &n));
        std::vector<material> ms(n);
        std::vector<material_desc> ds(n);
        detail::check(crabby_ctx_loaded_materials(raw_, name, ms.data(), ds.data(), n, &n));
        std::vector<std::pair<material, material_desc>> out;
        for (std::size_t i = 0; i < ms.size() && i < n; i++) out.emplace_back(ms[i], ds[i]);
        return out;
    }
    /// Loads a material shader (WGSL) from assets/: it may define
    /// `fn vertex(v: Vertex) -> Vertex` and `fn surface(s: Surface) -> Surface`.
    /// Hot reloaded; throws if it doesn't compile (the message says where).
    /// What happened to each of the game's shaders the last time it was
    /// compiled (oldest first): whether an edit took effect, and where an
    /// error is in its own WGSL.
    std::vector<crabby::shader_status> shader_status() const {
        std::size_t count = 0;
        detail::check(crabby_ctx_shader_status_count(raw_, &count));
        std::vector<crabby::shader_status> list(count);
        for (std::size_t i = 0; i < count; ++i) {
            detail::check(crabby_ctx_shader_status(raw_, i, &list[i].info));
            list[i].shader = detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_ctx_shader_status_name(raw_, i, b, c, n); });
            list[i].errors = detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_ctx_shader_status_text(raw_, i, b, c, n); });
        }
        return list;
    }
    /// shader_status() as JSON, as the `shaders.status` control method returns it.
    std::string shader_status_json() const {
        return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_ctx_shader_status_json(raw_, b, c, n); });
    }
    /// What this running game can do, as JSON: the control endpoint's
    /// `capabilities` answer (features, mode, subsystems, methods, and every
    /// reflected type with its described schema). Reading it changes nothing.
    std::string capabilities_json() const {
        return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_ctx_capabilities_json(raw_, b, c, n); });
    }
    crabby::shader load_shader(const char* path) { crabby::shader x{}; detail::check(crabby_ctx_load_shader(raw_, path, &x)); return x; }
    /// Makes a material shader from WGSL source; `name` is for error messages.
    crabby::shader create_shader(const char* name, const char* source) {
        crabby::shader x{}; detail::check(crabby_ctx_create_shader(raw_, name, source, &x)); return x;
    }
    /// Loads a screen effect (WGSL defining `fn effect(p: Pixel) -> vec3<f32>`)
    /// from assets/, hot reloaded; the same file again gives the same effect.
    crabby::effect load_effect(const char* path) { crabby::effect e{}; detail::check(crabby_ctx_load_effect(raw_, path, &e)); return e; }
    /// A screen effect from WGSL source; `name` is for error messages.
    crabby::effect create_effect(const char* name, const char* source) {
        crabby::effect e{}; detail::check(crabby_ctx_create_effect(raw_, name, source, &e)); return e;
    }
    /// The screen effects run over the lit 3D frame each frame, in order.
    std::vector<screen_effect> effects() const {
        std::size_t n = 0;
        detail::check(crabby_ctx_effects(raw_, nullptr, 0, &n));
        std::vector<screen_effect> list(n);
        detail::check(crabby_ctx_effects(raw_, list.data(), list.size(), &n));
        return list;
    }
    /// Replaces the screen effects run each frame (empty: none).
    void set_effects(const std::vector<screen_effect>& list) { detail::check(crabby_ctx_set_effects(raw_, list.data(), list.size())); }
    /// Zeroed or initialized GPU vec4 storage, updated without CPU particle work.
    instance_batch create_instance_batch(mesh m, const std::vector<std::array<float,16>>& matrices,
        instance_batch_options options = {32.0f, std::numeric_limits<float>::infinity(), true, colors::white}) {
        instance_batch b{};
        detail::check(crabby_ctx_create_instance_batch(raw_,m,reinterpret_cast<const float (*)[16]>(matrices.data()),matrices.size(),options,&b));
        return b;
    }
    void update_instance_batch(instance_batch b, const std::vector<std::array<float,16>>& matrices) {
        detail::check(crabby_ctx_update_instance_batch(raw_,b,reinterpret_cast<const float (*)[16]>(matrices.data()),matrices.size()));
    }
    instance_batch create_instance_batch_levels(const std::vector<instance_level>& levels,
        const std::vector<std::array<float,16>>& matrices, const std::vector<color>& colors = {},
        instance_batch_options options = {32.0f, std::numeric_limits<float>::infinity(), true, colors::white}) {
        instance_batch b{};
        detail::check(crabby_ctx_create_instance_batch_levels(raw_, levels.data(), levels.size(),
            reinterpret_cast<const float (*)[16]>(matrices.data()), matrices.size(), colors.data(), colors.size(), options, &b));
        return b;
    }
    void update_instance_batch_colors(instance_batch b, const std::vector<std::array<float,16>>& matrices,
        const std::vector<color>& colors = {}) {
        detail::check(crabby_ctx_update_instance_batch_colors(raw_, b,
            reinterpret_cast<const float (*)[16]>(matrices.data()), matrices.size(), colors.data(), colors.size()));
    }
    void free_instance_batch(instance_batch b) { detail::check(crabby_ctx_free_instance_batch(raw_,b)); }
    gpu_buffer create_gpu_buffer(uint32_t len, const std::vector<std::array<float, 4>>& data = {}) {
        gpu_buffer b{}; detail::check(crabby_ctx_create_gpu_buffer(raw_, len,
            reinterpret_cast<const float (*)[4]>(data.data()), data.size(), &b)); return b;
    }
    void write_gpu_buffer(gpu_buffer b, uint32_t offset, const std::vector<std::array<float, 4>>& data) {
        detail::check(crabby_ctx_write_gpu_buffer(raw_, b, offset, reinterpret_cast<const float (*)[4]>(data.data()), data.size()));
    }
    /// Frees a GPU buffer; its compute passes are removed and meshes using
    /// it draw nothing. The next buffer made takes its id: drop the handle.
    void free_gpu_buffer(gpu_buffer b) { detail::check(crabby_ctx_free_gpu_buffer(raw_, b)); }
    /// Draws `count` instances of a mesh from a GPU buffer, each at its
    /// entity's transform and tint (`items[v.instance_index]` in the shader).
    void set_mesh_gpu_buffer(mesh m, gpu_buffer b, uint32_t count) {
        detail::check(crabby_ctx_set_mesh_gpu_buffer(raw_, m, b, count));
    }
    /// Grows a mesh's culling bounds by the sphere `center`, `radius`: where
    /// its buffer-driven instances can be.
    void set_mesh_gpu_bounds(mesh m, vec3 center, float radius) {
        detail::check(crabby_ctx_set_mesh_gpu_bounds(raw_, m, center, radius));
    }
    /// Loads a compute pass (WGSL defining `fn compute(c: Cell) -> vec4<f32>`)
    /// from assets/, hot reloaded; the same file again gives the same one.
    crabby::compute load_compute(const char* path) { crabby::compute c{}; detail::check(crabby_ctx_load_compute(raw_, path, &c)); return c; }
    /// A compute pass from WGSL source; `name` is for error messages.
    crabby::compute create_compute(const char* name, const char* source) {
        crabby::compute c{}; detail::check(crabby_ctx_create_compute(raw_, name, source, &c)); return c;
    }
    /// The compute passes run each frame, in order, before anything is drawn.
    std::vector<compute_pass> compute_passes() const {
        std::size_t n = 0;
        detail::check(crabby_ctx_compute_passes(raw_, nullptr, 0, &n));
        std::vector<compute_pass> list(n);
        detail::check(crabby_ctx_compute_passes(raw_, list.data(), list.size(), &n));
        return list;
    }
    /// Replaces the compute passes run each frame (empty: none).
    void set_compute_passes(const std::vector<compute_pass>& list) {
        detail::check(crabby_ctx_set_compute_passes(raw_, list.data(), list.size()));
    }
    /// A texture target cameras draw the 3D world into (linear HDR light);
    /// materials use it like any texture.
    texture create_render_target(std::uint32_t w, std::uint32_t h) {
        texture t{}; detail::check(crabby_ctx_create_render_target(raw_, w, h, &t)); return t;
    }
    /// The target camera seeing `c`'s view mirrored in the level plane at
    /// height `level`, clipped to what's above it. Read its target at the
    /// screen position flipped upside down.
    target_camera reflection_camera(const camera3d& c, texture target, float level) {
        target_camera t{}; detail::check(crabby_ctx_reflection_camera(raw_, &c, target, level, &t)); return t;
    }
    /// The cameras drawing into render targets each frame, before the main view.
    std::vector<target_camera> target_cameras() const {
        std::size_t n = 0;
        detail::check(crabby_ctx_target_cameras(raw_, nullptr, 0, &n));
        std::vector<target_camera> list(n);
        detail::check(crabby_ctx_target_cameras(raw_, list.data(), list.size(), &n));
        return list;
    }
    /// Replaces the target cameras drawn each frame (empty: none).
    void set_target_cameras(const std::vector<target_camera>& list) {
        detail::check(crabby_ctx_set_target_cameras(raw_, list.data(), list.size()));
    }
    /// A plain white material, made once and shared.
    material default_material() { material m{}; detail::check(crabby_ctx_default_material(raw_, &m)); return m; }
    /// Puts triangles on the GPU as a mesh of one material; draw it with
    /// entities that have a transform3d and a model_of(it).
    mesh create_mesh(const mesh_data& data, material m) { mesh x{}; detail::check(crabby_ctx_create_mesh(raw_, data.raw(), m, &x)); return x; }
    /// A mesh with no material of its own (Rust `Ctx::create_plain_mesh`):
    /// create_mesh with the default material, for meshes whose look comes from
    /// each model (set_model_material).
    mesh create_plain_mesh(const mesh_data& data) { mesh x{}; detail::check(crabby_ctx_create_plain_mesh(raw_, data.raw(), &x)); return x; }
    /// Loads a glTF model (.glb, or .gltf with its files) from assets/ as
    /// one mesh with its materials; the same file again gives the same mesh.
    mesh load_mesh(const char* path) { mesh x{}; detail::check(crabby_ctx_load_mesh(raw_, path, &x)); return x; }
    /// A texture array of `width` x `height` images, one after another in
    /// `rgba` (4 bytes a pixel).
    texture_array create_texture_array(std::uint32_t width, std::uint32_t height, const std::vector<std::uint8_t>& rgba, texture_kind kind) {
        texture_array a{};
        detail::check(crabby_ctx_create_texture_array(raw_, width, height, rgba.data(), rgba.size(), std::uint32_t(kind), &a));
        return a;
    }
    /// Loads same-size images from assets/ into a texture array, one layer each.
    texture_array load_texture_array(const std::vector<const char*>& paths, texture_kind kind) {
        texture_array a{};
        detail::check(crabby_ctx_load_texture_array(raw_, paths.data(), paths.size(), std::uint32_t(kind), &a));
        return a;
    }
    /// The texture array decals take their images from (Rust `set_decal_textures`).
    /// Make it with texture_kind::data; decals decode colors from sRGB themselves.
    void set_decal_textures(texture_array a) { detail::check(crabby_ctx_set_decal_textures(raw_, a)); }
    /// Back to flat-colored decals.
    void clear_decal_textures() { detail::check(crabby_ctx_clear_decal_textures(raw_)); }
    /// Whether this GPU draws projected decals (false headless and on WebGL2).
    bool supports_decals() { bool yes = false; detail::check(crabby_ctx_supports_decals(raw_, &yes)); return yes; }
    /// Build the 3D pipeline variants frames would build when first drawn
    /// (crossfades, buffer-driven instances); call at the end of loading.
    void warm_render_pipelines() { detail::check(crabby_ctx_warm_render_pipelines(raw_)); }
    /// A material for terrain: ground layers blended by a weight map.
    material create_terrain_material(const terrain_material_desc& desc) {
        material m{}; detail::check(crabby_ctx_create_terrain_material(raw_, &desc, &m)); return m;
    }
    /// A terrain tile `size` wide from n x n heights (row after row along
    /// +Z, NaN for a hole) drawn with `m`; spawn it with a transform3d at
    /// the tile's corner instead of a model.
    terrain_tile create_terrain_tile(const std::vector<float>& heights, float size, material m) {
        terrain_tile t{}; detail::check(crabby_ctx_create_terrain_tile(raw_, heights.data(), heights.size(), size, m, &t)); return t;
    }
    /// Puts a tile built (on any thread) as terrain_tile_data on the GPU.
    terrain_tile upload_terrain_tile(const terrain_tile_data& data, material m) {
        terrain_tile t{}; detail::check(crabby_ctx_upload_terrain_tile(raw_, data.raw(), m, &t)); return t;
    }
    /// Frees every detail level of a tile's GPU memory; despawn its entity too.
    void free_terrain_tile(terrain_tile t) { detail::check(crabby_ctx_free_terrain_tile(raw_, t)); }
    /// Frees a mesh's GPU memory; models still showing it draw nothing.
    void free_mesh(mesh m) { detail::check(crabby_ctx_free_mesh(raw_, m)); }
    /// Captures the scene around reflection probe `e` on the next rendered frame
    /// (only when asked: bake again to refresh). Throws once four environments exist.
    void bake_reflection_probe(entity e) { detail::check(crabby_ctx_bake_reflection_probe(raw_, e)); }
    /// Loads a linear HDR/EXR 2:1 equirectangular image as probe `e`'s environment.
    void load_reflection_probe(entity e, const std::string& name) { detail::check(crabby_ctx_load_reflection_probe(raw_, e, name.c_str())); }
    /// Whether models showing `m` cast shadows (on at first); off suits grass and pebbles.
    void set_mesh_shadows(mesh m, bool casts) { detail::check(crabby_ctx_set_mesh_shadows(raw_, m, casts)); }
    /// Draws `e`'s model with `m` instead of the materials its mesh was made
    /// with, every part (Rust `ModelMaterial`): one uploaded mesh in several
    /// materials. For plain, skinned and morphing models, LOD groups and
    /// terrain tiles.
    void set_model_material(entity e, material m) { detail::check(crabby_ctx_set_model_material(raw_, e, m, true)); }
    /// `e`'s model draws with its mesh's own materials again.
    void clear_model_material(entity e) { detail::check(crabby_ctx_set_model_material(raw_, e, material{}, false)); }
    /// The material set with set_model_material, if any.
    std::optional<material> model_material(entity e) const {
        material m{}; bool has = false;
        detail::check(crabby_ctx_model_material(raw_, e, &m, &has));
        return has ? std::optional<material>(m) : std::nullopt;
    }
    /// Draws the listed parts of `e`'s model with other materials (Rust
    /// `ModelMaterials`): `{part, material}` pairs, parts numbered from 0 in
    /// the order the mesh was made with. Parts not listed keep the material of
    /// set_model_material, or the mesh's own. An empty list takes it off.
    void set_model_part_materials(entity e, const std::vector<std::pair<std::uint32_t, material>>& parts) {
        std::vector<std::uint32_t> numbers;
        std::vector<material> materials;
        for (const auto& [part, m] : parts) { numbers.push_back(part); materials.push_back(m); }
        detail::check(crabby_ctx_set_model_part_materials(raw_, e, numbers.data(), materials.data(), parts.size()));
    }
    /// The list set with set_model_part_materials, by part number (empty: none).
    std::vector<std::pair<std::uint32_t, material>> model_part_materials(entity e) const {
        std::size_t n = 0;
        detail::check(crabby_ctx_model_part_materials(raw_, e, nullptr, nullptr, 0, &n));
        std::vector<std::uint32_t> numbers(n);
        std::vector<material> materials(n);
        detail::check(crabby_ctx_model_part_materials(raw_, e, numbers.data(), materials.data(), n, &n));
        std::vector<std::pair<std::uint32_t, material>> parts;
        for (std::size_t i = 0; i < n && i < numbers.size(); ++i) parts.emplace_back(numbers[i], materials[i]);
        return parts;
    }
    /// Draws `m` into the shadow maps for `e` instead of its model (Rust `ShadowMesh`);
    /// without a model, `e` casts only that shadow.
    void set_shadow_mesh(entity e, mesh m) { detail::check(crabby_ctx_set_shadow_mesh(raw_, e, m, true)); }
    /// `e` casts no shadow (a model, or a terrain tile's ground).
    void set_no_shadow(entity e) { detail::check(crabby_ctx_set_shadow_mesh(raw_, e, mesh{}, false)); }
    /// Gives `e` levels of detail (Rust `LodGroup`), most detailed first, for an
    /// object `size` tall, its distance measured to `center` in its own space.
    void set_lod_group(entity e, float size, const std::vector<lod_level>& levels, vec3 center = {0, 0, 0}) {
        std::vector<CrabbyLodLevel> raw;
        raw.reserve(levels.size());
        for (const lod_level& l : levels) {
            raw.push_back(CrabbyLodLevel{l.screen_height, l.fade_width, l.renderers.data(), l.renderers.size(),
                                         l.custom_shadow, l.shadow_casters.data(), l.shadow_casters.size()});
        }
        detail::check(crabby_ctx_set_lod_group(raw_, e, size, center, raw.data(), raw.size()));
    }
    /// The level of `e`'s LOD group the 3D camera picks now.
    lod_pick lod_pick_of(entity e) const { lod_pick p{}; detail::check(crabby_ctx_lod_pick(raw_, e, &p)); return p; }
    lod_settings lod() const { lod_settings s{}; detail::check(crabby_ctx_lod_settings(raw_, &s)); return s; }
    /// bias 0 or more: 2 keeps detail twice as far, 0.5 drops it at half the distance.
    void set_lod(const lod_settings& s) { detail::check(crabby_ctx_set_lod_settings(raw_, s)); }
    /// Puts a skeleton of `joints`, at rest, on `e` (Rust `Skeleton::new`).
    void set_skeleton(entity e, const std::vector<joint>& joints) {
        std::vector<CrabbyJoint> raw;
        raw.reserve(joints.size());
        for (const joint& j : joints) raw.push_back(CrabbyJoint{j.name.c_str(), j.parent, j.rest});
        detail::check(crabby_ctx_set_skeleton(raw_, e, raw.data(), raw.size()));
    }
    /// Puts the skeleton of a glTF file with skins on `e`, at rest (Rust `Graphics::load_skeleton`).
    void load_skeleton(entity e, const char* path) { detail::check(crabby_ctx_load_skeleton(raw_, e, path)); }
    /// A mesh that bends with the skeleton on `skeleton`: `data`'s joints
    /// (mesh_data::set_skin) index its joints; modelled with it at rest.
    mesh create_skinned_mesh(const mesh_data& data, material m, entity skeleton) {
        mesh x{}; detail::check(crabby_ctx_create_skinned_mesh(raw_, data.raw(), m, skeleton, &x)); return x;
    }
    std::size_t joint_count(entity e) const { std::size_t n = 0; detail::check(crabby_ctx_joint_count(raw_, e, &n)); return n; }
    /// The first joint of `e`'s skeleton called `name`.
    std::optional<std::size_t> find_joint(entity e, const char* name) const {
        bool found = false; std::size_t j = 0;
        detail::check(crabby_ctx_find_joint(raw_, e, name, &found, &j));
        return found ? std::optional<std::size_t>(j) : std::nullopt;
    }
    /// Where joint `j` of `e`'s skeleton sits relative to its parent this frame.
    transform3d joint_pose(entity e, std::size_t j) const { transform3d t{}; detail::check(crabby_ctx_joint_pose(raw_, e, j, &t)); return t; }
    void set_joint_pose(entity e, std::size_t j, const transform3d& pose) { detail::check(crabby_ctx_set_joint_pose(raw_, e, j, &pose)); }
    /// Puts every joint of `e`'s skeleton back at rest.
    void reset_pose(entity e) { detail::check(crabby_ctx_reset_pose(raw_, e)); }
    /// Joint `j`'s matrix in `e`'s own space (column after column), from its pose now.
    std::array<float, 16> joint_matrix(entity e, std::size_t j) const {
        std::array<float, 16> m{}; detail::check(crabby_ctx_joint_matrix(raw_, e, j, m.data())); return m;
    }
    /// Writes manually sampled skeleton morph channels to models.
    void write_animated_morphs() { detail::check(crabby_ctx_write_animated_morphs(raw_)); }
    /// The index of mesh `m`'s morph target called `name` (Rust
    /// `Graphics::morph_target`), for set_morph_weight and morph_weight.
    std::optional<std::size_t> morph_target(mesh m, const char* name) const {
        std::size_t result = 0;
        const CrabbyStatus status = crabby_ctx_morph_target(raw_, m, name, &result);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return result;
    }
    /// Sets how much of morph target `t` `e`'s model shows: 0 none, 1 all
    /// of it (Rust `MorphWeights::set`); outside that range exaggerates.
    /// Gives `e` morph weights if it had none, so it stops showing its
    /// mesh's default weights.
    void set_morph_weight(entity e, std::size_t t, float weight) {
        detail::check(crabby_ctx_set_morph_weight(raw_, e, t, weight));
    }
    /// How much of morph target `t` `e`'s model shows, or nothing when `e`
    /// has no morph weights: then it shows its mesh's default weights.
    std::optional<float> morph_weight(entity e, std::size_t t) const {
        float result = 0;
        const CrabbyStatus status = crabby_ctx_morph_weight(raw_, e, t, &result);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return result;
    }
    /// Takes `e`'s morph weights away, so its model shows its mesh's
    /// default weights again (Rust `MorphWeights` removed).
    void clear_morph_weights(entity e) { detail::check(crabby_ctx_clear_morph_weights(raw_, e)); }
    /// The clip called `name` of a glTF file with skins (Rust `Graphics::load_clip`).
    crabby::clip load_clip(const char* path, const char* name) {
        CrabbyClip* c = nullptr; detail::check(crabby_ctx_load_clip(raw_, path, name, &c)); return crabby::clip(c);
    }
    /// Poses `e`'s skeleton with `c` at `time`, blending from its pose now
    /// by `weight` (1: the clip's pose).
    void sample_clip(const crabby::clip& c, entity e, float time, float weight = 1) {
        detail::check(crabby_ctx_sample_clip(raw_, c.raw(), e, time, weight));
    }
    /// Bends `e`'s chain upper -> middle -> end so `end` reaches `target` (world), the middle
    /// bending toward `pole`. Out of reach: a straight chain pointing at it. Use in a late system.
    void two_bone_ik(entity e, std::size_t upper, std::size_t middle, std::size_t end, vec3 target, vec3 pole, float weight = 1) {
        detail::check(crabby_ctx_two_bone_ik(raw_, e, upper, middle, end, target, pole, weight));
    }
    /// Turns joint `j` of `e` so its local `axis` points at `target` (world), at most `max_angle` radians.
    void look_at_joint(entity e, std::size_t j, vec3 axis, vec3 target, float max_angle = 3.1415927f, float weight = 1) {
        detail::check(crabby_ctx_look_at_joint(raw_, e, j, axis, target, max_angle, weight));
    }
    /// Draws `e`'s bones this frame, over the scene.
    void draw_skeleton(entity e, color c) { detail::check(crabby_ctx_draw_skeleton(raw_, e, c)); }
    /// Bakes `sources` into this context's navmesh (agents replan). Throws with the reason on bad geometry.
    void bake_navmesh(const nav_sources& sources, const nav_bake_settings& settings = nav_bake_defaults()) {
        detail::check(crabby_ctx_bake_navmesh(raw_, sources.raw(), &settings));
    }
    /// The nearest walkable point within `max_distance`, or std::nullopt.
    std::optional<vec3> navmesh_sample_position(vec3 point, float max_distance) {
        vec3 p{}; bool found = false;
        detail::check(crabby_ctx_navmesh_sample_position(raw_, point, max_distance, &p, &found));
        if (!found) return std::nullopt;
        return p;
    }
    /// A path's corners from `from` to `to`, start and end included; empty when there is none.
    std::vector<vec3> navmesh_find_path(vec3 from, vec3 to) {
        std::vector<vec3> corners(32);
        std::size_t len = 0;
        detail::check(crabby_ctx_navmesh_find_path(raw_, from, to, corners.data(), corners.size(), &len));
        if (len > corners.size()) {
            corners.resize(len);
            detail::check(crabby_ctx_navmesh_find_path(raw_, from, to, corners.data(), corners.size(), &len));
        }
        corners.resize(len);
        return corners;
    }
    /// Makes `e` (with a transform3d) an agent; moves only with game::navmesh3d().
    /// Opt into sampled crowd avoidance; weight controls separation steering.
    void set_nav_avoidance(entity e, bool enabled = true, float weight = 2, float neighbour_distance = 3.6f) {
        detail::check(crabby_ctx_set_nav_avoidance(raw_, e, enabled, weight, neighbour_distance));
    }
    void set_nav_agent(entity e, float speed = 3, float radius = 0.3f, float acceleration = 10) {
        detail::check(crabby_ctx_set_nav_agent(raw_, e, speed, radius, acceleration));
    }
    /// Sends `e`'s agent to `destination`; std::nullopt stops it.
    void set_nav_destination(entity e, std::optional<vec3> destination) {
        detail::check(crabby_ctx_set_nav_destination(raw_, e, destination ? &*destination : nullptr));
    }
    /// Makes `e` a carving box obstacle: cached tile layers are rebuilt to cut a real hole.
    /// Simulation consumes at most one layer per tick.
    void set_nav_carve_obstacle(entity e, vec3 center, vec3 half_extents) {
        detail::check(crabby_ctx_set_nav_carve_obstacle(raw_, e, center, half_extents));
    }
    /// Runs up to `budget` extra tile updates after simulation has gathered obstacle
    /// components; returns the steps taken and whether updates are still pending.
    std::pair<std::size_t, bool> navmesh_update_carving(std::size_t budget = 1) {
        std::size_t steps = 0; bool pending = false;
        detail::check(crabby_ctx_navmesh_update_carving(raw_, budget, &steps, &pending));
        return {steps, pending};
    }
    /// Makes `e` a box obstacle agents path around.
    void set_nav_obstacle(entity e, vec3 center, vec3 half_extents) {
        detail::check(crabby_ctx_set_nav_obstacle(raw_, e, center, half_extents));
    }
    void remove_nav(entity e) { detail::check(crabby_ctx_remove_nav(raw_, e)); }
    /// Gives `e` (which needs a 3D transform) a new particle emitter, replacing any old one.
    void set_particle_emitter3d(entity e, const particle_settings& settings) {
        CrabbyParticleSettings3D c = settings.raw();
        detail::check(crabby_ctx_set_particle_emitter3d(raw_, e, &c));
    }
    /// Emits `count` particles from `e`'s emitter on the next frame.
    void particle_burst(entity e, std::uint32_t count) { detail::check(crabby_ctx_particle_burst(raw_, e, count)); }
    /// How many of `e`'s particles are alive.
    std::uint32_t particle_live_count(entity e) {
        std::uint32_t n = 0;
        detail::check(crabby_ctx_particle_live_count(raw_, e, &n));
        return n;
    }
    /// Removes `e`'s emitter and its particles.
    void remove_particle_emitter3d(entity e) { detail::check(crabby_ctx_remove_particle_emitter3d(raw_, e)); }
    /// Gives `e` (which needs a 3D transform) a new ribbon trail, replacing any old one.
    void set_trail3d(entity e, const trail3d& trail) {
        CrabbyTrail3D c = trail.raw();
        detail::check(crabby_ctx_set_trail3d(raw_, e, &c));
    }
    /// How many points `e`'s trail keeps now (trails only record in visual frames).
    std::uint32_t trail3d_point_count(entity e) {
        std::uint32_t n = 0;
        detail::check(crabby_ctx_trail3d_point_count(raw_, e, &n));
        return n;
    }
    /// Forgets every point of `e`'s trail: after a teleport, so no ribbon spans the jump.
    void clear_trail3d(entity e) { detail::check(crabby_ctx_clear_trail3d(raw_, e)); }
    /// Starts or stops `e`'s trail adding points; while off, the ribbon fades out.
    void set_trail3d_emitting(entity e, bool emitting) { detail::check(crabby_ctx_set_trail3d_emitting(raw_, e, emitting)); }
    void remove_trail3d(entity e) { detail::check(crabby_ctx_remove_trail3d(raw_, e)); }
    /// Gives `e` (which needs a 3D transform) a line, replacing any old one.
    void set_line3d(entity e, const line3d& line) {
        CrabbyLine3D c = line.raw();
        detail::check(crabby_ctx_set_line3d(raw_, e, &c));
    }
    void remove_line3d(entity e) { detail::check(crabby_ctx_remove_line3d(raw_, e)); }
    /// A glTF file's mesh on the CPU (Rust `Graphics::load_mesh_data`), all its
    /// parts in one, without making anything on the GPU: to cut decals from.
    /// Loads the file each call; keep the result.
    mesh_data load_mesh_data(const char* path) {
        CrabbyMeshData* d = nullptr;
        detail::check(crabby_ctx_load_mesh_data(raw_, path, &d));
        return mesh_data(d);
    }
    /// Gives `e` (with a skeleton) an animator playing `g`; the engine advances it every frame.
    void set_animator(entity e, const animation_graph& g) { detail::check(crabby_ctx_set_animator(raw_, e, g.raw())); }
    void animator_set_float(entity e, std::size_t parameter, float value) { detail::check(crabby_ctx_animator_set_float(raw_, e, parameter, value)); }
    void animator_set_bool(entity e, std::size_t parameter, bool value) { detail::check(crabby_ctx_animator_set_bool(raw_, e, parameter, value)); }
    void animator_set_trigger(entity e, std::size_t parameter, bool value = true) { detail::check(crabby_ctx_animator_set_trigger(raw_, e, parameter, value)); }
    /// The state `e`'s animator is in (or fading to).
    std::size_t animator_state(entity e) { std::size_t s = 0; detail::check(crabby_ctx_animator_state(raw_, e, &s, nullptr)); return s; }
    /// How far through its state it is: 1 is one full play.
    float animator_normalized_time(entity e) { float t = 0; detail::check(crabby_ctx_animator_state(raw_, e, nullptr, &t)); return t; }
    void animator_crossfade(entity e, std::size_t state, float duration, float offset = 0) {
        detail::check(crabby_ctx_animator_crossfade(raw_, e, state, duration, offset));
    }
    /// Jumps to `state` at normalized time `time` on the next update.
    void animator_play(entity e, std::size_t state, float time = 0) { animator_crossfade(e, state, 0, time); }
    /// Adds a layer playing `g` over `e`'s base animation; returns its index.
    /// `mask_joint` limits it to that joint and its children ("" for all joints).
    std::size_t animator_add_layer(entity e, const animation_graph& g, layer_blend mode = layer_blend::override_pose,
                                   const std::string& mask_joint = "") {
        std::size_t index = 0;
        detail::check(crabby_ctx_animator_add_layer(raw_, e, g.raw(), static_cast<std::uint32_t>(mode),
                                                    mask_joint.empty() ? nullptr : mask_joint.c_str(), &index));
        return index;
    }
    void animator_set_layer_weight(entity e, std::size_t layer, float weight) {
        detail::check(crabby_ctx_animator_set_layer_weight(raw_, e, layer, weight));
    }
    void animator_layer_set_float(entity e, std::size_t layer, std::size_t parameter, float value) {
        detail::check(crabby_ctx_animator_layer_set_float(raw_, e, layer, parameter, value));
    }
    void animator_layer_set_bool(entity e, std::size_t layer, std::size_t parameter, bool value) {
        detail::check(crabby_ctx_animator_layer_set_bool(raw_, e, layer, parameter, value));
    }
    void animator_layer_set_trigger(entity e, std::size_t layer, std::size_t parameter, bool value = true) {
        detail::check(crabby_ctx_animator_layer_set_trigger(raw_, e, layer, parameter, value));
    }
    void animator_layer_crossfade(entity e, std::size_t layer, std::size_t state, float duration, float offset = 0) {
        detail::check(crabby_ctx_animator_layer_crossfade(raw_, e, layer, state, duration, offset));
    }
    /// What `e`'s animator does with joint `root_joint`'s horizontal movement and turning.
    void animator_set_root_motion(entity e, root_motion mode, std::size_t root_joint = 0) {
        detail::check(crabby_ctx_animator_set_root_motion(raw_, e, static_cast<std::uint32_t>(mode), root_joint));
    }
    /// The movement (entity axes, before scale) and turn taken out of the pose in the latest update.
    std::pair<vec3, quat> animator_root_motion(entity e) {
        vec3 p{};
        quat r{0, 0, 0, 1};
        detail::check(crabby_ctx_animator_root_motion(raw_, e, &p, &r));
        return {p, r};
    }
    void animator_set_manual(entity e, bool manual) { detail::check(crabby_ctx_animator_set_manual(raw_, e, manual)); }
    /// Advances `e`'s animator by `dt` and poses its skeleton now.
    void animator_update(entity e, float dt) { detail::check(crabby_ctx_animator_update(raw_, e, dt)); }
    /// The oldest unread event of `e`'s animator, or std::nullopt.
    std::optional<std::string> animator_next_event(entity e) {
        char name[128];
        bool found = false;
        detail::check(crabby_ctx_animator_next_event(raw_, e, name, sizeof name, &found));
        if (!found) return std::nullopt;
        return std::string(name);
    }
    /// Poses `e`'s skinned model with `skeleton`'s skeleton and draws it
    /// there (Rust `SkinOf`), binding joints by name.
    void set_skin_of(entity e, entity skeleton) { detail::check(crabby_ctx_set_skin_of(raw_, e, skeleton)); }
    /// Keeps `e`'s transform on joint `j` of `skeleton`, moved by `offset`
    /// in the joint's space (Rust `AttachToJoint`).
    void attach_to_joint(entity e, entity skeleton, std::size_t j, const transform3d& offset) {
        detail::check(crabby_ctx_attach_to_joint(raw_, e, skeleton, j, &offset));
    }
    bool mesh_shadows(mesh m) const { bool casts = true; detail::check(crabby_ctx_mesh_shadows(raw_, m, &casts)); return casts; }
    /// Whether this game made `m` and hasn't freed it.
    bool is_mesh_live(mesh m) const { bool live = false; detail::check(crabby_ctx_is_mesh_live(raw_, m, &live)); return live; }
    /// Whether handle `h` (a texture, sound, font, mesh, material, shader,
    /// compute, effect, gpu_buffer, instance_batch or texture_array) names
    /// something this game made and hasn't freed (Rust `Ctx::is_live`).
    template<class H> bool is_live(H h) const {
        bool live = false;
        detail::check(crabby_ctx_handle_is_live(raw_, detail::handle_kind<H>::value, h.id, &live));
        return live;
    }
    /// Frees handle `h` (Rust `Ctx::free`): its id is never reused (except
    /// for meshes and GPU buffers, whose next one made takes it), and
    /// whatever still uses it draws the default (nothing for
    /// a mesh, white for a texture, the default material, the built-in font).
    template<class H> void free(H h) {
        detail::check(crabby_ctx_handle_free(raw_, detail::handle_kind<H>::value, h.id));
    }
    /// How many handles of each kind are live now (Rust `Ctx::live_counts`).
    crabby::live_counts live_counts() const {
        crabby::live_counts c{}; detail::check(crabby_ctx_live_counts(raw_, &c)); return c;
    }
    /// Sets `kind`'s residency budget in bytes; std::nullopt for no limit
    /// (Rust `Ctx::set_residency_budget`).
    void set_residency_budget(residency_kind kind, std::optional<std::uint64_t> bytes) {
        detail::check(crabby_ctx_set_residency_budget(raw_, static_cast<std::uint32_t>(kind), bytes.has_value(), bytes.value_or(0)));
    }
    /// `kind`'s budget and counters (Rust `Graphics::residency`).
    crabby::residency_stats residency(residency_kind kind) const {
        crabby::residency_stats r{}; detail::check(crabby_ctx_residency(raw_, static_cast<std::uint32_t>(kind), &r)); return r;
    }
    /// Whether the asset of `kind` called `name` is resident.
    bool is_resident(residency_kind kind, const char* name) const {
        bool r = false; detail::check(crabby_ctx_is_resident(raw_, static_cast<std::uint32_t>(kind), name, &r)); return r;
    }
    /// Texture `path` at full resolution under the texture budget, loaded
    /// now if it isn't; with `pin`, never evicted until unpinned (Rust
    /// `Ctx::resident_texture`, `pin_resident_texture`).
    texture resident_texture(const char* path, bool pin = false) {
        texture t{}; detail::check(crabby_ctx_resident_texture(raw_, path, pin, &t)); return t;
    }
    /// Mesh `path` under the mesh budget (Rust `Ctx::resident_mesh`, `pin_resident_mesh`).
    mesh resident_mesh(const char* path, bool pin = false) {
        mesh m{}; detail::check(crabby_ctx_resident_mesh(raw_, path, pin, &m)); return m;
    }
    /// Lets a pinned asset be evicted again.
    void unpin_resident(residency_kind kind, const char* path) {
        detail::check(crabby_ctx_unpin_resident(raw_, static_cast<std::uint32_t>(kind), path));
    }
    /// Texture `path` streamed in without waiting (Rust `Ctx::stream_texture`):
    /// std::nullopt until its stand-in is uploaded.
    std::optional<texture> stream_texture(const char* path) {
        texture t{}; bool ready = false;
        detail::check(crabby_ctx_stream_texture(raw_, path, &t, &ready));
        if (!ready) return std::nullopt;
        return t;
    }
    /// Mesh `path` streamed in without waiting (Rust `Ctx::stream_mesh`).
    std::optional<mesh> stream_mesh(const char* path) {
        mesh m{}; bool ready = false;
        detail::check(crabby_ctx_stream_mesh(raw_, path, &m, &ready));
        if (!ready) return std::nullopt;
        return m;
    }
    /// Where the asset of `kind` called `name` is (Rust `Graphics::asset_state`).
    crabby::asset_state asset_state(residency_kind kind, const char* name) const {
        std::uint32_t s = 0;
        detail::check(crabby_ctx_asset_state(raw_, static_cast<std::uint32_t>(kind), name, &s));
        return static_cast<crabby::asset_state>(s);
    }
    /// Why loading it failed; empty if it didn't (Rust `Graphics::asset_error`).
    std::string asset_error(residency_kind kind, const char* name) const {
        std::size_t len = 0;
        detail::check(crabby_ctx_asset_error(raw_, static_cast<std::uint32_t>(kind), name, nullptr, 0, &len));
        std::string text(len + 1, '\0');
        detail::check(crabby_ctx_asset_error(raw_, static_cast<std::uint32_t>(kind), name, text.data(), text.size(), &len));
        text.resize(len);
        return text;
    }
    /// Bytes the upload step may send each frame; std::nullopt for no limit.
    void set_upload_budget(std::optional<std::uint64_t> bytes) {
        detail::check(crabby_ctx_set_upload_budget(raw_, bytes.has_value(), bytes.value_or(0)));
    }
    /// Asset states change in the same frames every run (Rust `Graphics::set_deterministic_uploads`).
    void set_deterministic_uploads(bool on) { detail::check(crabby_ctx_set_deterministic_uploads(raw_, on)); }
    /// The upload queue's budget and counters.
    crabby::upload_stats upload_stats() const {
        crabby::upload_stats u{}; detail::check(crabby_ctx_upload_stats(raw_, &u)); return u;
    }
    /// Waits until everything streamed so far is loaded (loading screens, tests).
    void finish_uploads() { detail::check(crabby_ctx_finish_uploads(raw_)); }
    /// Streams the levels of `e`'s LOD group in from `files`, one per level
    /// (Rust `StreamedLod`).
    void set_streamed_lod(entity e, const std::vector<const char*>& files) {
        detail::check(crabby_ctx_set_streamed_lod(raw_, e, files.data(), files.size()));
    }
    /// How many 3D models were drawn last frame.
    std::size_t models_drawn() const { std::size_t n = 0; detail::check(crabby_ctx_models_drawn(raw_, &n)); return n; }
    crabby::camera3d camera3d() const { crabby::camera3d c{}; detail::check(crabby_ctx_camera3d(raw_, &c)); return c; }
    /// fov_y between 0 and pi, 0 < near < far.
    void set_camera3d(const crabby::camera3d& c) { detail::check(crabby_ctx_set_camera3d(raw_, &c)); }
    /// Turns the 3D camera to look at `target`, +Y up.
    void camera3d_look_at(vec3 target) { detail::check(crabby_ctx_camera3d_look_at(raw_, target)); }
    /// Skip models hidden behind what was drawn a frame or two before (off by default).
    void set_occlusion_culling(bool on) { detail::check(crabby_ctx_set_occlusion_culling(raw_, on)); }
    bool occlusion_culling() const { bool on = false; detail::check(crabby_ctx_occlusion_culling(raw_, &on)); return on; }
    /// The ray from the 3D camera through a window point (as the mouse
    /// position): its start and unit direction.
    std::pair<vec3, vec3> screen_ray(vec2 screen) const {
        vec3 origin{}, direction{}; detail::check(crabby_ctx_screen_ray(raw_, screen, &origin, &direction)); return {origin, direction};
    }
    crabby::lighting lighting() const { crabby::lighting l{}; detail::check(crabby_ctx_lighting(raw_, &l)); return l; }
    /// Intensities and the shadow distance can't be negative; shadow_cascades is 1 to 4,
    /// shadow_refresh 1 to 8, shadow_filter 1 to 3; shadow_resolution is 256 to 4096, light_shadow_resolution
    /// 128 to 2048, light_shadow_views 0 to 24 and light_shadow_atlas a power of two
    /// from 1024 to 8192. With gi_enabled, gi_extent and gi_spacing must be above
    /// zero, gi_probes_per_frame 1 to 16 and gi_intensity not negative.
    void set_lighting(const crabby::lighting& l) { detail::check(crabby_ctx_set_lighting(raw_, &l)); }
    /// The sky behind the 3D world and the fog in front of it.
    crabby::sky sky() const { crabby::sky s{}; detail::check(crabby_ctx_sky(raw_, &s)); return s; }
    /// Haze, stars, fog density, fog_start, fog_end and fog_sun_glow can't be negative;
    /// sun_size and fog_falloff must be above 0; fog_sun_focus is 0 to under 1;
    /// light_shafts is 0 to 1.
    void set_sky(const crabby::sky& s) { detail::check(crabby_ctx_set_sky(raw_, &s)); }
    /// Exposure, bloom, color grading and tone mapping of the 3D frame.
    crabby::post post() const { crabby::post p{}; detail::check(crabby_ctx_post(raw_, &p)); return p; }
    /// GTAO ao_half_resolution uses bilateral upsampling; ao_temporal accumulates
    /// camera-reprojected history independently of TAA. Contact shadows stay full resolution.
    /// Bloom, its threshold, contrast and saturation can't be negative; render_scale is
    /// 0.25 to 2 and sharpen 0 to 1; contact_shadows can't be negative; ssr is 0 to 1 and
    /// ssr_max_distance above zero.
    void set_post(const crabby::post& p) { detail::check(crabby_ctx_set_post(raw_, &p)); }
    /// The current graphics quality, gathered from the post and lighting settings.
    crabby::graphics_quality graphics_quality() const {
        crabby::graphics_quality q{}; detail::check(crabby_ctx_graphics_quality(raw_, &q)); return q;
    }
    /// Switches the 3D picture to `q` without a restart: the next frame draws at the new
    /// resolution scale and shadow map sizes. Throws, changing nothing, for a field
    /// outside its range.
    void set_graphics_quality(const crabby::graphics_quality& q) { detail::check(crabby_ctx_set_graphics_quality(raw_, &q)); }
    /// The GPU report as JSON (Rust `ctx.gpu_capabilities()`): adapter, limits,
    /// tier, forced_tier and one {feature, available, fallback, reason} entry per
    /// feature. Empty before a GPU opens (headless, before the window).
    std::optional<std::string> gpu_capabilities() const {
        std::size_t len = 0;
        CrabbyStatus status = crabby_ctx_gpu_capabilities(raw_, nullptr, 0, &len);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return detail::read_text([&](char* b, std::size_t c, std::size_t* n) { return crabby_ctx_gpu_capabilities(raw_, b, c, n); });
    }
    /// The GPU's tier; empty before a GPU opens.
    std::optional<crabby::gpu_tier> gpu_tier() const {
        uint32_t t = 0;
        CrabbyStatus status = crabby_ctx_gpu_tier(raw_, &t);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status); return crabby::gpu_tier(t);
    }
    /// Whether the GPU has feature `f` (false before a GPU opens); the fallback
    /// and the reason are in gpu_capabilities().
    bool gpu_has(crabby::gpu_feature f) const {
        bool on = false;
        CrabbyStatus status = crabby_ctx_gpu_feature(raw_, uint32_t(f), &on);
        if (status == CRABBY_NOT_FOUND) return false;
        detail::check(status); return on;
    }
    /// The preset for the open GPU, fitted to what it has (Rust
    /// `GraphicsQuality::recommended`); empty before a GPU opens.
    std::optional<crabby::graphics_quality> recommended_graphics_quality() const {
        crabby::graphics_quality q{};
        CrabbyStatus status = crabby_ctx_recommended_graphics_quality(raw_, &q);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status); return q;
    }
    /// Makes temporal antialiasing and temporal AO forget earlier frames: call it
    /// after a camera cut or an object teleport. (Resizes and big camera jumps
    /// reset by themselves.)
    void reset_temporal_history() { detail::check(crabby_ctx_reset_temporal_history(raw_)); }
    void save_screenshot(const char* path) { detail::check(crabby_ctx_save_screenshot(raw_, path)); }
    /// texture_filter::nearest keeps pixel art square.
    void set_texture_filter(texture t, texture_filter f) {
        detail::check(crabby_ctx_set_texture_filter(raw_, t, f == texture_filter::nearest));
    }

    // Shapes (world space).
    void draw_rect(rect r, color c) { detail::check(crabby_ctx_draw_rect(raw_, r, c)); }
    void draw_line(vec2 a, vec2 b, float width, color c) { detail::check(crabby_ctx_draw_line(raw_, a, b, width, c)); }
    void draw_circle(vec2 center, float radius, color c) { detail::check(crabby_ctx_draw_circle(raw_, center, radius, c)); }


    /// Adds or replaces owned SDF text on an entity with a Transform3D.
    void set_text3d(entity e, const crabby::text3d& t) {
        CrabbyText3D v{t.font, t.size, t.color, t.outline, t.outline_width,
                       t.softness, static_cast<uint32_t>(t.align), t.billboard};
        detail::check(crabby_ctx_set_text3d(raw_, e, t.text.c_str(), &v));
    }
    /// No value when the live entity has no text.
    std::optional<crabby::text3d> text3d(entity e) const {
        CrabbyText3D v{};
        auto status = crabby_ctx_text3d(raw_, e, &v);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        std::size_t len = 0;
        detail::check(crabby_ctx_text3d_text(raw_, e, nullptr, 0, &len));
        std::string text(len + 1, '\0');
        detail::check(crabby_ctx_text3d_text(raw_, e, &text[0], text.size(), &len));
        text.resize(len);
        crabby::text3d t(std::move(text));
        t.font = v.font; t.size = v.size; t.color = v.color; t.outline = v.outline;
        t.outline_width = v.outline_width; t.softness = v.softness;
        t.align = static_cast<text_align3d>(v.align); t.billboard = v.billboard;
        return t;
    }
    void remove_text3d(entity e) { detail::check(crabby_ctx_remove_text3d(raw_, e)); }

    // Fonts and text (screen space).
    font load_font(const char* path) { font f{}; detail::check(crabby_ctx_load_font(raw_, path, &f)); return f; }
    void draw_text(const char* text, vec2 pos, const text_params& params) { detail::check(crabby_ctx_draw_text_ex(raw_, text, pos, &params)); }
    vec2 measure_text(const char* text, font f, float size) { vec2 v{}; detail::check(crabby_ctx_measure_text_ex(raw_, text, f, size, &v)); return v; }

    // UI widgets (screen space), styled by ui_style().
    crabby::ui_style ui_style() const { crabby::ui_style s{}; detail::check(crabby_ctx_ui_style(raw_, &s)); return s; }
    void set_ui_style(const crabby::ui_style& s) { detail::check(crabby_ctx_set_ui_style(raw_, &s)); }
    bool mouse_over_ui() const { bool b = false; detail::check(crabby_ctx_mouse_over_ui(raw_, &b)); return b; }
    void ui_panel(rect r) { detail::check(crabby_ctx_ui_panel(raw_, r)); }
    vec2 ui_label(vec2 pos, const char* text) { vec2 v{}; detail::check(crabby_ctx_ui_label(raw_, pos, text, &v)); return v; }
    bool ui_button(rect r, const char* label) { bool b = false; detail::check(crabby_ctx_ui_button(raw_, r, label, &b)); return b; }
    /// The retained UI tree (Rust `ctx.ui_retained`): panels, buttons, text inputs and more, with real layout.
    ui::tree ui_retained() const { return ui::tree(raw_); }

    // World-space UI panels (Rust `UiPanel3D`): retained UI drawn into a render target shown on a quad.
    /// Gives `e` (which needs a `transform3d`) a UI panel `width` x `height` pixels and `size` meters
    /// in the world, plus the model that shows it: a glowing, alpha-blended quad facing +Z. Build its
    /// UI with `panel_ui(e)`. Replaces `e`'s model and any panel it had. Returns the panel's texture.
    texture create_ui_panel(entity e, std::uint32_t width, std::uint32_t height, vec2 size) {
        texture t{}; detail::check(crabby_ctx_create_ui_panel(raw_, e, width, height, size, &t)); return t;
    }
    /// The UI of `e`'s panel (Rust `UiPanel3D::ui`): the same calls as `ui_retained()`, in the panel's pixels.
    ui::tree panel_ui(entity e) const { return ui::tree(raw_, e); }
    /// Removes `e`'s panel; its model stays.
    void remove_ui_panel(entity e) { detail::check(crabby_ctx_remove_ui_panel(raw_, e)); }
    /// Whether the mouse and keyboard can use `e`'s panel (default true).
    void set_ui_panel_interactive(entity e, bool on) { detail::check(crabby_ctx_set_ui_panel_interactive(raw_, e, on)); }
    /// Where the mouse (or the pointer ray) is on `e`'s panel this frame, in its pixels; empty when it isn't over it.
    std::optional<vec2> ui_panel_pointer(entity e) const {
        bool over = false; vec2 at{};
        detail::check(crabby_ctx_ui_panel_pointer(raw_, e, &over, &at));
        return over ? std::optional<vec2>(at) : std::nullopt;
    }
    /// Panels use the ray from `origin` along `direction` instead of the camera's ray through the mouse:
    /// for a locked cursor use the camera's forward ray, for VR a controller's. `clear_ui_pointer_ray` goes back to the mouse.
    void set_ui_pointer_ray(vec3 origin, vec3 direction) { detail::check(crabby_ctx_set_ui_pointer_ray(raw_, origin, direction)); }
    void clear_ui_pointer_ray() { detail::check(crabby_ctx_clear_ui_pointer_ray(raw_)); }

    // Sound (silent in headless runs).
    sound load_sound(const char* path) { sound s{}; detail::check(crabby_ctx_load_sound(raw_, path, &s)); return s; }
    sound create_sound(std::uint32_t sample_rate, const float* samples, std::size_t len) {
        sound s{}; detail::check(crabby_ctx_create_sound(raw_, sample_rate, samples, len, &s)); return s;
    }
    playing play_sound(sound s) { playing p{}; detail::check(crabby_ctx_play_sound(raw_, s, nullptr, &p)); return p; }
    playing play_sound(sound s, const sound_params& params) { playing p{}; detail::check(crabby_ctx_play_sound(raw_, s, &params, &p)); return p; }
    void stop_sound(playing p) { detail::check(crabby_ctx_stop_sound(raw_, p)); }
    /// A long sound (music) streamed from disk as it plays.
    sound load_stream(const char* path) { sound s{}; detail::check(crabby_ctx_load_stream(raw_, path, &s)); return s; }
    /// The bus named `name`, made the first time; keep the handle.
    CrabbyAudioBus audio_bus(const char* name) { CrabbyAudioBus b{}; detail::check(crabby_ctx_audio_bus(raw_, name, &b)); return b; }
    void set_audio_bus_volume(CrabbyAudioBus b, float volume) { detail::check(crabby_ctx_set_audio_bus_volume(raw_, b, volume)); }
    void mute_audio_bus(CrabbyAudioBus b, bool muted) { detail::check(crabby_ctx_mute_audio_bus(raw_, b, muted)); }
    playing play_sound_on(sound s, CrabbyAudioBus b, const sound_params& params = sound_defaults(),
                          const audio_effects& effects = audio_effects_defaults()) {
        playing p{}; detail::check(crabby_ctx_play_sound_on(raw_, s, &params, b, &effects, &p)); return p;
    }
    /// Changes a playing voice's low-pass and reverb, smoothly.
    void set_sound_effects(playing p, const audio_effects& effects) {
        detail::check(crabby_ctx_set_sound_effects(raw_, p, &effects));
    }
    /// Plays a sound from a point; it follows the listener every frame.
    playing play_sound_at(sound s, vec3 position, const audio_settings3d& settings = audio_settings3d_defaults()) {
        playing p{}; detail::check(crabby_ctx_play_sound_at(raw_, s, position, &settings, &p)); return p;
    }
    /// Makes `e` (with a 3D transform) play `s` from where it is, following it.
    void set_audio_source3d(entity e, sound s, const audio_settings3d& settings = audio_settings3d_defaults()) {
        detail::check(crabby_ctx_set_audio_source3d(raw_, e, s, &settings));
    }
    void remove_audio_source3d(entity e) { detail::check(crabby_ctx_remove_audio_source3d(raw_, e)); }
    /// Puts the ears somewhere other than the camera; std::nullopt follows the camera again.
    void set_audio_listener(std::optional<audio_listener3d> l) {
        detail::check(crabby_ctx_set_audio_listener(raw_, l ? &*l : nullptr));
    }
    /// Where the ears are: the listener set above, or else the camera.
    audio_listener3d audio_listener() { audio_listener3d l{}; detail::check(crabby_ctx_audio_listener(raw_, &l)); return l; }

    /// Spreads work over `every` ticks (safe from query_par callbacks).
    crabby::turns turns(std::uint32_t every) const { crabby::turns t; detail::check(crabby_ctx_turns(raw_, every, &t.raw)); return t; }
    /// Textures reloaded from changed files at the start of this frame (hot
    /// reload). They keep their handles; redo anything computed from them.
    std::vector<texture> reloaded_textures() const {
        const CrabbyTexture* list = nullptr; std::size_t n = 0;
        detail::check(crabby_ctx_reloaded_textures(raw_, &list, &n));
        return std::vector<texture>(list, list + n);
    }

    // Commands: applied after the system; usable inside world().each (not each_par).
    void despawn_later(entity e) { detail::check(crabby_ctx_despawn_later(raw_, e)); }
    // (A type used for the first time inside a query can't be registered
    // yet; such commands and events are sent right after the system returns.)
    template<class... Ts> void spawn_later(const Ts&... components) {
        static_assert(!std::disjunction<std::is_same<Ts, polygon>...>::value, "spawn polygon colliders with world.spawn");
        const std::array<std::optional<std::uint32_t>, sizeof...(Ts)> ids{{find_id<Ts>()...}};
        if (!std::all_of(ids.begin(), ids.end(), [](const auto& id) { return id.has_value(); })) {
            after_system([=](ctx& c) { c.spawn_later(components...); });
            return;
        }
        std::size_t i = 0;
        std::array<CrabbyValue, sizeof...(Ts)> args{{CrabbyValue{*ids[i++], &components, sizeof(Ts)}...}};
        detail::check(crabby_ctx_spawn_later(raw_, args.data(), args.size()));
    }
    template<class T> void set_later(entity e, const T& component) {
        static_assert(!std::is_same<T, polygon>::value, "set polygon colliders with world.set");
        const auto id = find_id<T>();
        if (!id) { after_system([=](ctx& c) { c.set_later(e, component); }); return; }
        CrabbyValue v{*id, &component, sizeof(T)};
        detail::check(crabby_ctx_set_later(raw_, e, &v));
    }
    template<class T> void remove_later(entity e) {
        const auto id = find_id<T>();
        if (!id) { after_system([=](ctx& c) { c.remove_later<T>(e); }); return; }
        detail::check(crabby_ctx_remove_later(raw_, e, *id));
    }

    // Events: messages between systems, any plain struct. Usable inside
    // world().each (not each_par).
    template<class T> void send(const T& event) {
        static_assert(alignof(T) <= 16, "events can be at most 16-byte aligned");
        const auto id = find_id<T>();
        if (!id) { after_system([=](ctx& c) { c.send(event); }); return; }
        detail::check(crabby_ctx_send_event(raw_, *id, &event, sizeof(T)));
    }
    /// The events of type `T` this system hasn't read yet, oldest first;
    /// valid until the next send of this type or the end of the system.
    template<class T> events_view<T> read() {
        static_assert(alignof(T) <= 16, "events can be at most 16-byte aligned");
        const void* data = nullptr;
        std::size_t count = 0;
        const auto id = find_id<T>();
        if (!id) return events_view<T>(nullptr, 0);  // never used, so never sent
        detail::check(crabby_ctx_read_events(raw_, *id, sizeof(T), &data, &count));
        return events_view<T>(static_cast<const T*>(data), count);
    }

    // Physics (game::physics), as of the last physics step. Not inside query callbacks.
    void set_gravity(vec2 gravity) { detail::check(crabby_ctx_set_gravity(raw_, gravity)); }
    /// The first collider a ray from `origin` towards `direction` hits within `max_distance`.
    std::optional<ray_hit> cast_ray(vec2 origin, vec2 direction, float max_distance) {
        bool found = false;
        ray_hit hit{};
        detail::check(crabby_ctx_cast_ray(raw_, origin, direction, max_distance, &found, &hit));
        if (!found) return std::nullopt;
        return hit;
    }
    /// Common explicit-filter spelling; the old cast_ray call is unchanged.
    std::optional<ray_hit> cast_ray_filtered(vec2 origin, vec2 direction, float max_distance, filter2d filter) {
        bool found = false; ray_hit hit{};
        detail::check(crabby_ctx_cast_ray_filtered(raw_, origin, direction, max_distance, filter.raw, &found, &hit));
        if (!found) return std::nullopt;
        return hit;
    }
    std::optional<hit3d> cast_ray_filtered(vec3 origin, vec3 direction, float max_distance, filter3d filter) {
        return cast_ray3d(origin, direction, max_distance, filter);
    }
    /// A circle in 2D and a sphere in 3D, with matching argument order.
    void overlap_ball(vec2 center, float radius, filter2d filter, std::vector<CrabbyEntity>& out) {
        overlap3d(out, [&](CrabbyEntity* data, std::size_t cap, std::size_t* n) {
            return crabby_ctx_overlap_ball(raw_, center, radius, filter.raw, data, cap, n);
        });
    }
    void overlap_ball(vec3 center, float radius, filter3d filter, std::vector<CrabbyEntity>& out) {
        overlap_sphere3d(center, radius, out, filter);
    }
    /// Entities whose collider contains `point`, into `out`.
    void entities_at(vec2 point, std::vector<CrabbyEntity>& out) {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(crabby_ctx_physics_at(raw_, point, out.data(), out.size(), &n));
        if (n > out.size()) { out.resize(n); detail::check(crabby_ctx_physics_at(raw_, point, out.data(), out.size(), &n)); }
        out.resize(n);
    }
    /// Entities whose collider overlaps the circle, into `out`.
    void entities_in_circle(vec2 center, float radius, std::vector<CrabbyEntity>& out) {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(crabby_ctx_physics_in_circle(raw_, center, radius, out.data(), out.size(), &n));
        if (n > out.size()) { out.resize(n); detail::check(crabby_ctx_physics_in_circle(raw_, center, radius, out.data(), out.size(), &n)); }
        out.resize(n);
    }
    /// Pushes a dynamic body once (mass × pixels/s); false if `e` isn't a body.
    bool apply_impulse(entity e, vec2 impulse) {
        auto status = crabby_ctx_apply_impulse(raw_, e, impulse);
        if (status == CRABBY_NOT_FOUND) return false;
        detail::check(status);
        return true;
    }
    /// A body's mass (density × area), if `e` is a body.
    std::optional<float> mass(entity e) {
        float m = 0;
        auto status = crabby_ctx_body_mass(raw_, e, &m);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return m;
    }
    /// Pairs that started touching since this system last asked (sensor
    /// overlaps too), oldest first; valid until the system returns.
    events_view<collision> collisions_started() { return collisions(true); }
    /// Pairs that stopped touching since this system last asked.
    events_view<collision> collisions_stopped() { return collisions(false); }
private:
    events_view<collision> collisions(bool started) {
        const collision* data = nullptr;
        std::size_t count = 0;
        detail::check(crabby_ctx_collisions(raw_, started, &data, &count));
        return events_view<collision>(data, count);
    }
public:

    // 3D physics (game::physics3d), as of the last physics step. Not inside query callbacks.
    void set_gravity3d(vec3 gravity) { detail::check(crabby_ctx_set_gravity3d(raw_, gravity)); }
    /// A heightfield from the same n x n heights and `size` as
    /// create_terrain_tile (NaN for holes): give mesh_collider(it) to a
    /// fixed body at the tile's transform3d (its corner).
    collision_mesh heightfield_collider(const std::vector<float>& heights, float size) {
        collision_mesh m{}; detail::check(crabby_ctx_physics3d_heightfield(raw_, heights.data(), heights.size(), size, &m)); return m;
    }
    /// A convex hull around `points`.
    collision_mesh convex_hull_collider(const std::vector<vec3>& points) {
        collision_mesh m{}; detail::check(crabby_ctx_physics3d_convex_hull(raw_, points.data(), points.size(), &m)); return m;
    }
    /// A triangle mesh: three indices into `vertices` per triangle.
    collision_mesh trimesh_collider(const std::vector<vec3>& vertices, const std::vector<std::uint32_t>& indices) {
        collision_mesh m{};
        detail::check(crabby_ctx_physics3d_trimesh(raw_, vertices.data(), vertices.size(), indices.data(), indices.size(), &m));
        return m;
    }
    /// Lets go of a collision mesh; colliders using it keep it until removed.
    void free_collision_mesh(collision_mesh m) { detail::check(crabby_ctx_physics3d_free_mesh(raw_, m)); }
    /// A collision mesh from data built (on any thread) as collision_mesh_data.
    collision_mesh add_collision_mesh(const collision_mesh_data& data) {
        collision_mesh m{}; detail::check(crabby_ctx_physics3d_add_mesh(raw_, data.raw(), &m)); return m;
    }
    /// Collider streaming (Rust `Physics3D::stream_colliders`): with a
    /// number, the colliders of fixed bodies and terrain colliders are only
    /// in the physics world near entities with a collider_focus, at most
    /// that many more each step, nearest first; std::nullopt turns it off.
    void stream_colliders(std::optional<std::size_t> max_new) {
        detail::check(crabby_ctx_physics3d_stream_colliders(raw_, max_new.has_value(), max_new.value_or(0)));
    }
    /// Whether `e`'s collider is in the physics world now (false for a
    /// static collider that streaming left out).
    bool is_collider_active(entity e) const {
        bool active = false; detail::check(crabby_ctx_physics3d_is_collider_active(raw_, e, &active)); return active;
    }
    /// How many joints the 3D physics world holds after its last step.
    std::size_t joint_count() const { std::size_t n = 0; detail::check(crabby_ctx_physics3d_joint_count(raw_, &n)); return n; }
    /// Sets the contact hook (Rust `Physics3D::set_contact_hook`): each step,
    /// before the solver uses them, `hook` is called for the contacts of every
    /// body with `hook_modifier3d()` (or a modifier with `.hook = true`). It
    /// can ignore the pair or remove points, change friction and bounciness,
    /// or set a surface velocity; changes last for that step. It runs inside
    /// the physics step: it sees only the pair (capture the rest), must not
    /// use this ctx or its world, and the `contact_pair3d&` is valid only
    /// during the call. An exception it throws stops the game after the step
    /// and comes out of `run`/`run_headless`. Replaces an earlier hook.
    void set_contact_hook3d(std::function<void(contact_pair3d&)> hook) {
        if (!hook) { clear_contact_hook3d(); return; }
        owner_->contact_hook = std::move(hook);
        detail::check(crabby_ctx_set_contact_hook3d(raw_, contact_trampoline, owner_.get()));
    }
    /// Removes the contact hook.
    void clear_contact_hook3d() {
        detail::check(crabby_ctx_set_contact_hook3d(raw_, nullptr, nullptr));
        owner_->contact_hook = nullptr;
    }
    /// The closest collider a ray hits within `max_distance`.
    std::optional<hit3d> cast_ray3d(vec3 origin, vec3 direction, float max_distance, filter3d filter = {}) {
        bool found = false; hit3d hit{};
        detail::check(crabby_ctx_cast_ray3d(raw_, origin, direction, max_distance, filter.raw, &found, &hit));
        if (!found) return std::nullopt;
        return hit;
    }
    /// Every collider a ray hits within `max_distance`, nearest first, into `out`.
    void cast_ray3d_all(vec3 origin, vec3 direction, float max_distance, std::vector<hit3d>& out, filter3d filter = {}) {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(crabby_ctx_cast_ray3d_all(raw_, origin, direction, max_distance, filter.raw, out.data(), out.size(), &n));
        if (n > out.size()) {
            out.resize(n);
            detail::check(crabby_ctx_cast_ray3d_all(raw_, origin, direction, max_distance, filter.raw, out.data(), out.size(), &n));
        }
        out.resize(n);
    }
    /// Sweeps a ball: the first collider it touches within `max_distance`.
    std::optional<hit3d> cast_sphere3d(vec3 origin, float radius, vec3 direction, float max_distance, filter3d filter = {}) {
        bool found = false; hit3d hit{};
        detail::check(crabby_ctx_cast_sphere3d(raw_, origin, radius, direction, max_distance, filter.raw, &found, &hit));
        if (!found) return std::nullopt;
        return hit;
    }
    /// Sweeps an upright capsule centered on `origin`.
    std::optional<hit3d> cast_capsule3d(vec3 origin, float height, float radius, vec3 direction, float max_distance,
                                        filter3d filter = {}) {
        bool found = false; hit3d hit{};
        detail::check(crabby_ctx_cast_capsule3d(raw_, origin, height, radius, direction, max_distance, filter.raw, &found, &hit));
        if (!found) return std::nullopt;
        return hit;
    }
    /// Entities whose collider overlaps the ball, into `out`.
    void overlap_sphere3d(vec3 center, float radius, std::vector<CrabbyEntity>& out, filter3d filter = {}) {
        overlap3d(out, [&](CrabbyEntity* data, std::size_t cap, std::size_t* n) {
            return crabby_ctx_overlap_sphere3d(raw_, center, radius, filter.raw, data, cap, n);
        });
    }
    /// Entities whose collider overlaps the box of `size` turned by `rotation`.
    void overlap_box3d(vec3 center, vec3 size, quat rotation, std::vector<CrabbyEntity>& out, filter3d filter = {}) {
        overlap3d(out, [&](CrabbyEntity* data, std::size_t cap, std::size_t* n) {
            return crabby_ctx_overlap_box3d(raw_, center, size, rotation, filter.raw, data, cap, n);
        });
    }
    /// Entities whose collider overlaps the upright capsule.
    void overlap_capsule3d(vec3 center, float height, float radius, std::vector<CrabbyEntity>& out, filter3d filter = {}) {
        overlap3d(out, [&](CrabbyEntity* data, std::size_t cap, std::size_t* n) {
            return crabby_ctx_overlap_capsule3d(raw_, center, height, radius, filter.raw, data, cap, n);
        });
    }
    /// Pushes a dynamic 3D body once (kg × m/s); false if `e` isn't one.
    bool apply_impulse3d(entity e, vec3 impulse) {
        auto status = crabby_ctx_apply_impulse3d(raw_, e, impulse);
        if (status == CRABBY_NOT_FOUND) return false;
        detail::check(status);
        return true;
    }
    /// A 3D body's mass in kg, if `e` is one.
    std::optional<float> mass3d(entity e) {
        float m = 0;
        auto status = crabby_ctx_body_mass3d(raw_, e, &m);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return m;
    }
    /// A 3D body's center of mass in the world, if `e` is one.
    std::optional<vec3> center_of_mass3d(entity e) {
        vec3 c{};
        auto status = crabby_ctx_body_center_of_mass3d(raw_, e, &c);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return c;
    }
    /// Hands `e`'s skeleton over to physics (Rust `Ctx::start_ragdoll`): a
    /// body per bone, jointed to the nearest chosen ancestor. The bodies are
    /// linked at once, so you can push them straight away. Throws on an
    /// unknown or repeated joint, a radius that isn't positive, or no skeleton.
    void start_ragdoll(entity e, const std::vector<ragdoll_bone>& bones) {
        std::vector<CrabbyRagdollBone> raw;
        raw.reserve(bones.size());
        for (const ragdoll_bone& b : bones) raw.push_back(b.raw());
        detail::check(crabby_ctx_start_ragdoll(raw_, e, raw.data(), raw.size()));
    }
    /// Removes `e`'s ragdoll bodies and blends its skeleton back to the
    /// animated pose over `seconds` (0 snaps). Call it before despawning `e`.
    void stop_ragdoll(entity e, float seconds = 0.0f) { detail::check(crabby_ctx_stop_ragdoll(raw_, e, seconds)); }
    /// The body standing in for joint `joint` (see find_joint) of a ragdolled `e`.
    std::optional<entity> ragdoll_body(entity e, std::size_t joint) const {
        entity body = 0;
        auto status = crabby_ctx_ragdoll_body(raw_, e, joint, &body);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return body;
    }
    /// Whether `e` is ragdolled (or still blending back from it).
    bool ragdolled(entity e) const { bool r = false, b = false; detail::check(crabby_ctx_ragdoll_state(raw_, e, &r, &b)); return r; }
    /// Whether `e` is blending back to its animation after stop_ragdoll.
    bool ragdoll_blending(entity e) const { bool r = false, b = false; detail::check(crabby_ctx_ragdoll_state(raw_, e, &r, &b)); return b; }
private:
    template<class F> static void overlap3d(std::vector<CrabbyEntity>& out, F call) {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(call(out.data(), out.size(), &n));
        if (n > out.size()) { out.resize(n); detail::check(call(out.data(), out.size(), &n)); }
        out.resize(n);
    }
public:

    // The built-in spatial grid (game::spatial_grid); safe from query_par callbacks.
    void spatial_radius(vec2 center, float radius, std::vector<grid_hit>& out) const {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(crabby_ctx_spatial_radius(raw_, center, radius, out.data(), out.size(), &n));
        if (n > out.size()) { out.resize(n); detail::check(crabby_ctx_spatial_radius(raw_, center, radius, out.data(), out.size(), &n)); }
        out.resize(n);
    }
    void spatial_rect(vec2 min, vec2 max, std::vector<grid_hit>& out) const {
        std::size_t n = 0;
        out.resize(out.capacity());
        detail::check(crabby_ctx_spatial_rect(raw_, min, max, out.data(), out.size(), &n));
        if (n > out.size()) { out.resize(n); detail::check(crabby_ctx_spatial_rect(raw_, min, max, out.data(), out.size(), &n)); }
        out.resize(n);
    }
    /// An inline-query view of the built-in grid, valid during this call.
    grid_view<grid_hit> spatial_view() const { CrabbyGridView v{}; detail::check(crabby_ctx_spatial_view(raw_, &v)); return grid_view<grid_hit>(v); }
    std::optional<grid_hit> spatial_nearest(vec2 center, float max_radius, entity exclude = UINT64_MAX) const {
        grid_hit hit{};
        auto status = crabby_ctx_spatial_nearest(raw_, center, max_radius, exclude, &hit);
        if (status == CRABBY_NOT_FOUND) return std::nullopt;
        detail::check(status);
        return hit;
    }
};

/// Low-detail ground for the whole world beyond the loaded terrain tiles
/// (Rust `FarTerrain`): a few large meshes from a coarse grid of heights,
/// cut away a cell at a time where the cell's own tile is shown, with
/// skirts so there's no crack.
class far_terrain {
    std::unique_ptr<CrabbyFarTerrain, void (*)(CrabbyFarTerrain*)> raw_;
    static CrabbyFarTerrain* make(const std::vector<float>& heights, std::size_t columns, vec2 corner, float spacing,
                                  float cell_size, material m) {
        CrabbyFarTerrain* f = crabby_far_terrain_new(heights.data(), heights.size(), columns, corner, spacing, cell_size, m);
        if (!f) throw std::invalid_argument(crabby_last_error());
        return f;
    }
public:
    /// `heights`, `columns` per row (row after row along +Z, NaN for a
    /// hole), the first at `corner` (x, z) and the rest `spacing` apart,
    /// drawn with `m`; cut a cell at a time for tiles `cell_size` wide (a
    /// whole number of spacings; `corner` on the grid of spacings).
    far_terrain(const std::vector<float>& heights, std::size_t columns, vec2 corner, float spacing, float cell_size, material m)
        : raw_(make(heights, columns, corner, spacing, cell_size, m), crabby_far_terrain_free) {}
    /// Cuts the far ground away over `c`, whose own tile is now shown.
    void cut(cell c) { detail::check(crabby_far_terrain_cut(raw_.get(), c, true)); }
    /// Fills `c` with far ground again, when its tile goes.
    void uncut(cell c) { detail::check(crabby_far_terrain_cut(raw_.get(), c, false)); }
    bool is_cut(cell c) const {
        bool cut = false; detail::check(crabby_far_terrain_is_cut(raw_.get(), c, &cut, nullptr)); return cut;
    }
    /// How many meshes (and entities) the far ground has in the world.
    std::size_t mesh_count() const {
        bool cut = false; std::size_t n = 0;
        detail::check(crabby_far_terrain_is_cut(raw_.get(), cell{0, 0}, &cut, &n)); return n;
    }
    /// Builds the meshes that changed since the last update (all of them
    /// the first time) and puts them in the world.
    void update(ctx& c) { detail::check(crabby_far_terrain_update(raw_.get(), c.raw())); }
    /// Despawns its entities and frees its meshes; the next update builds them again.
    void clear(ctx& c) { detail::check(crabby_far_terrain_clear(raw_.get(), c.raw())); }
};

namespace detail {
/// Whether plugin type `P` has a `setup(ctx&)` member.
template<class P, class = void> struct has_setup : std::false_type {};
template<class P>
struct has_setup<P, std::void_t<decltype(std::declval<P&>().setup(std::declval<ctx&>()))>> : std::true_type {};
} // namespace detail

/// The game builder (Rust `Game`). Add systems, then run() or run_headless().
class game {
    struct state {
        std::shared_ptr<detail::identity> owner = std::make_shared<detail::identity>();
        detail::first_error error;
    };
    struct holder {
        std::function<void(ctx&)> fn;
        state* st;
    };
    CrabbyGame* raw_;
    std::unique_ptr<state> state_ = std::make_unique<state>();
    std::vector<std::unique_ptr<holder>> systems_;

    static void trampoline(CrabbyCtx* raw, void* user) noexcept {
        auto& h = *static_cast<holder*>(user);
        if (h.st->error.set.load()) return;
        try {
            ctx c(raw, h.st->owner); h.fn(c); c.run_after_system();
            if (h.st->owner->hook_error.set.load()) std::rethrow_exception(h.st->owner->hook_error.error);
        }
        catch (...) { h.st->error.capture(); (void)crabby_ctx_quit(raw); }
    }
    holder* keep(std::function<void(ctx&)> fn) {
        systems_.push_back(std::make_unique<holder>(holder{std::move(fn), state_.get()}));
        return systems_.back().get();
    }
    CrabbyGame* take() {
        if (!raw_) throw std::logic_error("game was already run");
        CrabbyGame* g = raw_; raw_ = nullptr; return g;
    }
    game& check(CrabbyStatus s) { if (!raw_) throw std::logic_error("game was already run"); detail::check(s); return *this; }
public:
    game(const char* title, std::uint32_t width, std::uint32_t height) : raw_(crabby_game_new(title, width, height)) {
        if (!raw_) throw std::runtime_error(crabby_last_error());
    }
    ~game() { crabby_game_free(raw_); }
    game(const game&) = delete;
    game& operator=(const game&) = delete;

    game& fixed_hz(std::uint32_t hz) { return check(crabby_game_fixed_hz(raw_, hz)); }
    game& seed(std::uint64_t seed) { return check(crabby_game_seed(raw_, seed)); }
    game& vsync(bool on) { return check(crabby_game_vsync(raw_, on)); }
    /// Samples per pixel of the 3D frame: 4 (default) or 2.
    game& msaa(std::uint32_t samples) { return check(crabby_game_msaa(raw_, samples)); }
    /// Fill the screen instead of opening a window.
    game& fullscreen(bool on) { return check(crabby_game_fullscreen(raw_, on)); }
    game& spatial_grid(float cell_size) { return check(crabby_game_spatial_grid(raw_, cell_size)); }
    game& control_endpoint(bool on) { return check(crabby_game_control_endpoint(raw_, on)); }
    /// Reload textures when their files change, under the same handles (on by
    /// default in debug builds of the engine; CRABBY_HOT_RELOAD=1/0 overrides).
    game& hot_reload(bool on) { return check(crabby_game_hot_reload(raw_, on)); }
    /// The runtime inspector, opened with the backquote key (on by default in
    /// debug builds of the engine; CRABBY_INSPECTOR=1/0/open overrides).
    game& inspector(bool on) { return check(crabby_game_inspector(raw_, on)); }
    /// The built-in camera controls: right/middle drag pans, the wheel zooms.
    game& camera_controls() { return check(crabby_game_camera_controls(raw_)); }
    /// Adds the built-in system that moves attached entities next to the
    /// entity they follow (and despawns them when it's gone). Add it after
    /// the sim systems that move things.
    game& follow_attachments() { return check(crabby_game_follow_attachments(raw_)); }
    /// Turns on 2D physics with `gravity` in pixels/s² (y is down): entities
    /// with a rigid_body, a collider and a transform2d are simulated. The step
    /// runs in phase::physics wherever this is called: add systems that feed
    /// it with add(phase::before_physics, ...); sim_system runs after it.
    game& physics(vec2 gravity) { return check(crabby_game_physics(raw_, gravity)); }
    /// Turns on 3D physics with `gravity` in m/s² (+Y up): entities with a
    /// rigid_body3d, a collider3d and a transform3d, and characters, are
    /// simulated at 50 steps a second (unless fixed_hz chose a rate), and
    /// drawn blended between steps. The step runs in phase::physics wherever
    /// this is called; feed it from add(phase::before_physics, ...).
    game& physics3d(vec3 gravity) { return check(crabby_game_physics3d(raw_, gravity)); }
    /// Hosts a lockstep session for `players` (2-8; you are player 0) on
    /// `addr` ("0.0.0.0:7777"; port 0 picks one, written to `*port`). Input
    /// is scheduled `input_delay` ticks ahead; worlds are compared every
    /// `checksum_every` ticks. Ticks run once every player's input is in.
    game& lockstep_host(const char* addr, std::uint32_t players, std::uint32_t input_delay = 3,
                        std::uint32_t checksum_every = 60, std::uint16_t* port = nullptr) {
        return check(crabby_game_lockstep_host(raw_, addr, players, input_delay, checksum_every, port));
    }
    /// Joins the lockstep session hosted at `addr` ("192.168.1.20:7777").
    game& lockstep_join(const char* addr) { return check(crabby_game_lockstep_join(raw_, addr)); }
    /// Hosts a lockstep session over Steam (ponko_ffi feature `steam`):
    /// starts Steam for `app_id` (480: Valve's test app) and opens a
    /// friends-only lobby; friends join through invites (ctx::steam_invite).
    game& lockstep_steam_host(std::uint32_t app_id, std::uint32_t players, std::uint32_t input_delay = 3,
                              std::uint32_t checksum_every = 60) {
        return check(crabby_game_lockstep_steam_host(raw_, app_id, players, input_delay, checksum_every));
    }
    /// Joins a Steam lobby (from steam_lobby_from_args or steam_wait_for_invite).
    game& lockstep_steam_join(std::uint32_t app_id, std::uint64_t lobby) {
        return check(crabby_game_lockstep_steam_join(raw_, app_id, lobby));
    }
    /// Moves navmesh agents every simulation tick, at this point in the order.
    game& navmesh3d() { return check(crabby_game_navmesh3d(raw_)); }
    /// A system on the fixed simulation tick; `every` > 1 runs it every n ticks.
    template<class F> game& sim_system(F fn, const char* name = nullptr, std::uint32_t every = 1) {
        return check(crabby_game_sim_system(raw_, name, every, trampoline, keep(std::move(fn))));
    }
    /// A system in an explicit phase; e.g. phase::before_physics for logic the
    /// current tick's physics step must see. Fixed-tick phases take `every` > 1.
    template<class F> game& add(phase p, F fn, const char* name = nullptr, std::uint32_t every = 1) {
        return check(crabby_game_add_system(raw_, static_cast<std::uint32_t>(p), name, every, trampoline,
                                            keep(std::move(fn))));
    }
    /// A system in an explicit phase with a declaration of what it reads and
    /// writes (see crabby::access). Debug builds stop the game if it uses
    /// anything undeclared. It still runs in order, like any other system.
    template<class F> game& add(phase p, const access& a, F fn, const char* name = nullptr, std::uint32_t every = 1) {
        return check(crabby_game_add_system_with_access(raw_, static_cast<std::uint32_t>(p), name, every, a.raw(),
                                                        trampoline, keep(std::move(fn))));
    }
    /// One line per pair of declared systems in the same phase: whether
    /// they could run together, or what they conflict on.
    std::string access_report() {
        return detail::read_text([&](char* b, std::size_t c, std::size_t* l) { return crabby_game_access_report(raw_, b, c, l); });
    }
    /// A system once per frame (input, camera, UI). Not run headless.
    template<class F> game& system(F fn, const char* name = nullptr) {
        return check(crabby_game_system(raw_, name, trampoline, keep(std::move(fn))));
    }
    /// A system that runs every frame after animation posed the skeletons, before drawing (IK, look-at).
    template<class F> game& late_system(F fn, const char* name = nullptr) {
        return check(crabby_game_late_system(raw_, name, trampoline, keep(std::move(fn))));
    }
    /// Like add(), and returns the system's id for remove(). Two systems
    /// made from the same function get different ids.
    template<class F> system_id add_system(phase p, F fn, const char* name = nullptr, std::uint32_t every = 1) {
        system_id id;
        check(crabby_game_add_system_id(raw_, static_cast<std::uint32_t>(p), name, every, trampoline,
                                        keep(std::move(fn)), &id.bits));
        return id;
    }
    /// Removes systems by id: one from add_system, or a built-in such as
    /// systems::draw_sprites (every instance of it). An id that names
    /// nothing is not an error.
    game& remove(system_id id) { return check(crabby_game_remove(raw_, id.bits)); }
    /// Removes systems by name, e.g. "ponko::ecs::draw_sprites". Kept for
    /// existing code: prefer remove(systems::draw_sprites).
    game& remove_system(const char* name) { return check(crabby_game_remove_system(raw_, name)); }
    /// A startup callback, `setup(ctx&)`: runs once after the engine started
    /// and before any system (also with zero ticks). Callbacks and plugin
    /// setups run in the order they were added, before the one passed to
    /// run(); commands are applied after each.
    template<class S> game& setup(S fn) { return check(crabby_game_setup(raw_, trampoline, keep(std::move(fn)))); }
    /// Adds a module of the game (Rust `Plugin`): any object with
    /// `void build(ponko::game&)`, which adds its systems and settings now,
    /// and optionally `void setup(ponko::ctx&)`, queued like setup(). Each
    /// system names its phase, so the order plugins are added in only
    /// matters between systems of the same phase.
    template<class P> game& plugin(P p) {
        if (!raw_) throw std::logic_error("game was already run");
        auto shared = std::make_shared<P>(std::move(p));
        shared->build(*this);
        if constexpr (detail::has_setup<P>::value) setup([shared](ctx& c) { shared->setup(c); });
        return *this;
    }

    /// Opens the window and runs until quit. `setup(ctx&)` runs once first.
    template<class S> void run(S setup) {
        holder* h = keep(std::move(setup));
        auto status = crabby_game_run(take(), trampoline, h);
        state_->error.rethrow();
        state_->owner->hook_error.rethrow();
        detail::check(status);
    }
    void run() { run([](ctx&) {}); }
    /// Runs up to `ticks` ticks with no window or GPU and returns the world.
    template<class S> crabby::world run_headless(std::uint64_t ticks, S setup) {
        holder* h = keep(std::move(setup));
        CrabbyWorld* result = nullptr;
        auto status = crabby_game_run_headless(take(), ticks, trampoline, h, &result);
        crabby::world owned(result, state_->owner);
        state_->error.rethrow();
        state_->owner->hook_error.rethrow();
        detail::check(status);
        return owned;
    }
    crabby::world run_headless(std::uint64_t ticks) { return run_headless(ticks, [](ctx&) {}); }

    /// What profile_headless returns.
    struct profiled {
        crabby::world world;
        crabby::perf_report report;
    };
    /// Like run_headless, but timed: the first `warmup` ticks aren't measured,
    /// the next `ticks` are. The report says where the time went (systems,
    /// CRABBY_PROFILE_SCOPE scopes), how many allocations each tick made, the
    /// slowest ticks and what to try. For tests and CI: see perf_report::check.
    template<class S> profiled profile_headless(std::uint64_t warmup, std::uint64_t ticks, S setup) {
        holder* h = keep(std::move(setup));
        CrabbyWorld* result = nullptr;
        auto status = crabby_game_profile_headless(take(), warmup, ticks, trampoline, h, &result);
        crabby::world owned(result, state_->owner);
        state_->error.rethrow();
        state_->owner->hook_error.rethrow();
        detail::check(status);
        return profiled{std::move(owned), crabby::perf_report(detail::read_text([](char* b, std::size_t c, std::size_t* n) { return crabby_last_perf_report(b, c, n); }))};
    }
};

/// The lobby of the Steam invite the game was launched with, or 0.
inline std::uint64_t steam_lobby_from_args() {
    std::uint64_t l = 0;
    detail::check(crabby_steam_lobby_from_args(&l));
    return l;
}
/// Waits up to `timeout_ms` for the player to accept a friend's Steam
/// invite; the lobby, or 0. Starts Steam for `app_id`.
inline std::uint64_t steam_wait_for_invite(std::uint32_t app_id, std::uint32_t timeout_ms) {
    std::uint64_t l = 0;
    detail::check(crabby_steam_wait_for_invite(app_id, timeout_ms, &l));
    return l;
}
} // namespace crabby

#define CRABBY_PP_CAT2(a, b) a##b
#define CRABBY_PP_CAT(a, b) CRABBY_PP_CAT2(a, b)
/// Times the rest of the block as a scope called `name` (a string literal).
/// It shows up next to the systems in the overlay, in CRABBY_BENCH reports and
/// in ctx::perf_report(). Scopes nest, work on any thread (times add up), and
/// cost one atomic load while profiling is off.
///
///     void ai(crabby::ctx& ctx) {
///         CRABBY_PROFILE_SCOPE("ai");
///         { CRABBY_PROFILE_SCOPE("ai/pathing"); /* ... */ }
///     }
#define CRABBY_PROFILE_SCOPE(name) \
    static const CrabbyProfileScope* const CRABBY_PP_CAT(crabby_scope_, __LINE__) = crabby_profile_scope(name); \
    ::crabby::profile_scope_guard CRABBY_PP_CAT(crabby_scope_guard_, __LINE__)(CRABBY_PP_CAT(crabby_scope_, __LINE__))

/// Put this once, at global scope in one .cpp file, to count C++ heap
/// allocations (operator new) per frame and per system in the reports. It
/// replaces the global operator new/delete; counting happens only while
/// profiling is on, so the cost otherwise is one atomic load per `new`.
#define CRABBY_COUNT_CPP_ALLOCATIONS \
    void* operator new(std::size_t n) { crabby_note_allocation(n); if (void* p = std::malloc(n ? n : 1)) return p; throw std::bad_alloc(); } \
    void* operator new[](std::size_t n) { crabby_note_allocation(n); if (void* p = std::malloc(n ? n : 1)) return p; throw std::bad_alloc(); } \
    void operator delete(void* p) noexcept { std::free(p); } \
    void operator delete[](void* p) noexcept { std::free(p); } \
    void operator delete(void* p, std::size_t) noexcept { std::free(p); } \
    void operator delete[](void* p, std::size_t) noexcept { std::free(p); }
#endif
