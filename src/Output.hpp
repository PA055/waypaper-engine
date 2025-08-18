#pragma once

#include <cstdint>
#include <string>
#include <wayland-client-protocol.h>

namespace WaypaperEngine {

class Output {
public:
    Output();
    ~Output();

    uint32_t wayland_name = 0;
    std::string name = "";
    std::string description = "";
    wl_output* output;
    int scale;

    void initListeners();
};

}
