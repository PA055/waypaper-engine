#include "Engine.hpp"

#include "wlr-layer-shell-unstable-v1-client-protocol.h"

#include <cstddef>
#include <cstdio>
#include <cstring>
#include <iostream>
#include <wayland-client-core.h>
#include <wayland-client-protocol.h>

namespace WaypaperEngine {

static void registry_global_handler(void *data, struct wl_registry *registry,
                                    uint32_t name, const char *interface,
                                    uint32_t version) {
  Engine *engine = static_cast<Engine *>(data);

  if (strcmp(interface, wl_compositor_interface.name) == 0) {
    engine->compositor = static_cast<wl_compositor *>(
        wl_registry_bind(registry, name, &wl_compositor_interface, version));
  } else if (strcmp(interface, wl_shm_interface.name) == 0) {
    engine->shm = static_cast<wl_shm *>(
        wl_registry_bind(registry, name, &wl_shm_interface, version));
  } else if (strcmp(interface, zwlr_layer_shell_v1_interface.name) == 0) {
    engine->layer_shell = static_cast<zwlr_layer_shell_v1 *>(wl_registry_bind(
        registry, name, &zwlr_layer_shell_v1_interface, version));
  }
}

static void registry_global_remove_handler(void *data,
                                           struct wl_registry *registry,
                                           uint32_t name) {
  Engine *engine = static_cast<Engine *>(data);
  (void)engine;
  (void)name;
  (void)registry;
}

struct wl_registry_listener registry_listener = {
    .global = WaypaperEngine::registry_global_handler,
    .global_remove = WaypaperEngine::registry_global_remove_handler,
};

Engine::Engine() {
  this->display = wl_display_connect(NULL);

  if (!this->display) {
    std::cerr << "Failed to Connect to a Wayland Display." << std::endl;
    return;
  }

  struct wl_registry *registry = wl_display_get_registry(this->display);
  wl_registry_add_listener(registry, &registry_listener, this);

  wl_display_roundtrip(this->display);

  if (!this->compositor || !this->shm || !this->layer_shell) {
    std::cerr << "Missing some Required Globals." << std::endl;
    return;
  }

  printf("Recived all Required Globals.\n");

  while (1) {
    wl_display_dispatch(this->display);
  }
}

Engine::~Engine() { wl_display_disconnect(this->display); }
} // namespace WaypaperEngine
