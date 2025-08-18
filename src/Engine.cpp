#include "Engine.hpp"

#include "Output.hpp"
#include "wlr-layer-shell-unstable-v1-client-protocol.h"

#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <string>
#include <vector>
#include <wayland-client-core.h>
#include <wayland-client-protocol.h>

namespace WaypaperEngine {

static void registry_global_handler(void* data, struct wl_registry* registry, uint32_t name,
                                    const char* interface, uint32_t version) {
    Engine* engine = static_cast<Engine*>(data);

    if (strcmp(interface, wl_compositor_interface.name) == 0) {
        engine->compositor = static_cast<wl_compositor*>(
            wl_registry_bind(registry, name, &wl_compositor_interface, version));
    } else if (strcmp(interface, wl_shm_interface.name) == 0) {
        engine->shm =
            static_cast<wl_shm*>(wl_registry_bind(registry, name, &wl_shm_interface, version));
    } else if (strcmp(interface, zwlr_layer_shell_v1_interface.name) == 0) {
        engine->layer_shell = static_cast<zwlr_layer_shell_v1*>(
            wl_registry_bind(registry, name, &zwlr_layer_shell_v1_interface, version));
    } else if (strcmp(interface, wl_seat_interface.name) == 0) {
        engine->seat =
            static_cast<wl_seat*>(wl_registry_bind(registry, name, &wl_seat_interface, version));
    } else if (strcmp(interface, wl_output_interface.name) == 0) {
        const auto output = engine->outputs.emplace_back(std::make_unique<Output>()).get();
        output->wayland_name = name;
        output->output = static_cast<wl_output*>(wl_registry_bind(registry, name, &wl_output_interface, version));
        output->initListeners();
    }
}

static void registry_global_remove_handler(void* data, struct wl_registry* registry,
                                           uint32_t name) {
    Engine* engine = static_cast<Engine*>(data);
    (void)engine;
    (void)name;
    (void)registry;
}

struct wl_registry_listener registry_listener = {
    .global = WaypaperEngine::registry_global_handler,
    .global_remove = WaypaperEngine::registry_global_remove_handler,
};

Engine::Engine() {}

Engine::~Engine() {
    wl_display_disconnect(this->display);
}

int Engine::init() {
    this->display = wl_display_connect(NULL);

    if (!this->display) {
        std::cerr << "Failed to Connect to a Wayland Display." << std::endl;
        return 1;
    }

    struct wl_registry* registry = wl_display_get_registry(this->display);
    wl_registry_add_listener(registry, &registry_listener, this);

    wl_display_roundtrip(this->display);

    if (!this->compositor || !this->shm || !this->layer_shell) {
        std::cerr << "Missing some Required Globals." << std::endl;
        return 1;
    }

    printf("Recived all Required Globals.\n");

    while (1) { wl_display_dispatch(this->display); }
}

bool lockSingleInstance() {
    const std::string XDG_RUNTIME_DIR = getenv("XDG_RUNTIME_DIR");
    const std::string LOCKFILE = XDG_RUNTIME_DIR + "/waypaper-engine.lock";

    if (std::filesystem::exists(LOCKFILE)) { return false; }

    std::ofstream lockfile(LOCKFILE, std::ios::trunc);
    lockfile.close();
    return true;
}

void Engine::parse_cli_args(std::vector<std::string> args) {}

} // namespace WaypaperEngine
