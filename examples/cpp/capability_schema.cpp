#include "ponko.hpp"
#include <cstdlib>
struct Health { float hp; };
int main() {
    ponko::world w;
    w.reflect<Health>("Health", {ponko::field("hp", &Health::hp)});
    w.reflect_resource<Health>({ponko::field("hp", &Health::hp)});
    w.spawn(Health{5.0f});
    w.set_resource(Health{8.0f});
    const auto before = w.save();
    w.describe_component("Health", R"({"Object":[["hp","Number"]]})");
    // Resource names follow the wrapper's existing type-name convention.
    w.describe_resource(ponko::detail::type_name<Health>().c_str(), R"({"Object":[["hp","Number"]]})");
    if (w.save() != before) std::abort();
    bool failed = false;
    try { w.describe_component("missing", "\"Number\""); } catch (const std::exception&) { failed = true; }
    if (!failed) std::abort();
    failed = false;
    try { w.describe_resource("missing", "\"Number\""); } catch (const std::exception&) { failed = true; }
    if (!failed) std::abort();
}
