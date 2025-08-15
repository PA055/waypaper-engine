#include "Engine.hpp"
#include <cstddef>
#include <cstdio>
#include <iostream>
#include <wayland-client-core.h>

namespace WaypaperEngine {

Engine::Engine() {
    this->display = wl_display_connect(NULL);

    if (!this->display) {
        std::cerr << "Failed to Connect to a Wayland Display" << std::endl;
        exit(1);
    }

    printf("idk it connected ig idrk what to do with this tho");
}

Engine::~Engine() {
    wl_display_disconnect(this->display);
}
}
