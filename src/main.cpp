#include "Engine.hpp"

#include <memory>
#include <string>
#include <vector>

int main(int argc, char* argv[]) {
    using namespace WaypaperEngine;
    auto engine = std::make_unique<Engine>();
    engine->parse_cli_args(std::vector<std::string>(argv + 1, argv + argc));
    return engine->init();
}
