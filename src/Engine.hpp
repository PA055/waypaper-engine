#pragma once

#include "Output.hpp"
#include "wlr-layer-shell-unstable-v1-client-protocol.h"

#include <memory>
#include <string>
#include <vector>
#include <wayland-client-protocol.h>
#include <wayland-client.h>

namespace WaypaperEngine {

class Engine {
    public:
    struct wl_display* display;
    struct wl_compositor* compositor;
    struct wl_shm* shm;
    struct wl_seat* seat;
    struct zwlr_layer_shell_v1* layer_shell;
    std::vector<std::unique_ptr<Output>> outputs;

    Engine();
    ~Engine();

    int init();
    void parse_cli_args(std::vector<std::string> args);
};
} // namespace WaypaperEngine
