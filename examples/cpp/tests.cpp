#include "crabby.hpp"
#include <cstdlib>
#include <iostream>
#include <string>
#include <type_traits>
#define CHECK(x) do { if (!(x)) { std::cerr << "failed: " #x << '\n'; std::abort(); } } while (false)
struct Pos { float x, y; };
struct Vel { float x, y; };
template<class F> void throws(F f) { bool caught = false; try { f(); } catch (const std::exception&) { caught = true; } CHECK(caught); }
int main() {
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
    CHECK(a.size() == 1);
    throws([&] { a.query([&](auto) { (void)a.size(); }, p); });
    throws([&] { a.query([](auto) {}, p, p); });
    crabby::world b;
    throws([&] { b.spawn(p.with(Pos{})); });
    throws([&] { b.query([](auto) {}, p); });
    throws([&] { b.get(e, p); });
    throws([&] { b.set(e, p, Pos{}); });
    throws([&] { b.remove(e, p); });
    crabby::world c(std::move(a)); CHECK(c.get(e, p).x == 9);
    throws([&] { (void)a.size(); });
    b = std::move(c); CHECK(b.get(e, p).x == 9);
    b.remove(e, v); throws([&] { (void)b.get(e, v); });
    b.despawn(e); CHECK(!b.alive(e)); throws([&] { b.despawn(e); });
    std::size_t seen = 0;
    b.query([&](auto chunk) { seen += chunk.size(); }); CHECK(seen == 0);
    std::uint64_t ticks = 0;
    auto sim = crabby::world::run_headless(3, 50,
        [](crabby::world_view& w) { w.spawn(); return true; },
        [&](crabby::world_view& w, std::uint64_t, float dt) { CHECK(w.size() == 1); CHECK(dt > .019f && dt < .021f); ++ticks; return true; });
    CHECK(ticks == 3 && sim.size() == 1);
    ticks = 0;
    auto stopped = crabby::world::run_headless(10, 60, [](crabby::world_view&) { return true; }, [&](crabby::world_view&, std::uint64_t, float) { ++ticks; return false; });
    CHECK(ticks == 1 && stopped.size() == 0);
    throws([] { crabby::world::run_headless(1, 60, [](crabby::world_view&) -> bool { throw std::runtime_error("setup"); }, [](crabby::world_view&, std::uint64_t, float) { return true; }); });
    throws([] { crabby::world::run_headless(1, 60, [](crabby::world_view&) { return true; }, [](crabby::world_view&, std::uint64_t, float) -> bool { throw std::runtime_error("step"); }); });
}
