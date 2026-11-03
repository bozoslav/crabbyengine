// The battle demo in C++: the same game as examples/battle.rs, written
// against crabby.hpp. Two crab armies march along flow fields, keep their
// spacing, fight whatever enemy they meet, and tear down the enemy base.
//
//   ./crabby_cpp_battle                              (window, 20k units)
//   UNITS=200000 ./crabby_cpp_battle
//   HEADLESS=1 UNITS=1000000 TICKS=600 ./crabby_cpp_battle
//
// Mouse: left click sets red's rally point; drag with the right button to
// pan, scroll to zoom. Keys: A attack, D defend, Space pause, 1-3 speed,
// R restart, Escape quit.
//
// Who's winning is a reflected `Score` resource: agents read it through
// crabby-mcp (get_resource with "Score").
#include "crabby.hpp"
#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <thread>
#include <vector>

using crabby::vec2;

constexpr float CELL = 4.0f;
constexpr std::uint32_t MAP_W = 400, MAP_H = 225;
constexpr float UNITS_PER_MAP = 20000.0f;
constexpr std::uint8_t MUD = 5;

constexpr float SPEED = 34.0f;
constexpr float HP = 10.0f;
constexpr float DAMAGE = 2.0f;
constexpr float COOLDOWN = 0.6f;
constexpr float SIGHT = 18.0f;
constexpr float REACH = 5.0f;
constexpr float PERSONAL_SPACE = 3.5f;
constexpr float BASE_RADIUS = 22.0f;

constexpr std::uint32_t RED = 0, BLUE = 1;
constexpr crabby::entity NO_HIT = UINT64_MAX;
constexpr crabby::entity HIT_BASE = UINT64_MAX - 1;

struct Unit {
    std::uint32_t team;
    float hp;
    vec2 vel;
    float cooldown;
    bool engaged;
    vec2 push;
    crabby::entity hit;   // decided in parallel, applied in order after
};

struct Base {
    std::uint32_t team;
    float hp;
    float max_hp;
    float budget;
};

struct Score {
    std::uint32_t red_alive, blue_alive;
    std::uint32_t red_rounds, blue_rounds;
    std::uint32_t winner;   // 0 none, 1 red, 2 blue
};

static double env_number(const char* name, double fallback) {
    const char* value = std::getenv(name);
    return value ? std::atof(value) : fallback;
}

static crabby::color team_color(std::uint32_t team) {
    return team == RED ? crabby::rgb(1.0f, 0.42f, 0.36f) : crabby::rgb(0.36f, 0.72f, 1.0f);
}

/// Everything the systems share: the map, one field per team, the grids.
struct Battle {
    std::unique_ptr<crabby::nav_grid> nav;
    crabby::flow_field fields[2];
    vec2 rally[2], bases[2];
    crabby::entity base_entities[2] = {0, 0};
    std::size_t cap = 0;
    float field_ms = 0;
    std::optional<crabby::texture> look;
    // Per team: a fine grid for friends, a coarse one for enemies.
    crabby::grid near[2] = {crabby::grid(PERSONAL_SPACE * 2), crabby::grid(PERSONAL_SPACE * 2)};
    crabby::grid far[2] = {crabby::grid(SIGHT), crabby::grid(SIGHT)};
    struct Placed { CrabbyEntity entity; vec2 at; std::uint32_t team; };
    std::vector<Placed> all;
    std::vector<crabby::grid_item> positions[2];
    std::vector<std::pair<std::uint32_t, crabby::entity>> hits;

    void set_rally(std::uint32_t team, vec2 at) {
        const auto start = std::chrono::steady_clock::now();
        rally[team] = at;
        fields[team].compute(*nav, at);
        field_ms = float(std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start).count());
    }

    float cost_at(vec2 p) const {
        const auto c = nav->cell_at(p);
        return c ? float(std::max<std::uint8_t>(*nav->cost((*c)[0], (*c)[1]), 1)) : 1.0f;
    }
};

// ---------------------------------------------------------------------- map

static void make_map(Battle& b, std::uint32_t w, std::uint32_t h, std::uint64_t seed) {
    b.nav = std::make_unique<crabby::nav_grid>(w, h, CELL);
    auto& nav = *b.nav;
    crabby::rng rng(seed);
    const float area = (float(w) / MAP_W) * (float(h) / MAP_H);
    auto blob = [&](std::uint8_t cost, float radius) {
        const vec2 c{rng.range(0, float(w)), rng.range(0, float(h))};
        const float r = radius * rng.range(0.6f, 1.4f);
        for (std::uint32_t y = std::uint32_t(std::max(c.y - r, 0.0f)); y < std::min(std::uint32_t(c.y + r), h); ++y)
            for (std::uint32_t x = std::uint32_t(std::max(c.x - r, 0.0f)); x < std::min(std::uint32_t(c.x + r), w); ++x)
                if (crabby::length(vec2{float(x), float(y)} - c) < r) nav.set_cost(x, y, cost);
    };
    for (int i = 0; i < int(16 * area); ++i) blob(MUD, 18);
    for (int i = 0; i < int(70 * area); ++i) blob(crabby::nav_grid::wall, 7);
    for (int i = 0; i < int(6 * area); ++i) blob(crabby::nav_grid::wall, 16);
    const vec2 world{float(w) * CELL, float(h) * CELL};
    b.bases[RED] = {0.08f * world.x, 0.5f * world.y};
    b.bases[BLUE] = {0.92f * world.x, 0.5f * world.y};
    for (vec2 base : b.bases) {
        const vec2 c = base / CELL;
        const float r = BASE_RADIUS * 3 / CELL;
        for (auto y = std::uint32_t(c.y - r); y < std::uint32_t(c.y + r); ++y)
            for (auto x = std::uint32_t(c.x - r); x < std::uint32_t(c.x + r); ++x) nav.set_cost(x, y, 1);
    }
    for (std::uint32_t x = 0; x < w; ++x) { nav.set_cost(x, 0, crabby::nav_grid::wall); nav.set_cost(x, h - 1, crabby::nav_grid::wall); }
    for (std::uint32_t y = 0; y < h; ++y) { nav.set_cost(0, y, crabby::nav_grid::wall); nav.set_cost(w - 1, y, crabby::nav_grid::wall); }
    // If the blobs cut the bases off from each other, dig a road between them.
    crabby::flow_field flow;
    flow.compute(nav, b.bases[BLUE]);
    if (!flow.distance_at(b.bases[RED])) {
        const auto y = std::uint32_t(b.bases[RED].y / CELL);
        for (std::uint32_t x = 1; x + 1 < w; ++x)
            for (std::uint32_t dy = 0; dy < 3; ++dy)
                nav.set_cost(x, y + dy - 1, std::max<std::uint8_t>(*nav.cost(x, y + dy - 1), 1));
    }
}

// ------------------------------------------------------------------ systems

static void spawn_unit(crabby::ctx& ctx, Battle& b, std::uint32_t team, vec2 at) {
    const Unit unit{team, HP, {0, 0}, 0, false, {0, 0}, NO_HIT};
    if (b.look) ctx.spawn_later(crabby::transform_at(at), crabby::sprite_of(*b.look, team_color(team)), unit);
    else ctx.spawn_later(crabby::transform_at(at), unit);
}

/// Every tick, on all cores: separation, enemy search, attack or march.
/// Marching units look for enemies only on their turn (every 4th tick).
static void fight(crabby::ctx& ctx, Battle& b) {
    auto world = ctx.world();
    const float dt = ctx.dt();
    const crabby::random random = ctx.random();
    const crabby::turns look = ctx.turns(4);
    const bool crowding = ctx.sim_tick() % 2 == 0;
    const bool over = world.resource<Score>().winner != 0;
    // Every unit's place; each chunk fills its own rows. On all cores when
    // there are enough units to pay for it.
    b.all.resize(world.count<crabby::transform2d, Unit>());
    auto fill = [&](auto chunk) {
        auto [t, u] = chunk.columns();
        for (std::size_t i = 0; i < chunk.size(); ++i)
            b.all[chunk.first() + i] = {chunk.entities()[i], t[i].position, u[i].team};
    };
    if (b.all.size() > 100000) world.query_par<crabby::transform2d, Unit>(fill);
    else world.query<crabby::transform2d, Unit>(fill);
    {
        // Split by team, branch-free: teams are mixed in memory, so an `if`
        // would mispredict half the time. Big armies split both at once.
        auto split = [&b](std::uint32_t team) {
            auto& out = b.positions[team];
            out.resize(b.all.size());
            std::size_t n = 0;
            for (const auto& p : b.all) {
                out[n] = {p.entity, p.at, 0};
                n += p.team == team;
            }
            out.resize(n);
        };
        if (b.all.size() > 100000) {
            std::thread red(split, 0);
            split(1);
            red.join();
        } else {
            split(0);
            split(1);
        }
        std::vector<std::thread> builders;
        for (int team = 0; team < 2; ++team) {
            if (crowding) builders.emplace_back([&, team] { b.near[team].rebuild(b.positions[team]); });
            builders.emplace_back([&, team] { b.far[team].rebuild(b.positions[team]); });
        }
        for (auto& t : builders) t.join();
    }
    const std::array<crabby::grid_view<crabby::grid_item>, 2> near{b.near[0].view(), b.near[1].view()};
    const std::array<crabby::grid_view<crabby::grid_item>, 2> far{b.far[0].view(), b.far[1].view()};
    const std::array<crabby::flow_view, 2> field{b.fields[0].view(), b.fields[1].view()};
    const Battle& battle = b;
    world.each_par([&](crabby::entity e, crabby::transform2d& t, Unit& unit) {
        crabby::rng rng = random.of(e);
        const vec2 me = t.position;
        unit.hit = NO_HIT;
        unit.cooldown -= dt;
        if (crowding) {
            vec2 push{0, 0};
            near[unit.team].for_each_in_radius(me, PERSONAL_SPACE, [&](const crabby::grid_item& o) {
                if (crabby::bits(o.entity) == e) return;
                const vec2 away = me - o.position;
                push += away / (crabby::length_squared(away) + 0.01f);
            });
            const float len = crabby::length(push);
            unit.push = len > 1 ? push / len : push;
        }
        // Nearest enemy: in reach first, in sight only if none is.
        bool found = false;
        float best = 0;
        const crabby::grid_item* target = nullptr;
        if (unit.engaged || look.includes(e)) {
            const auto& enemies = far[1 - unit.team];
            for (float radius : {REACH, SIGHT}) {
                enemies.for_each_in_radius(me, radius, [&](const crabby::grid_item& o) {
                    const float d2 = crabby::length_squared(o.position - me);
                    if (!found || d2 < best) { found = true; best = d2; target = &o; }
                });
                if (found) break;
            }
            unit.engaged = found;
        }
        vec2 want{0, 0};
        const vec2 enemy_base = battle.bases[1 - unit.team];
        if (found && best < REACH * REACH) {
            if (unit.cooldown <= 0) {
                unit.cooldown = COOLDOWN * rng.range(0.8f, 1.2f);
                unit.hit = crabby::bits(target->entity);
            }
        } else if (found) {
            want = crabby::normalize_or(target->position - me, {0, 0}) * SPEED;
        } else if (crabby::length(me - enemy_base) < BASE_RADIUS + REACH) {
            if (unit.cooldown <= 0 && !over) {
                unit.cooldown = COOLDOWN;
                unit.hit = HIT_BASE;
            }
        } else {
            want = field[unit.team].direction_at(me) * SPEED;
        }
        want = want / battle.cost_at(me) + unit.push * SPEED;
        unit.vel = unit.vel + (want - unit.vel) * 0.2f + rng.unit_vec2() * 2.0f;
        // Slide along walls instead of walking into them.
        const vec2 next = me + unit.vel * dt;
        const auto& nav = *battle.nav;
        if (nav.walkable(next)) t.position = next;
        else if (nav.walkable({next.x, me.y})) t.position.x = next.x;
        else if (nav.walkable({me.x, next.y})) t.position.y = next.y;
    });
}

/// Applies the hits in order, removes the dead, checks for a fallen base.
static void resolve(crabby::ctx& ctx, Battle& b) {
    auto world = ctx.world();
    b.hits.clear();
    world.each([&](const Unit& u) {
        if (u.hit != NO_HIT) b.hits.push_back({u.team, u.hit});
    });
    for (auto [team, hit] : b.hits) {
        if (hit == HIT_BASE) {
            Base base = world.get<Base>(b.base_entities[1 - team]);
            base.hp = std::max(base.hp - DAMAGE, 0.0f);
            world.set(b.base_entities[1 - team], base);
        } else {
            Unit u = world.get<Unit>(hit);
            u.hp -= DAMAGE;
            world.set(hit, u);
        }
    }
    std::uint32_t alive[2] = {0, 0};
    world.each([&](crabby::entity e, const Unit& u) {
        if (u.hp <= 0) ctx.despawn_later(e);
        else alive[u.team] += 1;
    });
    Score s = world.resource<Score>();
    s.red_alive = alive[RED];
    s.blue_alive = alive[BLUE];
    for (std::uint32_t team : {RED, BLUE}) {
        if (s.winner == 0 && world.get<Base>(b.base_entities[team]).hp <= 0) {
            s.winner = 2 - team;   // the other team: red (1) or blue (2)
            (team == RED ? s.blue_rounds : s.red_rounds) += 1;
        }
    }
    world.set_resource(s);
}

/// Bases send new units, up to the cap, while the round lasts.
static void reinforce(crabby::ctx& ctx, Battle& b) {
    auto world = ctx.world();
    const Score s = world.resource<Score>();
    if (s.winner != 0) return;
    crabby::rng rng = ctx.random().stream(1);
    const float per_tick = float(b.cap) / (40.0f * 60.0f);
    for (std::uint32_t team : {RED, BLUE}) {
        Base base = world.get<Base>(b.base_entities[team]);
        const std::size_t alive = team == RED ? s.red_alive : s.blue_alive;
        const std::size_t room = b.cap > alive ? b.cap - alive : 0;
        base.budget = std::min(base.budget + per_tick, float(room));
        const int n = int(base.budget);
        base.budget -= float(n);
        world.set(b.base_entities[team], base);
        for (int i = 0; i < n; ++i)
            spawn_unit(ctx, b, team, b.bases[team] + rng.unit_vec2() * rng.range(0, BASE_RADIUS));
    }
}

/// Blue's general: attack red's base, or defend when outnumbered 2 to 1.
static void blue_ai(crabby::ctx& ctx, Battle& b) {
    const Score s = ctx.world().resource<Score>();
    const vec2 goal = s.blue_alive * 2 < s.red_alive ? b.bases[BLUE] : b.bases[RED];
    if (b.rally[BLUE].x != goal.x || b.rally[BLUE].y != goal.y) b.set_rally(BLUE, goal);
}

/// Both armies at full strength around their bases, bases repaired.
static void start_round(crabby::ctx& ctx, Battle& b) {
    auto world = ctx.world();
    crabby::rng rng = ctx.random().stream(ctx.sim_tick());
    world.each([&](crabby::entity e, const Unit&) { ctx.despawn_later(e); });
    for (std::uint32_t team : {RED, BLUE}) {
        Base base = world.get<Base>(b.base_entities[team]);
        base.hp = base.max_hp;
        world.set(b.base_entities[team], base);
    }
    Score s = world.resource<Score>();
    s.winner = 0;
    world.set_resource(s);
    const float spread = std::sqrt(float(b.cap)) * 2.2f;
    for (std::uint32_t team : {RED, BLUE}) {
        for (std::size_t placed = 0; placed < b.cap;) {
            const vec2 p = b.bases[team] + rng.unit_vec2() * std::sqrt(rng.range(0, 1)) * spread;
            if (b.nav->walkable(p)) { spawn_unit(ctx, b, team, p); ++placed; }
        }
    }
}

static void setup(crabby::ctx& ctx, Battle& b, std::size_t units, std::uint64_t seed) {
    const float scale = std::sqrt(std::max(float(units) / UNITS_PER_MAP, 1.0f));
    make_map(b, std::uint32_t(MAP_W * scale), std::uint32_t(MAP_H * scale), seed);
    auto world = ctx.world();
    // Reflected, so agents can read and change them through crabby-mcp.
    world.reflect<Unit>({
        crabby::field("team", &Unit::team), crabby::field("hp", &Unit::hp),
        crabby::field("vel", &Unit::vel), crabby::field("cooldown", &Unit::cooldown),
        crabby::field("engaged", &Unit::engaged),
    });
    world.reflect<Base>({
        crabby::field("team", &Base::team), crabby::field("hp", &Base::hp),
        crabby::field("max_hp", &Base::max_hp), crabby::field("budget", &Base::budget),
    });
    world.reflect_resource<Score>({
        crabby::field("red_alive", &Score::red_alive), crabby::field("blue_alive", &Score::blue_alive),
        crabby::field("red_rounds", &Score::red_rounds), crabby::field("blue_rounds", &Score::blue_rounds),
        crabby::field("winner", &Score::winner),
    });
    const float max_hp = 400.0f * std::sqrt(float(units) / 1000.0f);
    for (std::uint32_t team : {RED, BLUE})
        b.base_entities[team] = world.spawn(crabby::transform_at(b.bases[team]), Base{team, max_hp, max_hp, 0});
    world.set_resource(Score{0, 0, 0, 0, 0});
    b.cap = units / 2;
    b.set_rally(RED, b.bases[BLUE]);
    b.set_rally(BLUE, b.bases[RED]);
    start_round(ctx, b);
}

static crabby::game& battle_systems(crabby::game& game, Battle& b) {
    return game.sim_system([&](crabby::ctx& ctx) { fight(ctx, b); }, "fight")
        .sim_system([&](crabby::ctx& ctx) { resolve(ctx, b); }, "resolve")
        .sim_system([&](crabby::ctx& ctx) { reinforce(ctx, b); }, "reinforce")
        .sim_system([&](crabby::ctx& ctx) { blue_ai(ctx, b); }, "blue_ai", 30)
        .sim_system([&](crabby::ctx& ctx) { ctx.world().sort_spatial(64); }, "keep_neighbors_close", 60);
}

// ------------------------------------------------------------------ playing

static void controls(crabby::ctx& ctx, Battle& b) {
    std::optional<vec2> rally;
    if (ctx.mouse_pressed(CRABBY_MOUSE_LEFT) && !ctx.mouse_over_ui()) rally = ctx.mouse_world();
    else if (ctx.key_pressed(CRABBY_KEY_A)) rally = b.bases[BLUE];
    else if (ctx.key_pressed(CRABBY_KEY_D)) rally = b.bases[RED];
    if (rally && b.nav->walkable(*rally)) b.set_rally(RED, *rally);
    if (ctx.key_pressed(CRABBY_KEY_R)) start_round(ctx, b);
    if (ctx.key_pressed(CRABBY_KEY_SPACE)) { if (ctx.is_paused()) ctx.resume(); else ctx.pause(); }
    const std::pair<crabby::key, float> speeds[] = {{CRABBY_KEY_NUM_1, 1}, {CRABBY_KEY_NUM_2, 2}, {CRABBY_KEY_NUM_3, 4}};
    for (auto [k, speed] : speeds) if (ctx.key_pressed(k)) ctx.set_sim_speed(speed);
    if (ctx.key_pressed(CRABBY_KEY_ESCAPE)) ctx.quit();
}

/// The map (a pixel per cell), the bases with their health, the rally point, the HUD.
struct Draw {
    Battle* b;
    std::optional<crabby::texture> map;
    void operator()(crabby::ctx& ctx) {
        const auto& nav = *b->nav;
        if (!map) {
            std::vector<std::uint8_t> px(std::size_t(nav.width()) * nav.height() * 4);
            for (std::uint32_t y = 0; y < nav.height(); ++y)
                for (std::uint32_t x = 0; x < nav.width(); ++x) {
                    const auto n = std::uint8_t((x * 73856093u ^ y * 19349663u) % 9);
                    const std::uint8_t cost = *nav.cost(x, y);
                    const std::array<std::uint8_t, 4> c =
                        cost == crabby::nav_grid::wall ? std::array<std::uint8_t, 4>{std::uint8_t(52 + n), std::uint8_t(56 + n), std::uint8_t(66 + n), 255}
                        : cost == MUD ? std::array<std::uint8_t, 4>{std::uint8_t(74 + n), std::uint8_t(56 + n), 34, 255}
                                      : std::array<std::uint8_t, 4>{std::uint8_t(30 + n), std::uint8_t(46 + n), 32, 255};
                    std::copy(c.begin(), c.end(), px.begin() + (std::size_t(y) * nav.width() + x) * 4);
                }
            map = ctx.create_texture(nav.width(), nav.height(), px.data(), px.size());
            ctx.set_texture_filter(*map, crabby::texture_filter::nearest);
        }
        ctx.clear(crabby::rgb(0, 0, 0));
        const vec2 world{float(nav.width()) * CELL, float(nav.height()) * CELL};
        auto params = crabby::draw_defaults();
        params.scale = {CELL, CELL};
        params.z = -10;
        ctx.draw(*map, world / 2.0f, params);
        auto w = ctx.world();
        for (std::uint32_t team : {RED, BLUE}) {
            const Base base = w.get<Base>(b->base_entities[team]);
            const vec2 at = b->bases[team];
            crabby::color c = team_color(team);
            ctx.draw_circle(at, BASE_RADIUS, crabby::rgba(c.r, c.g, c.b, 0.35f));
            ctx.draw_circle(at, BASE_RADIUS * 0.4f, c);
            ctx.draw_rect(crabby::rect_at({at.x - BASE_RADIUS, at.y - BASE_RADIUS - 8}, {BASE_RADIUS * 2, 3}), crabby::rgba(0, 0, 0, 0.6f));
            ctx.draw_rect(crabby::rect_at({at.x - BASE_RADIUS, at.y - BASE_RADIUS - 8}, {BASE_RADIUS * 2 * base.hp / base.max_hp, 3}), c);
        }
        ctx.draw_circle(b->rally[RED], 5, crabby::rgba(1, 0.9f, 0.3f, 0.9f));

        const Score s = w.resource<Score>();
        const crabby::rect panel = crabby::rect_at({12, 12}, {300, 130});
        ctx.ui_panel(panel);
        char text[256];
        std::snprintf(text, sizeof text,
                      "red %u vs blue %u   (rounds %u-%u)\n%.0f FPS, %s\nfield %.2f ms\nclick: rally  A: attack  D: defend\nR: restart  1-3: speed",
                      s.red_alive, s.blue_alive, s.red_rounds, s.blue_rounds, double(ctx.fps()),
                      ctx.is_paused() ? "paused" : "running", double(b->field_ms));
        ctx.ui_label({panel.min.x + 10, panel.min.y + 10}, text);
        if (s.winner != 0) {
            const char* msg = s.winner == 1 ? "Red wins! R for another round" : "Blue wins. R to try again";
            const vec2 size = ctx.measure_text(msg, 32);
            ctx.draw_text(msg, ctx.window_size() / 2.0f - size / 2.0f, 32, team_color(s.winner - 1));
        }
    }
};

int main() {
    const auto units = std::size_t(env_number("UNITS", 20000));
    const auto seed = std::uint64_t(env_number("SEED", 1));
    Battle battle;
    if (env_number("HEADLESS", 0) != 0) {
        const auto ticks = std::uint64_t(env_number("TICKS", 600));
        auto start = std::chrono::steady_clock::now();
        crabby::game game("battle", 1, 1);
        game.seed(seed);
        battle_systems(game, battle).sim_system([&](crabby::ctx& ctx) {
            const Score s = ctx.world().resource<Score>();
            if (s.winner != 0) start_round(ctx, battle);   // headless rounds restart on their own
            if (ctx.sim_tick() % 120 == 0)
                std::printf("tick %llu: red %u blue %u, rounds %u-%u\n", (unsigned long long)ctx.sim_tick(),
                            s.red_alive, s.blue_alive, s.red_rounds, s.blue_rounds);
        }, "report");
        auto world = game.run_headless(ticks, [&](crabby::ctx& ctx) {
            setup(ctx, battle, units, seed);
            start = std::chrono::steady_clock::now();
        });
        const double secs = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
        std::printf("%zu units, %ux%u cells, %llu ticks: %.2f ms/tick (%.0f ticks/s)\n", units, battle.nav->width(),
                    battle.nav->height(), (unsigned long long)ticks, secs * 1e3 / double(ticks), double(ticks) / secs);
        return 0;
    }
    crabby::game game("Crabby Engine: C++ battle", 1280, 720);
    game.seed(seed);
    battle_systems(game, battle)
        .system([&](crabby::ctx& ctx) { controls(ctx, battle); }, "controls")
        .camera_controls()
        .system(Draw{&battle, {}}, "draw")
        .run([&](crabby::ctx& ctx) {
            const std::uint8_t white[16] = {255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255};
            battle.look = ctx.create_texture(2, 2, white, sizeof white);
            setup(ctx, battle, units, seed);
            const vec2 world{float(battle.nav->width()) * CELL, float(battle.nav->height()) * CELL};
            crabby::camera cam = ctx.camera();
            cam.zoom = std::min(ctx.window_size().x / world.x, ctx.window_size().y / world.y);
            cam.position = world / 2.0f - ctx.window_size() / (2.0f * cam.zoom);
            ctx.set_camera(cam);
        });
}
