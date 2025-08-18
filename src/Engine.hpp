#pragma once

#include "wlr-layer-shell-unstable-v1-client-protocol.h"
#include <wayland-client-protocol.h>
#include <wayland-client.h>

namespace WaypaperEngine {

class Engine {
public:
  struct wl_display *display;
  struct wl_compositor *compositor;
  struct wl_shm *shm;
  struct zwlr_layer_shell_v1 *layer_shell;

  Engine();
  ~Engine();
};
} // namespace WaypaperEngine
