// Crabby Engine for C++17: a header-only wrapper over crabby.h.
//
// Mirrors the Rust API: crabby::game is Game, crabby::ctx is Ctx,
// crabby::world_view is World. Errors become exceptions; exceptions thrown by
// your systems and callbacks are caught before they reach the engine and
// rethrown from run()/run_headless()/query().
#ifndef CRABBY_HPP
#define CRABBY_HPP
#include "crabby.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <climits>
#include <cmath>
#include <cstdint>
#include <exception>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <string>
#include <initializer_list>
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

namespace crabby {
using entity = std::uint64_t;
using vec2 = CrabbyVec2;
using color = CrabbyColor;
using texture = CrabbyTexture;
using transform2d = CrabbyTransform2D;
using sprite = CrabbySprite;
using camera = CrabbyCamera;
using draw_params = CrabbyDrawParams;
using grid_hit = CrabbyGridHit;
using grid_item = CrabbyGridItem;
using key = CrabbyKey;
using mouse = CrabbyMouseButton;
using rect = CrabbyRect;
using font = CrabbyFont;
using text_params = CrabbyTextParams;
using ui_style = CrabbyUiStyle;
using sound = CrabbySound;
using playing = CrabbyPlaying;
using sound_params = CrabbySoundParams;
using sprite_sheet = CrabbySpriteSheet;
using animation = CrabbyAnimation;
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
inline entity bits(CrabbyEntity e) { return crabby_entity_bits(e); }

inline color rgb(float r, float g, float b) { return {r, g, b, 1}; }
inline color rgba(float r, float g, float b, float a) { return {r, g, b, a}; }
namespace colors {
constexpr color white{1, 1, 1, 1}, black{0, 0, 0, 1}, gray{.5f, .5f, .5f, 1}, red{1, 0, 0, 1},
    green{0, 1, 0, 1}, blue{0, 0, 1, 1}, yellow{1, 1, 0, 1}, transparent{0, 0, 0, 0}, crabby_dark{.11f, .12f, .15f, 1};
}
/// A transform at `position`, no rotation, scale 1.
inline transform2d transform_at(vec2 position, float rotation = 0, vec2 scale = {1, 1}) { return {position, rotation, scale}; }
/// A sprite showing all of `t`, centered, white, z 0.
inline sprite sprite_of(texture t, color tint = colors::white, float z = 0) {
    sprite s{}; s.texture = t; s.color = tint; s.z = z; s.pivot = {.5f, .5f}; return s;
}
inline draw_params draw_defaults() { return crabby_draw_params_default(); }
/// Built-in font, 16 px, white.
inline text_params text_defaults() { return crabby_text_params_default(); }
/// Full volume, normal speed, once.
inline sound_params sound_defaults() { return crabby_sound_params_default(); }
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

namespace detail {
struct identity { std::unordered_map<std::uint32_t, std::type_index> types; };
inline void check(CrabbyStatus status) {
    if (status != CRABBY_OK) throw std::runtime_error(crabby_last_error());
}
template<class T> constexpr bool pod = std::is_trivially_copyable<T>::value &&
    std::is_trivially_destructible<T>::value && std::is_standard_layout<T>::value;
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
class world_view;
class ctx;
template<class T> class component;
// Owns the copied value until spawn returns, including values made from temporaries.
template<class T> struct value {
    component<T> token;
    T data;
};
// A token belongs to one world (or game); it remains valid when that world is moved.
template<class T> class component {
    static_assert(detail::pod<T>, "components must be trivial, standard-layout values");
    std::uint32_t id_ = 0;
    std::shared_ptr<detail::identity> owner_;
    component(std::uint32_t id, std::shared_ptr<detail::identity> owner) : id_(id), owner_(std::move(owner)) {}
    friend class world_view;
    friend class ctx;
public:
    /// An unset token; assign one from world_view::component before use.
    component() = default;
    value<T> with(T data) const { return {*this, data}; }
};
// Borrowed columns are valid only for the duration of the query callback.
template<class... Ts> class chunk {
    const CrabbyChunk* raw_;
public:
    explicit chunk(const CrabbyChunk* raw) : raw_(raw) {}
    std::size_t size() const noexcept { return raw_->count; }
    template<std::size_t I> auto column() const noexcept {
        using T = typename std::tuple_element<I, std::tuple<Ts...>>::type;
        return static_cast<T*>(raw_->columns[I]);
    }
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
// A non-owning view. Views given to systems must not escape their call.
class world_view {
    friend class world;
    friend class ctx;
protected:
    CrabbyWorld* raw_;
    std::shared_ptr<detail::identity> owner_;
    world_view(CrabbyWorld* raw, std::shared_ptr<detail::identity> owner) : raw_(raw), owner_(std::move(owner)) {}
    void valid() const { if (!raw_) throw std::logic_error("world has been moved from"); }
    template<class T> std::uint32_t id(const crabby::component<T>& token) const {
        valid();
        if (!token.owner_) throw std::invalid_argument("component token is unset");
        if (token.owner_ != owner_) throw std::invalid_argument("component belongs to another world");
        return token.id_;
    }
    template<class T> CrabbyValue c_value(const value<T>& v) const { return {id(v.token), &v.data, sizeof(T)}; }
    template<class T> crabby::component<T> token(std::uint32_t id) {
        const std::type_index type(typeid(T));
        const auto registered = owner_->types.emplace(id, type);
        if (!registered.second && registered.first->second != type)
            throw std::invalid_argument("component name is registered for a different C++ type");
        return crabby::component<T>(id, owner_);
    }
    template<class F, class... Ts, class Run> void run_query(Run run, F&& fn, const crabby::component<Ts>&... tokens) {
        valid(); std::array<std::uint32_t, sizeof...(Ts)> ids{{id(tokens)...}};
        using function = typename std::remove_reference<F>::type;
        struct context { function* fn; detail::first_error error; } c{&fn, {}};
        auto trampoline = [](const CrabbyChunk* raw, void* user) noexcept {
            auto& c = *static_cast<context*>(user);
            if (c.error.set.load()) return;
            try { (*c.fn)(chunk<Ts...>(raw)); } catch (...) { c.error.capture(); }
        };
        auto status = run(raw_, ids.data(), ids.size(), trampoline, &c);
        c.error.rethrow();
        detail::check(status);
    }
public:
    template<class T> crabby::component<T> component(const char* name) {
        static_assert(detail::pod<T>, "components must be trivial, standard-layout values");
        valid(); std::uint32_t result = 0;
        detail::check(crabby_component_register(raw_, name, sizeof(T), alignof(T), &result));
        return token<T>(result);
    }
    /// Registers `T` and describes its fields, so tools and agents can read
    /// and write it as JSON (`crabby-mcp`) and `save()` includes it.
    template<class T> crabby::component<T> component(const char* name, std::initializer_list<field_info<T>> fields) {
        auto token = component<T>(name);
        std::vector<CrabbyField> raw;
        raw.reserve(fields.size());
        for (const auto& f : fields) raw.push_back(f.raw);
        detail::check(crabby_component_reflect(raw_, token.id_, raw.data(), raw.size()));
        return token;
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
    /// The built-in Transform2D component.
    crabby::component<transform2d> transforms() {
        valid(); std::uint32_t result = 0;
        detail::check(crabby_transform_component(raw_, &result));
        return token<transform2d>(result);
    }
    /// The built-in Sprite component (get/set/spawn only, not a query column).
    crabby::component<sprite> sprites() {
        valid(); std::uint32_t result = 0;
        detail::check(crabby_sprite_component(raw_, &result));
        return token<sprite>(result);
    }
    /// The built-in Animation component (crabby::animation), a query column too.
    crabby::component<animation> animations() {
        valid(); std::uint32_t result = 0;
        detail::check(crabby_animation_component(raw_, &result));
        return token<animation>(result);
    }
    /// Gives `e` an empty `width` x `height` tilemap (its Transform2D is the top-left).
    tilemap_ref add_tilemap(entity e, const sprite_sheet& sheet, std::uint32_t width, std::uint32_t height);
    /// `e`'s tilemap; throws if it has none. Valid until `e` loses its map.
    tilemap_ref tilemap(entity e);
    template<class... Ts> entity spawn(const value<Ts>&... values) {
        valid(); std::array<CrabbyValue, sizeof...(Ts)> args{{c_value(values)...}};
        entity result = 0;
        detail::check(crabby_spawn(raw_, args.data(), args.size(), &result));
        return result;
    }
    void despawn(entity e) { valid(); detail::check(crabby_despawn(raw_, e)); }
    bool alive(entity e) const { valid(); bool result = false; detail::check(crabby_is_alive(raw_, e, &result)); return result; }
    std::size_t size() const { valid(); std::size_t result = 0; detail::check(crabby_world_len(raw_, &result)); return result; }
    template<class T> T get(entity e, const crabby::component<T>& token) const {
        T result{}; detail::check(crabby_get(raw_, e, id(token), &result, sizeof(T))); return result;
    }
    template<class T> void set(entity e, const crabby::component<T>& token, const T& data) {
        detail::check(crabby_set(raw_, e, id(token), &data, sizeof(T)));
    }
    template<class T> void remove(entity e, const crabby::component<T>& token) {
        detail::check(crabby_remove(raw_, e, id(token)));
    }
    /// Calls `fn(chunk)` for every archetype with all the given components.
    template<class F, class... Ts> void query(F&& fn, const crabby::component<Ts>&... tokens) {
        run_query(crabby_query, std::forward<F>(fn), tokens...);
    }
    /// Like query, but on all CPU cores at once: `fn` must be thread-safe and
    /// only touch its own chunk (plus ctx::dt/random and spatial queries).
    template<class F, class... Ts> void query_par(F&& fn, const crabby::component<Ts>&... tokens) {
        run_query(crabby_query_par, std::forward<F>(fn), tokens...);
    }
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
class ctx {
    CrabbyCtx* raw_;
    std::shared_ptr<detail::identity> owner_;
    template<class T> CrabbyValue c_value(const value<T>& v) const { return {world().id(v.token), &v.data, sizeof(T)}; }
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
    vec2 window_size() const noexcept { return crabby_ctx_window_size(raw_); }
    crabby::random random() const noexcept { return crabby::random(crabby_ctx_random_bits(raw_)); }

    // Simulation controls.
    void quit() { detail::check(crabby_ctx_quit(raw_)); }
    void pause() { detail::check(crabby_ctx_pause(raw_)); }
    void resume() { detail::check(crabby_ctx_resume(raw_)); }
    void step() { detail::check(crabby_ctx_step(raw_)); }
    void set_sim_speed(float speed) { detail::check(crabby_ctx_set_sim_speed(raw_, speed)); }
    float sim_speed() const { float s = 0; detail::check(crabby_ctx_sim_speed(raw_, &s)); return s; }
    bool is_paused() const { bool p = false; detail::check(crabby_ctx_is_paused(raw_, &p)); return p; }

    // Input.
    bool key_down(key k) const { bool b = false; detail::check(crabby_ctx_key_down(raw_, k, &b)); return b; }
    bool key_pressed(key k) const { bool b = false; detail::check(crabby_ctx_key_pressed(raw_, k, &b)); return b; }
    bool key_released(key k) const { bool b = false; detail::check(crabby_ctx_key_released(raw_, k, &b)); return b; }
    bool mouse_down(mouse m) const { bool b = false; detail::check(crabby_ctx_mouse_down(raw_, m, &b)); return b; }
    bool mouse_pressed(mouse m) const { bool b = false; detail::check(crabby_ctx_mouse_pressed(raw_, m, &b)); return b; }
    bool mouse_released(mouse m) const { bool b = false; detail::check(crabby_ctx_mouse_released(raw_, m, &b)); return b; }
    vec2 mouse_pos() const { vec2 v{}; detail::check(crabby_ctx_mouse_pos(raw_, &v)); return v; }
    vec2 mouse_world() const { vec2 v{}; detail::check(crabby_ctx_mouse_world(raw_, &v)); return v; }
    vec2 mouse_wheel() const { vec2 v{}; detail::check(crabby_ctx_mouse_wheel(raw_, &v)); return v; }

    // Textures and drawing.
    /// Loads a JSON file from assets/ into a reflected component of `e`
    /// (fields merge); with hot reload on, edits to the file show up live.
    template<class T> void load_json(const char* path, entity e, const component<T>& token) {
        detail::check(crabby_ctx_load_json(raw_, path, e, world().id(token)));
    }
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
    void save_screenshot(const char* path) { detail::check(crabby_ctx_save_screenshot(raw_, path)); }
    /// texture_filter::nearest keeps pixel art square.
    void set_texture_filter(texture t, texture_filter f) {
        detail::check(crabby_ctx_set_texture_filter(raw_, t, f == texture_filter::nearest));
    }

    // Shapes (world space).
    void draw_rect(rect r, color c) { detail::check(crabby_ctx_draw_rect(raw_, r, c)); }
    void draw_line(vec2 a, vec2 b, float width, color c) { detail::check(crabby_ctx_draw_line(raw_, a, b, width, c)); }
    void draw_circle(vec2 center, float radius, color c) { detail::check(crabby_ctx_draw_circle(raw_, center, radius, c)); }

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

    // Sound (silent in headless runs).
    sound load_sound(const char* path) { sound s{}; detail::check(crabby_ctx_load_sound(raw_, path, &s)); return s; }
    sound create_sound(std::uint32_t sample_rate, const float* samples, std::size_t len) {
        sound s{}; detail::check(crabby_ctx_create_sound(raw_, sample_rate, samples, len, &s)); return s;
    }
    playing play_sound(sound s) { playing p{}; detail::check(crabby_ctx_play_sound(raw_, s, nullptr, &p)); return p; }
    playing play_sound(sound s, const sound_params& params) { playing p{}; detail::check(crabby_ctx_play_sound(raw_, s, &params, &p)); return p; }
    void stop_sound(playing p) { detail::check(crabby_ctx_stop_sound(raw_, p)); }

    /// Spreads work over `every` ticks (safe from query_par callbacks).
    crabby::turns turns(std::uint32_t every) const { crabby::turns t; detail::check(crabby_ctx_turns(raw_, every, &t.raw)); return t; }
    /// Textures reloaded from changed files at the start of this frame (hot
    /// reload). They keep their handles; redo anything computed from them.
    std::vector<texture> reloaded_textures() const {
        const CrabbyTexture* list = nullptr; std::size_t n = 0;
        detail::check(crabby_ctx_reloaded_textures(raw_, &list, &n));
        return std::vector<texture>(list, list + n);
    }

    // Commands: applied after the system; usable inside world().query (not query_par).
    void despawn_later(entity e) { detail::check(crabby_ctx_despawn_later(raw_, e)); }
    template<class... Ts> void spawn_later(const value<Ts>&... values) {
        std::array<CrabbyValue, sizeof...(Ts)> args{{c_value(values)...}};
        detail::check(crabby_ctx_spawn_later(raw_, args.data(), args.size()));
    }
    template<class T> void set_later(entity e, const component<T>& token, const T& data) {
        CrabbyValue v = c_value(token.with(data));
        detail::check(crabby_ctx_set_later(raw_, e, &v));
    }
    template<class T> void remove_later(entity e, const component<T>& token) {
        detail::check(crabby_ctx_remove_later(raw_, e, world().id(token)));
    }

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
        try { ctx c(raw, h.st->owner); h.fn(c); }
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
    game& spatial_grid(float cell_size) { return check(crabby_game_spatial_grid(raw_, cell_size)); }
    game& control_endpoint(bool on) { return check(crabby_game_control_endpoint(raw_, on)); }
    /// Reload textures when their files change, under the same handles (on by
    /// default in debug builds of the engine; CRABBY_HOT_RELOAD=1/0 overrides).
    game& hot_reload(bool on) { return check(crabby_game_hot_reload(raw_, on)); }
    /// The built-in camera controls: right/middle drag pans, the wheel zooms.
    game& camera_controls() { return check(crabby_game_camera_controls(raw_)); }
    /// A system on the fixed simulation tick; `every` > 1 runs it every n ticks.
    template<class F> game& sim_system(F fn, const char* name = nullptr, std::uint32_t every = 1) {
        return check(crabby_game_sim_system(raw_, name, every, trampoline, keep(std::move(fn))));
    }
    /// A system once per frame (input, camera, UI). Not run headless.
    template<class F> game& system(F fn, const char* name = nullptr) {
        return check(crabby_game_system(raw_, name, trampoline, keep(std::move(fn))));
    }
    /// Removes systems by name, e.g. "crabbyengine::ecs::draw_sprites".
    game& remove_system(const char* name) { return check(crabby_game_remove_system(raw_, name)); }

    /// Opens the window and runs until quit. `setup(ctx&)` runs once first.
    template<class S> void run(S setup) {
        holder* h = keep(std::move(setup));
        auto status = crabby_game_run(take(), trampoline, h);
        state_->error.rethrow();
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
        detail::check(status);
        return owned;
    }
    crabby::world run_headless(std::uint64_t ticks) { return run_headless(ticks, [](ctx&) {}); }
};
} // namespace crabby
#endif
