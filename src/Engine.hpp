#pragma once

#include <wayland-client-core.h>

namespace WaypaperEngine {

class Engine {
public:
    struct wl_display *display;

    Engine();
    ~Engine();
};
}
