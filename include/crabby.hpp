#ifndef CRABBY_HPP
#define CRABBY_HPP
#include "crabby.h"
#include <array>
#include <cstdint>
#include <exception>
#include <memory>
#include <stdexcept>
#include <tuple>
#include <type_traits>
#include <typeindex>
#include <unordered_map>
#include <utility>

namespace crabby {
using entity = std::uint64_t;
namespace detail {
struct identity { std::unordered_map<std::uint32_t, std::type_index> types; };
inline void check(CrabbyStatus status) {
    if (status != CRABBY_OK) throw std::runtime_error(crabby_last_error());
}
template<class T> constexpr bool pod = std::is_trivially_copyable<T>::value &&
    std::is_trivially_destructible<T>::value && std::is_standard_layout<T>::value;
}
class world_view;
template<class T> class component;
// Owns the copied value until spawn returns, including values made from temporaries.
template<class T> struct value {
    component<T> token;
    T data;
};
// A token belongs to one world; it remains valid when that world is moved.
template<class T> class component {
    static_assert(detail::pod<T>, "components must be trivial, standard-layout values");
    std::uint32_t id_;
    std::shared_ptr<detail::identity> owner_;
    component(std::uint32_t id, std::shared_ptr<detail::identity> owner) : id_(id), owner_(std::move(owner)) {}
    friend class world_view;
public:
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
        crabby::entity result = 0;
        detail::check(crabby_chunk_entity(raw_, index, &result));
        return result;
    }
};
// A non-owning view. Headless callback views must not escape their callback.
class world_view {
    friend class world;
protected:
    CrabbyWorld* raw_;
    std::shared_ptr<detail::identity> owner_;
    world_view(CrabbyWorld* raw, std::shared_ptr<detail::identity> owner) : raw_(raw), owner_(std::move(owner)) {}
    void valid() const { if (!raw_) throw std::logic_error("world has been moved from"); }
    template<class T> std::uint32_t id(const crabby::component<T>& token) const {
        valid();
        if (token.owner_ != owner_) throw std::invalid_argument("component belongs to another world");
        return token.id_;
    }
    template<class T> CrabbyValue c_value(const value<T>& v) const { return {id(v.token), &v.data, sizeof(T)}; }
public:
    template<class T> crabby::component<T> component(const char* name) {
        static_assert(detail::pod<T>, "components must be trivial, standard-layout values");
        valid(); std::uint32_t result = 0;
        detail::check(crabby_component_register(raw_, name, sizeof(T), alignof(T), &result));
        const std::type_index type(typeid(T));
        const auto registered = owner_->types.emplace(result, type);
        if (!registered.second && registered.first->second != type)
            throw std::invalid_argument("component name is registered for a different C++ type");
        return crabby::component<T>(result, owner_);
    }
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
    template<class F, class... Ts> void query(F&& fn, const crabby::component<Ts>&... tokens) {
        valid(); std::array<std::uint32_t, sizeof...(Ts)> ids{{id(tokens)...}};
        using function = typename std::remove_reference<F>::type;
        struct context { function* fn; std::exception_ptr error; } ctx{&fn, {}};
        auto trampoline = [](const CrabbyChunk* raw, void* user) noexcept {
            auto& c = *static_cast<context*>(user);
            if (c.error) return;
            try { (*c.fn)(chunk<Ts...>(raw)); } catch (...) { c.error = std::current_exception(); }
        };
        auto status = crabby_query(raw_, ids.data(), ids.size(), trampoline, &ctx);
        if (ctx.error) std::rethrow_exception(ctx.error);
        detail::check(status);
    }
};
class world final : public world_view {
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
    template<class Setup, class Step> static world run_headless(std::uint64_t ticks, std::uint32_t hz, Setup&& setup, Step&& step) {
        auto identity = std::make_shared<detail::identity>();
        using setup_type = typename std::remove_reference<Setup>::type;
        using step_type = typename std::remove_reference<Step>::type;
        struct context { setup_type* setup; step_type* step; std::shared_ptr<detail::identity> owner; std::exception_ptr error; } ctx{&setup, &step, identity, {}};
        auto setup_fn = [](CrabbyWorld* raw, void* user) noexcept -> bool {
            auto& c = *static_cast<context*>(user);
            try { world_view view(raw, c.owner); return (*c.setup)(view); }
            catch (...) { c.error = std::current_exception(); return false; }
        };
        auto step_fn = [](CrabbyWorld* raw, std::uint64_t tick, float dt, void* user) noexcept -> bool {
            auto& c = *static_cast<context*>(user);
            if (c.error) return false;
            try { world_view view(raw, c.owner); return (*c.step)(view, tick, dt); }
            catch (...) { c.error = std::current_exception(); return false; }
        };
        CrabbyWorld* result = nullptr;
        auto status = crabby_run_headless(ticks, hz, setup_fn, step_fn, &ctx, &result);
        world owned(result, std::move(identity));
        if (ctx.error) std::rethrow_exception(ctx.error);
        detail::check(status);
        return owned;
    }
};
} // namespace crabby
#endif
