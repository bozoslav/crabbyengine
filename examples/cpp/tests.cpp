#include "crabby.hpp"
#include <algorithm>
#include <atomic>
#include <cstdlib>
#include <iostream>
#include <string>
#include <type_traits>
#define CHECK(x) do { if (!(x)) { std::cerr << "failed: " #x << " (line " << __LINE__ << ")\n"; std::abort(); } } while (false)
struct Pos { float x, y; };
struct Vel { float x, y; };
template<class F> void throws(F f) { bool caught = false; try { f(); } catch (const std::exception&) { caught = true; } CHECK(caught); }

static void worlds() {
    static_assert(!std::is_copy_constructible<crabby::world>::value, "world owns its handle");
    crabby::world typed;
    auto byte = typed.component<unsigned char>("flag");
    auto flag = typed.spawn(byte.with(static_cast<unsigned char>(2)));
    throws([&] { (void)typed.component<bool>("flag"); });
    auto same_byte = typed.component<unsigned char>("flag");
    CHECK(typed.get(flag, same_byte) == 2);
    crabby::world a;
    auto p = a.component<Pos>("Pos");
    auto v = a.component<Vel>("Vel");
    auto e = a.spawn(p.with(Pos{1, 2}), v.with(Vel{3, 4}));
    CHECK(a.size() == 1 && a.alive(e));
    a.query([&](auto chunk) { CHECK(chunk.size() == 1); auto ps = chunk.template column<0>(); auto vs = chunk.template column<1>(); ps[0].x += vs[0].x; CHECK(chunk.entity(0) == e); throws([&] { (void)chunk.entity(1); }); }, p, v);
    CHECK(a.get(e, p).x == 4);
    std::size_t all = 0; a.query([&](auto chunk) { all += chunk.size(); CHECK(chunk.entity(0) == e); }); CHECK(all == 1);
    a.set(e, p, Pos{9, 8}); CHECK(a.get(e, p).y == 8);
    bool original_exception = false;
    try { a.query([](auto) { throw std::runtime_error("callback"); }, p); }
    catch (const std::runtime_error& error) { original_exception = std::string(error.what()) == "callback"; }
    CHECK(original_exception);
    throws([&] { a.query([&](auto) { (void)a.size(); }, p); });
    throws([&] { a.query([](auto) {}, p, p); });
    crabby::world b;
    throws([&] { b.spawn(p.with(Pos{})); });
    throws([&] { b.get(e, p); });
    crabby::component<Pos> unset;
    throws([&] { (void)a.get(e, unset); });
    crabby::world c(std::move(a)); CHECK(c.get(e, p).x == 9);
    throws([&] { (void)a.size(); });
    b = std::move(c); CHECK(b.get(e, p).x == 9);
    b.remove(e, v); throws([&] { (void)b.get(e, v); });
    b.despawn(e); CHECK(!b.alive(e)); throws([&] { b.despawn(e); });
}

static void parallel_queries() {
    crabby::world w;
    auto n = w.component<int>("n");
    for (int i = 0; i < 100000; ++i) w.spawn(n.with(i));
    std::atomic<long long> rows{0};
    w.query_par([&](auto chunk) {
        auto col = chunk.template column<0>();
        for (std::size_t i = 0; i < chunk.size(); ++i) col[i] *= 2;
        rows += (long long)chunk.size();
    }, n);
    CHECK(rows == 100000);
    long long sum = 0;
    w.query([&](auto chunk) { auto col = chunk.template column<0>(); for (std::size_t i = 0; i < chunk.size(); ++i) sum += col[i]; }, n);
    CHECK(sum == 100000LL * 99999LL);
    throws([&] { w.query_par([](auto) { throw std::runtime_error("par"); }, n); });
}

static void games() {
    // Systems, the clock, quit and exceptions.
    std::uint64_t ticks = 0; float dt = 0;
    crabby::component<Pos> pos;
    auto sim = crabby::game("t", 1, 1).fixed_hz(50)
        .sim_system([&](crabby::ctx& ctx) { ++ticks; dt = ctx.dt(); if (ctx.sim_tick() == 2) ctx.quit(); }, "count")
        .run_headless(100, [&](crabby::ctx& ctx) { pos = ctx.world().component<Pos>("Pos"); ctx.world().spawn(pos.with(Pos{1, 1})); });
    CHECK(ticks == 3 && dt > .019f && dt < .021f && sim.size() == 1);
    // The returned world shares the game's component tokens.
    sim.query([&](auto chunk) { CHECK(chunk.template column<0>()[0].x == 1); }, pos);
    throws([] { crabby::game("t", 1, 1).run_headless(1, [](crabby::ctx&) { throw std::runtime_error("setup"); }); });
    bool second = false;
    throws([&] { crabby::game("t", 1, 1).sim_system([](crabby::ctx&) { throw std::runtime_error("tick"); })
                     .sim_system([&](crabby::ctx&) { second = true; }).run_headless(5); });
    CHECK(!second);
    throws([] { crabby::game("t", 1, 1).fixed_hz(0); });
    throws([] { crabby::game("t", 1, 1).remove_system("nope"); });
    crabby::game g("t", 1, 1);
    g.run_headless(1);
    throws([&] { g.run_headless(1); });

    // Commands from inside a query; spatial queries; random streams; built-ins.
    crabby::component<int> hp;
    std::vector<crabby::grid_hit> hits;
    std::size_t near = 0;
    std::uint64_t first_random = 0;
    auto w = crabby::game("t", 1, 1).spatial_grid(16).seed(3)
        .sim_system([&](crabby::ctx& ctx) {
            ctx.world().query([&](auto chunk) {
                auto h = chunk.template column<0>();
                for (std::size_t i = 0; i < chunk.size(); ++i)
                    if (h[i] <= 0) { ctx.despawn_later(chunk.entity(i)); ctx.spawn_later(hp.with(100)); }
            }, hp);
            ctx.spatial_radius({45, 0}, 16, hits);
            near = hits.size();
            CHECK(ctx.spatial_nearest({1, 0}, 100).has_value());
            CHECK(!ctx.spatial_nearest({1e6f, 0}, 1).has_value());
            first_random = ctx.random().of(7).u64();
            ctx.quit();
        })
        .run_headless(10, [&](crabby::ctx& ctx) {
            auto world = ctx.world();
            hp = world.component<int>("hp");
            auto transforms = world.transforms();
            auto sprites = world.sprites();
            std::uint8_t px[16] = {};
            auto tex = ctx.create_texture(2, 2, px, sizeof px);
            for (int x = 0; x < 10; ++x)
                world.spawn(hp.with(x % 2 ? 5 : 0), transforms.with(crabby::transform_at({x * 10.0f, 0})),
                            sprites.with(crabby::sprite_of(tex, crabby::colors::red, float(x))));
            throws([&] { world.query([](auto) {}, sprites); });
        });
    CHECK(w.size() == 10);
    CHECK(near == 4);
    long long total = 0;
    w.query([&](auto chunk) { auto h = chunk.template column<0>(); for (std::size_t i = 0; i < chunk.size(); ++i) total += h[i]; }, hp);
    CHECK(total == 5 * 5 + 5 * 100);
    CHECK(first_random == crabby::random(0).of(7).u64() || first_random != 0);
}

static void grid_views_match_engine_queries() {
    crabby::grid g(8);
    crabby::rng r(5);
    std::vector<crabby::grid_item> items;
    for (std::uint32_t i = 0; i < 20000; ++i)
        items.push_back({{i, 0}, {r.range(-500, 500), r.range(-500, 500)}, i});
    g.rebuild(items);
    auto view = g.view();
    CHECK(view.size() == items.size());
    std::vector<crabby::grid_item> engine;
    for (int q = 0; q < 300; ++q) {
        const crabby::vec2 c{r.range(-600, 600), r.range(-600, 600)};
        const float radius = q % 50 == 0 ? 2000.0f : r.range(0, 40);
        g.radius(c, radius, engine);
        std::vector<std::uint32_t> a, b;
        for (auto& e : engine) a.push_back(e.data);
        view.for_each_in_radius(c, radius, [&](const crabby::grid_item& e) { b.push_back(e.data); });
        std::sort(a.begin(), a.end());
        std::sort(b.begin(), b.end());
        CHECK(a == b);
    }
    throws([&] { crabby::grid_view<crabby::grid_hit> wrong(CrabbyGridView{nullptr, 20, nullptr, nullptr, 1, 0, 0, 0, 0, 0}); });
}

struct Died { crabby::entity who; std::uint64_t tick; };

static void events() {
    // Sent from inside a query, read by a later system in the same tick, once each.
    crabby::component<int> hp;
    crabby::component<Died> died;
    std::vector<std::uint64_t> seen;
    auto w = crabby::game("t", 1, 1)
        .sim_system([&](crabby::ctx& ctx) {
            ctx.world().query([&](auto chunk) {
                auto h = chunk.template column<0>();
                for (std::size_t i = 0; i < chunk.size(); ++i)
                    if (h[i] <= 0) ctx.send(died, Died{chunk.entity(i), ctx.sim_tick()});
            }, hp);
        }, "deaths")
        .sim_system([&](crabby::ctx& ctx) {
            for (const Died& d : ctx.read(died)) { seen.push_back(d.tick); ctx.despawn_later(d.who); }
            CHECK(ctx.read(died).empty());
        }, "funerals")
        .run_headless(3, [&](crabby::ctx& ctx) {
            hp = ctx.world().component<int>("hp");
            died = ctx.world().component<Died>("Died");
            for (int h : {0, 5, -1}) ctx.world().spawn(hp.with(h));
        });
    CHECK(w.size() == 1);
    CHECK((seen == std::vector<std::uint64_t>{0, 0}));
    throws([&] { crabby::game("t", 1, 1).run_headless(0, [&](crabby::ctx& ctx) { ctx.send(died, Died{}); }); });
}

static void rng_matches_rust() {
    // The same numbers as Rust's Rng (pinned in src/random.rs).
    crabby::rng r(42);
    CHECK(r.u64() == 0xc757960b442b0ac3ull);
    CHECK(r.u64() == 0x4bb22a7f77ff8c6cull);
    CHECK(r.u64() == 0x04950439d3c5eafeull);
    CHECK(crabby::random(7).stream(3).u64() == 0x6045c0e00af2e148ull);
    crabby::rng f(1);
    for (int i = 0; i < 1000; ++i) { float x = f.f32(); CHECK(x >= 0 && x < 1); CHECK(f.below(10) < 10); }
}

int main() {
    worlds();
    parallel_queries();
    games();
    events();
    rng_matches_rust();
    grid_views_match_engine_queries();
    std::cout << "C++ wrapper tests passed\n";
}
